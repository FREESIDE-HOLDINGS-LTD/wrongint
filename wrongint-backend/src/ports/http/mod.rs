use crate::app;
use crate::app::{GetIndexSeries, IndexSeries};
use crate::config::Config;
use crate::domain::SourceId;
use crate::domain::time::{DateTime, Duration};
use crate::errors::Error;
use axum::extract::{Query, State};
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
        description = "Measures how argumentative programming communities are.",
        version = "0.1.0"
    ),
    paths(handle_index),
    components(schemas(ApiIndexSeries, ApiIndexPoint, ApiErrorBody))
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
            .route("/api/index", get(handle_index::<H>))
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
    params(IndexParams),
    responses(
        (status = 200, description = "Index series for a source", body = ApiIndexSeries),
        (status = 400, description = "Invalid query parameters", body = ApiErrorBody),
        (status = 500, description = "Internal server error", body = ApiErrorBody),
    )
)]
async fn handle_index<H>(
    State(state): State<AppState<H>>,
    Query(params): Query<IndexParams>,
) -> std::result::Result<Json<ApiIndexSeries>, ApiError>
where
    H: app::GetIndexSeriesHandler,
{
    let source = parse_source(&params.source)?;

    let now = DateTime::now();
    let from = match &params.from {
        Some(s) => parse_ts(s)?,
        None => now - Duration::new_from_days(DEFAULT_RANGE_DAYS),
    };
    let to = match &params.to {
        Some(s) => parse_ts(s)?,
        None => now,
    };
    if from > to {
        return Err(ApiError::BadRequest("'from' must be <= 'to'".into()));
    }

    let series = state
        .index_series
        .handle(&GetIndexSeries::new(source, from, to))
        .await?;
    Ok(Json(ApiIndexSeries::from(&series)))
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
struct IndexParams {
    /// Source identifier: `hackernews` or `lobsters`.
    source: String,
    /// Inclusive lower bound (ISO-8601). Defaults to 7 days ago.
    from: Option<String>,
    /// Inclusive upper bound (ISO-8601). Defaults to now.
    to: Option<String>,
}

#[derive(Serialize, ToSchema)]
struct ApiIndexSeries {
    source: String,
    points: Vec<ApiIndexPoint>,
}

impl From<&IndexSeries> for ApiIndexSeries {
    fn from(s: &IndexSeries) -> Self {
        Self {
            source: s.source().to_string(),
            points: s.points().iter().map(ApiIndexPoint::from).collect(),
        }
    }
}

#[derive(Serialize, ToSchema)]
struct ApiIndexPoint {
    captured_at: String,
    index: Option<f64>,
}

impl From<&app::IndexPoint> for ApiIndexPoint {
    fn from(p: &app::IndexPoint) -> Self {
        Self {
            captured_at: p.captured_at().to_rfc3339(),
            index: p.index().map(|i| i.value()),
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
    error: String,
}
