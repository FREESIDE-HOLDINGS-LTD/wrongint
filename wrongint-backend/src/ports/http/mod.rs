use crate::app::{Point, QueryService, Resolution, Series, SourceSel, SourceStatus};
use crate::config::Config;
use crate::domain::{SourceId, Ts};
use crate::errors::Error;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use chrono::Duration as ChronoDuration;
use prometheus::{Registry, TextEncoder};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

const DEFAULT_RANGE_DAYS: i64 = 7;

#[derive(Clone)]
pub struct AppState {
    query: Arc<QueryService>,
    registry: Registry,
}

impl AppState {
    pub fn new(query: Arc<QueryService>, registry: Registry) -> Self {
        Self { query, registry }
    }
}

pub struct Server<'a> {
    config: &'a Config,
    state: AppState,
}

impl<'a> Server<'a> {
    pub fn new(config: &'a Config, state: AppState) -> Self {
        Self { config, state }
    }

    pub async fn run(&self) -> crate::errors::Result<()> {
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        let router = Router::new()
            .route("/api/sources", get(handle_sources))
            .route("/api/scores", get(handle_scores))
            .route("/api/global", get(handle_global))
            .route("/metrics", get(handle_metrics))
            .layer(TraceLayer::new_for_http())
            .layer(cors)
            .with_state(self.state.clone());

        let listener = tokio::net::TcpListener::bind(self.config.http_address()).await?;
        axum::serve(listener, router).await?;
        Ok(())
    }
}

async fn handle_sources(
    State(state): State<AppState>,
) -> std::result::Result<Json<Vec<ApiSource>>, ApiError> {
    let overview = state.query.sources_overview()?;
    Ok(Json(overview.iter().map(ApiSource::from).collect()))
}

async fn handle_scores(
    State(state): State<AppState>,
    Query(params): Query<ScoreParams>,
) -> std::result::Result<Json<ApiSeries>, ApiError> {
    let source = params
        .source
        .as_deref()
        .ok_or_else(|| ApiError::BadRequest("missing required 'source' param".into()))?;
    let sel = parse_source_sel(source)?;
    let series = run_query(&state, sel, &params)?;
    Ok(Json(ApiSeries::from(&series)))
}

async fn handle_global(
    State(state): State<AppState>,
    Query(params): Query<ScoreParams>,
) -> std::result::Result<Json<ApiSeries>, ApiError> {
    let series = run_query(&state, SourceSel::Global, &params)?;
    Ok(Json(ApiSeries::from(&series)))
}

async fn handle_metrics(State(state): State<AppState>) -> std::result::Result<String, ApiError> {
    let encoder = TextEncoder::new();
    let families = state.registry.gather();
    Ok(encoder.encode_to_string(&families)?)
}

fn run_query(state: &AppState, sel: SourceSel, params: &ScoreParams) -> Result<Series, ApiError> {
    let now = chrono::Utc::now();
    let from = match &params.from {
        Some(s) => parse_ts(s)?,
        None => now - ChronoDuration::days(DEFAULT_RANGE_DAYS),
    };
    let to = match &params.to {
        Some(s) => parse_ts(s)?,
        None => now,
    };
    if from > to {
        return Err(ApiError::BadRequest("'from' must be <= 'to'".into()));
    }
    let res = match params.resolution.as_deref() {
        Some(s) => Resolution::parse(s)
            .ok_or_else(|| ApiError::BadRequest(format!("unknown resolution '{s}'")))?,
        None => Resolution::Hour,
    };
    Ok(state.query.scores(sel, from, to, res)?)
}

fn parse_source_sel(s: &str) -> Result<SourceSel, ApiError> {
    if s == "global" {
        return Ok(SourceSel::Global);
    }
    SourceId::parse(s)
        .map(SourceSel::One)
        .ok_or_else(|| ApiError::BadRequest(format!("unknown source '{s}'")))
}

fn parse_ts(s: &str) -> Result<Ts, ApiError> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .map_err(|_| ApiError::BadRequest(format!("bad timestamp '{s}' (want ISO-8601)")))
}

#[derive(Deserialize)]
struct ScoreParams {
    source: Option<String>,
    from: Option<String>,
    to: Option<String>,
    resolution: Option<String>,
}

#[derive(Serialize)]
struct ApiSource {
    id: String,
    name: String,
    current_score: Option<f64>,
    last_sample: Option<Ts>,
}

impl From<&SourceStatus> for ApiSource {
    fn from(s: &SourceStatus) -> Self {
        Self {
            id: s.id.as_str().to_string(),
            name: s.name.clone(),
            current_score: s.current_score,
            last_sample: s.last_sample,
        }
    }
}

#[derive(Serialize)]
struct ApiSeries {
    source: String,
    resolution: String,
    points: Vec<ApiPoint>,
}

impl From<&Series> for ApiSeries {
    fn from(s: &Series) -> Self {
        Self {
            source: s.source.clone(),
            resolution: s.resolution.as_str().to_string(),
            points: s.points.iter().map(ApiPoint::from).collect(),
        }
    }
}

#[derive(Serialize)]
struct ApiPoint {
    t: Ts,
    score: Option<f64>,
    comments: i64,
    upvotes: i64,
}

impl From<&Point> for ApiPoint {
    fn from(p: &Point) -> Self {
        Self {
            t: p.t,
            score: p.score,
            comments: p.comments,
            upvotes: p.upvotes,
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

#[derive(Serialize)]
struct ApiErrorBody {
    error: String,
}
