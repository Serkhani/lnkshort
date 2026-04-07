use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

pub use crate::api::health::health_handler as health;
pub use crate::models::{Link, LinkStats, LinkTarget};
use crate::repository::{LinkRepository, StatsRepository};
use crate::services::{LinkService, StatsService};

fn default_link_config() -> Arc<crate::config::LinkConfig> {
    Arc::new(crate::config::LinkConfig {
        max_retries: 3,
        default_cache_control:
            "public, max-age=300, s-maxage=300, state-while-revalidate=300, stale-if-error=300"
                .to_string(),
        query_timeout: std::time::Duration::from_millis(300),
    })
}

pub async fn redirect(
    State(db): State<DatabaseConnection>,
    Path(requested_link): Path<String>,
    headers: HeaderMap,
) -> Result<Response, (StatusCode, String)> {
    let config = default_link_config();
    let link_service = LinkService::new(LinkRepository::new(), config.clone());
    let stats_service = StatsService::new(StatsRepository::new(), config.clone());

    let link = link_service
        .get_link(&db, &requested_link)
        .await
        .map_err(|e| match e {
            crate::error::AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            other => (StatusCode::INTERNAL_SERVER_ERROR, other.to_string()),
        })?;

    let referer = headers
        .get("referer")
        .and_then(|val| val.to_str().ok())
        .map(|s| s.to_string());
    let user_agent = headers
        .get("user-agent")
        .and_then(|val| val.to_str().ok())
        .map(|s| s.to_string());

    stats_service
        .record_stat(&db, link.id.clone(), user_agent, referer)
        .await;

    Response::builder()
        .status(StatusCode::TEMPORARY_REDIRECT)
        .header("Location", link.target_url)
        .header("Cache-Control", &config.default_cache_control)
        .body(axum::body::Body::empty())
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))
}

pub async fn create_link(
    State(db): State<DatabaseConnection>,
    Json(new_link): Json<LinkTarget>,
) -> Result<Json<Link>, (StatusCode, String)> {
    let config = default_link_config();
    let link_service = LinkService::new(LinkRepository::new(), config);

    let link = link_service
        .create_link(&db, &new_link.target_url)
        .await
        .map_err(|e| match e {
            crate::error::AppError::InvalidUrl(msg) => (StatusCode::CONFLICT, msg),
            crate::error::AppError::ExhaustedRetries => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
            other => (StatusCode::INTERNAL_SERVER_ERROR, other.to_string()),
        })?;

    Ok(Json(link))
}

pub async fn update_link(
    State(db): State<DatabaseConnection>,
    Path(old_link_id): Path<String>,
    Json(new_link_target): Json<LinkTarget>,
) -> Result<Json<Link>, (StatusCode, String)> {
    let config = default_link_config();
    let link_service = LinkService::new(LinkRepository::new(), config);

    let link = link_service
        .update_link(&db, &old_link_id, &new_link_target.target_url)
        .await
        .map_err(|e| match e {
            crate::error::AppError::InvalidUrl(msg) => (StatusCode::CONFLICT, msg),
            crate::error::AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            other => (StatusCode::INTERNAL_SERVER_ERROR, other.to_string()),
        })?;

    Ok(Json(link))
}

pub async fn getstats(
    State(db): State<DatabaseConnection>,
    Path(link_id): Path<String>,
) -> Result<Json<Vec<LinkStats>>, (StatusCode, String)> {
    let config = default_link_config();
    let stats_service = StatsService::new(StatsRepository::new(), config);

    let stats = stats_service
        .get_stats(&db, &link_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(stats))
}
