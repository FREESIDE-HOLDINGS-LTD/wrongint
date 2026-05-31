use crate::app;
use crate::app::{GetIndexSeries, IndexCandles, IndexScope};
use crate::config::Config;
use crate::domain::time::{DateTime, Duration};
use crate::domain::{IndexCandle, SourceId};
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
    ),
    paths(handle_index_global, handle_index_source),
    components(schemas(ApiIndexCandles, ApiIndexCandle, ApiErrorBody))
)]
struct ApiDoc;

const DEFAULT_RANGE_DAYS: u64 = 7;

#[derive(Clone)]
pub struct AppState<H> {
    index_series: H,
    registry: Registry,
}

impl<H> AppState<H> {
    pub fn new(index_series: H, registry: Registry) -> Self {
        Self {
            index_series,
            registry,
        }
    }
}

pub struct Server<'a, H> {
    config: &'a Config,
    state: AppState<H>,
}

impl<'a, H> Server<'a, H>
where
    H: app::GetIndexSeriesHandler + Clone + Send + Sync + 'static,
{
    pub fn new(config: &'a Config, state: AppState<H>) -> Self {
        Self { config, state }
    }

    pub async fn run(&self) -> crate::errors::Result<()> {
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        let router = Router::new()
            .route("/api/index", get(handle_index_global::<H>))
            .route("/api/index/{source}", get(handle_index_source::<H>))
            .route("/metrics", get(handle_metrics::<H>))
            .route("/openapi.json", get(handle_openapi))
            .merge(Redoc::with_url("/docs", ApiDoc::openapi()))
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
        (status = 200, description = "Hourly OHLC candles for the global index", body = ApiIndexCandles),
        (status = 400, description = "Malformed `from`/`to` timestamp, or `from` after `to`", body = ApiErrorBody),
        (status = 500, description = "Unexpected server error", body = ApiErrorBody),
    )
)]
async fn handle_index_global<H>(
    State(state): State<AppState<H>>,
    Query(range): Query<IndexRange>,
) -> std::result::Result<Json<ApiIndexCandles>, ApiError>
where
    H: app::GetIndexSeriesHandler,
{
    index(&state, IndexScope::Global, &range).await
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
        (status = 200, description = "Hourly OHLC candles for one source", body = ApiIndexCandles),
        (status = 400, description = "Unknown source, malformed timestamp, or `from` after `to`", body = ApiErrorBody),
        (status = 500, description = "Unexpected server error", body = ApiErrorBody),
    )
)]
async fn handle_index_source<H>(
    State(state): State<AppState<H>>,
    Path(source): Path<String>,
    Query(range): Query<IndexRange>,
) -> std::result::Result<Json<ApiIndexCandles>, ApiError>
where
    H: app::GetIndexSeriesHandler,
{
    index(&state, IndexScope::Source(parse_source(&source)?), &range).await
}

async fn index<H>(
    state: &AppState<H>,
    scope: IndexScope,
    range: &IndexRange,
) -> std::result::Result<Json<ApiIndexCandles>, ApiError>
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
    Ok(Json(ApiIndexCandles::from(&candles)))
}

async fn handle_metrics<H>(
    State(state): State<AppState<H>>,
) -> std::result::Result<String, ApiError> {
    let encoder = TextEncoder::new();
    let families = state.registry.gather();
    Ok(encoder.encode_to_string(&families)?)
}

async fn handle_openapi() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
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
struct ApiIndexCandles {
    #[schema(example = "all")]
    source: String,
    candles: Vec<ApiIndexCandle>,
}

impl From<&IndexCandles> for ApiIndexCandles {
    fn from(s: &IndexCandles) -> Self {
        Self {
            source: s.scope().to_string(),
            candles: s
                .over_time()
                .candles()
                .iter()
                .map(ApiIndexCandle::from)
                .collect(),
        }
    }
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

enum ApiError {
    BadRequest(String),
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
