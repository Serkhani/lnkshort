use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;

use crate::error::AppError;
use crate::models::{Link, LinkStats, LinkTarget};
use crate::state::AppState;

pub async fn create_link_handler(
    State(state): State<AppState>,
    Json(payload): Json<LinkTarget>,
) -> Result<Json<Link>, AppError> {
    let link = state
        .link_service
        .create_link(&state.db, &payload.target_url)
        .await?;

    Ok(Json(link))
}

pub async fn redirect_handler(
    State(state): State<AppState>,
    Path(requested_link): Path<String>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let link = state
        .link_service
        .get_link(&state.db, &requested_link)
        .await?;

    tracing::debug!(
        "Redirecting link id {} to {}",
        requested_link,
        link.target_url
    );

    let referer = headers
        .get("referer")
        .and_then(|val| val.to_str().ok())
        .map(|s| s.to_string());
    let user_agent = headers
        .get("user-agent")
        .and_then(|val| val.to_str().ok())
        .map(|s| s.to_string());

    let stats_service = state.stats_service.clone();
    let pool = state.db.clone();
    let link_id = link.id.clone();

    // Record stats with timeout
    stats_service
        .record_stat(&pool, link_id, user_agent, referer)
        .await;

    Response::builder()
        .status(StatusCode::TEMPORARY_REDIRECT)
        .header("Location", link.target_url)
        .header("Cache-Control", &state.config.link.default_cache_control)
        .body(axum::body::Body::empty())
        .map_err(|e| AppError::Internal(format!("Failed to construct redirect response: {}", e)))
}

pub async fn update_link_handler(
    State(state): State<AppState>,
    Path(old_link_id): Path<String>,
    Json(payload): Json<LinkTarget>,
) -> Result<Json<Link>, AppError> {
    let updated_link = state
        .link_service
        .update_link(&state.db, &old_link_id, &payload.target_url)
        .await?;

    Ok(Json(updated_link))
}

pub async fn get_stats_handler(
    State(state): State<AppState>,
    Path(link_id): Path<String>,
) -> Result<Json<Vec<LinkStats>>, AppError> {
    let stats = state
        .stats_service
        .get_stats(&state.db, &link_id)
        .await?;

    Ok(Json(stats))
}
