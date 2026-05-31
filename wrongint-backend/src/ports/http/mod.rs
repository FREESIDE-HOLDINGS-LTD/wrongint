use crate::app;
use crate::app::{GetIndexSeries, IndexScope};
use crate::config::Config;
use crate::domain::time::{DateTime, Duration};
use crate::domain::{IndexCandle, Post, PostIndex, Snapshot, SourceId};
use crate::errors::Error;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use prometheus::{Registry, TextEncoder};
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use utoipa::{IntoParams, OpenApi, ToSchema};
use utoipa_redoc::{Redoc, Servable};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "wrongint",
        version = "0.1.0",
        description = "\
wrongint measures how argumentative programming communities are. It samples the \
front pages of Hacker News and lobste.rs, and for each snapshot computes an \
**index** = pooled comments ÷ pooled score. A higher index means more comments \
per upvote, i.e. more arguing.

Indexes are aggregated into **hourly OHLC candles** (open / high / low / close \
of the per-snapshot index within the hour), so the data reads like a stock \
ticker. Query the **global** index (both sources pooled) or a single source.

Timestamps are ISO-8601 / RFC-3339 in UTC. Ranges default to the last 7 days."
    ),
    tags(
        (name = "index", description = "Hourly OHLC index candles."),
        (name = "snapshot", description = "Captured front-page snapshots."),
    ),
    paths(handle_index_global, handle_index_source, handle_snapshot),
    components(schemas(ApiGlobalIndex, ApiSourceIndex, ApiIndexCandle, ApiSnapshot, ApiPost, ApiErrorBody))
)]
struct ApiDoc;

const DEFAULT_RANGE_DAYS: u64 = 7;

#[derive(Clone)]
pub struct AppState<H, S> {
    index_series: H,
    snapshots: S,
    registry: Registry,
}

impl<H, S> AppState<H, S> {
    pub fn new(index_series: H, snapshots: S, registry: Registry) -> Self {
        Self {
            index_series,
            snapshots,
            registry,
        }
    }
}

pub struct Server<'a, H, S> {
    config: &'a Config,
    state: AppState<H, S>,
}

impl<'a, H, S> Server<'a, H, S>
where
    H: app::GetIndexSeriesHandler + Clone + Send + Sync + 'static,
    S: app::GetSnapshotHandler + Clone + Send + Sync + 'static,
{
    pub fn new(config: &'a Config, state: AppState<H, S>) -> Self {
        Self { config, state }
    }

    pub async fn run(&self) -> crate::errors::Result<()> {
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        let router = Router::new()
            .route("/api/index", get(handle_index_global::<H, S>))
            .route("/api/index/{source}", get(handle_index_source::<H, S>))
            .route("/api/snapshot/{source}", get(handle_snapshot::<H, S>))
            .route("/metrics", get(handle_metrics::<H, S>))
            .route("/api/openapi.yml", get(handle_openapi_yaml))
            .merge(Redoc::with_url("/api", ApiDoc::openapi()))
            .layer(TraceLayer::new_for_http())
            .layer(cors)
            .with_state(self.state.clone());

        let listener = tokio::net::TcpListener::bind(self.config.http_address()).await?;
        axum::serve(listener, router).await?;
        Ok(())
    }
}

#[utoipa::path(
    get,
    path = "/api/index",
    tag = "index",
    operation_id = "get_global_index",
    summary = "Global index candles",
    description = "Hourly OHLC candles for the global index — every Hacker News and lobste.rs \
post in the window pooled together. Each candle's open/close are the first/last per-snapshot \
index in that hour; high/low are the extremes. Hours with no usable data (pooled score <= 0) \
are omitted. Candles are sorted oldest-first. Defaults to the last 7 days when from/to are absent.",
    params(IndexRange),
    responses(
        (status = 200, description = "Hourly OHLC candles for the global index", body = ApiGlobalIndex),
        (status = 400, description = "Malformed `from`/`to` timestamp, or `from` after `to`", body = ApiErrorBody),
        (status = 500, description = "Unexpected server error", body = ApiErrorBody),
    )
)]
async fn handle_index_global<H, S>(
    State(state): State<AppState<H, S>>,
    Query(range): Query<IndexRange>,
) -> std::result::Result<Json<ApiGlobalIndex>, ApiError>
where
    H: app::GetIndexSeriesHandler,
{
    let candles = index(&state, IndexScope::Global, &range).await?;
    Ok(Json(ApiGlobalIndex { candles }))
}

#[utoipa::path(
    get,
    path = "/api/index/{source}",
    tag = "index",
    operation_id = "get_source_index",
    summary = "Per-source index candles",
    description = "Same as the global endpoint but scoped to a single community. `source` must be \
`hackernews` or `lobsters`; anything else is a 400. Defaults to the last 7 days when from/to are absent.",
    params(
        ("source" = String, Path, description = "Community to query: `hackernews` or `lobsters`", example = "hackernews"),
        IndexRange,
    ),
    responses(
        (status = 200, description = "Hourly OHLC candles for one source", body = ApiSourceIndex),
        (status = 400, description = "Unknown source, malformed timestamp, or `from` after `to`", body = ApiErrorBody),
        (status = 500, description = "Unexpected server error", body = ApiErrorBody),
    )
)]
async fn handle_index_source<H, S>(
    State(state): State<AppState<H, S>>,
    Path(source): Path<String>,
    Query(range): Query<IndexRange>,
) -> std::result::Result<Json<ApiSourceIndex>, ApiError>
where
    H: app::GetIndexSeriesHandler,
{
    let id = parse_source(&source)?;
    let candles = index(&state, IndexScope::Source(id), &range).await?;
    Ok(Json(ApiSourceIndex {
        source: id.to_string(),
        candles,
    }))
}

#[utoipa::path(
    get,
    path = "/api/snapshot/{source}",
    tag = "snapshot",
    operation_id = "get_source_snapshot",
    summary = "Latest snapshot for a source",
    description = "The most recently captured front page for a source, with each post's own index \
(comments / score). `source` must be `hackernews` or `lobsters`. 404 if nothing captured yet.",
    params(
        ("source" = String, Path, description = "Community to query: `hackernews` or `lobsters`", example = "hackernews"),
    ),
    responses(
        (status = 200, description = "Latest captured snapshot", body = ApiSnapshot),
        (status = 400, description = "Unknown source", body = ApiErrorBody),
        (status = 404, description = "No snapshot captured yet", body = ApiErrorBody),
        (status = 500, description = "Unexpected server error", body = ApiErrorBody),
    )
)]
async fn handle_snapshot<H, S>(
    State(state): State<AppState<H, S>>,
    Path(source): Path<String>,
) -> std::result::Result<Json<ApiSnapshot>, ApiError>
where
    S: app::GetSnapshotHandler,
{
    let id = parse_source(&source)?;
    match state.snapshots.handle(id).await? {
        Some(snapshot) => Ok(Json(ApiSnapshot::from(&snapshot))),
        None => Err(ApiError::NotFound),
    }
}

async fn index<H, S>(
    state: &AppState<H, S>,
    scope: IndexScope,
    range: &IndexRange,
) -> std::result::Result<Vec<ApiIndexCandle>, ApiError>
where
    H: app::GetIndexSeriesHandler,
{
    let now = DateTime::now();
    let from = match &range.from {
        Some(s) => parse_ts(s)?,
        None => now - Duration::new_from_days(DEFAULT_RANGE_DAYS),
    };
    let to = match &range.to {
        Some(s) => parse_ts(s)?,
        None => now,
    };
    if from > to {
        return Err(ApiError::BadRequest("'from' must be <= 'to'".into()));
    }

    let candles = state
        .index_series
        .handle(&GetIndexSeries::new(scope, from, to))
        .await?;
    Ok(candles
        .over_time()
        .candles()
        .iter()
        .map(ApiIndexCandle::from)
        .collect())
}

async fn handle_metrics<H, S>(
    State(state): State<AppState<H, S>>,
) -> std::result::Result<String, ApiError> {
    let encoder = TextEncoder::new();
    let families = state.registry.gather();
    Ok(encoder.encode_to_string(&families)?)
}

async fn handle_openapi_yaml() -> std::result::Result<Response, ApiError> {
    let yaml = ApiDoc::openapi()
        .to_yaml()
        .map_err(|_| ApiError::Internal)?;
    Ok((
        [(axum::http::header::CONTENT_TYPE, "application/yaml")],
        yaml,
    )
        .into_response())
}

fn parse_source(s: &str) -> Result<SourceId, ApiError> {
    match s {
        "hackernews" => Ok(SourceId::HackerNews),
        "lobsters" => Ok(SourceId::Lobsters),
        _ => Err(ApiError::BadRequest(format!("unknown source '{s}'"))),
    }
}

fn parse_ts(s: &str) -> Result<DateTime, ApiError> {
    DateTime::new_from_rfc3339(s)
        .map_err(|_| ApiError::BadRequest(format!("bad timestamp '{s}' (want ISO-8601)")))
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct IndexRange {
    #[param(example = "2026-05-24T00:00:00Z")]
    from: Option<String>,
    #[param(example = "2026-05-31T00:00:00Z")]
    to: Option<String>,
}

#[derive(Serialize, ToSchema)]
struct ApiGlobalIndex {
    candles: Vec<ApiIndexCandle>,
}

#[derive(Serialize, ToSchema)]
struct ApiSourceIndex {
    #[schema(example = "hackernews")]
    source: String,
    candles: Vec<ApiIndexCandle>,
}

#[derive(Serialize, ToSchema)]
struct ApiIndexCandle {
    #[schema(example = "2026-05-31")]
    date: String,
    #[schema(example = 14)]
    hour: u32,
    open: Option<f64>,
    high: Option<f64>,
    low: Option<f64>,
    close: Option<f64>,
}

impl From<&IndexCandle> for ApiIndexCandle {
    fn from(c: &IndexCandle) -> Self {
        let ohlc = c.ohlc();
        Self {
            date: c.date().to_iso(),
            hour: c.hour(),
            open: ohlc.map(|o| o.open().value()),
            high: ohlc.map(|o| o.high().value()),
            low: ohlc.map(|o| o.low().value()),
            close: ohlc.map(|o| o.close().value()),
        }
    }
}

#[derive(Serialize, ToSchema)]
struct ApiSnapshot {
    #[schema(example = "hackernews")]
    source: String,
    captured_at: String,
    posts: Vec<ApiPost>,
}

impl From<&Snapshot> for ApiSnapshot {
    fn from(s: &Snapshot) -> Self {
        Self {
            source: s.source().to_string(),
            captured_at: s.captured_at().to_rfc3339(),
            posts: s.posts().iter().map(ApiPost::from).collect(),
        }
    }
}

#[derive(Serialize, ToSchema)]
struct ApiPost {
    id: String,
    title: String,
    comments_url: String,
    external_url: Option<String>,
    posted_at: String,
    comments: i64,
    score: i64,
    index: Option<f64>,
}

impl From<&Post> for ApiPost {
    fn from(p: &Post) -> Self {
        Self {
            id: p.post_id().as_str().to_string(),
            title: p.title().as_str().to_string(),
            comments_url: p.comments_url().as_str().to_string(),
            external_url: p.external_url().map(|u| u.as_str().to_string()),
            posted_at: p.posted_at().to_rfc3339(),
            comments: p.comments().value(),
            score: p.score().net(),
            index: PostIndex::from_post(p).map(|i| i.value()),
        }
    }
}

enum ApiError {
    BadRequest(String),
    NotFound,
    Internal,
}

impl From<Error> for ApiError {
    fn from(_err: Error) -> Self {
        ApiError::Internal
    }
}

impl From<prometheus::Error> for ApiError {
    fn from(_err: prometheus::Error) -> Self {
        ApiError::Internal
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::BadRequest(m) => (StatusCode::BAD_REQUEST, m),
            ApiError::NotFound => (StatusCode::NOT_FOUND, "not found".to_string()),
            ApiError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal server error".to_string(),
            ),
        };
        (status, Json(ApiErrorBody { error: message })).into_response()
    }
}

#[derive(Serialize, ToSchema)]
struct ApiErrorBody {
    #[schema(example = "unknown source 'bogus'")]
    error: String,
}
