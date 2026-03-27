use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::IntoResponse,
};
use sqlx::PgPool;

pub async fn auth(
    State(pool): State<PgPool>,
    req: Request,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let uri = req.uri().to_string();
    let api_key = req
        .headers()
        .get("x-api-key")
        .and_then(|val| val.to_str().ok())
        .ok_or_else(|| {
            let labels = [("uri", format!("{}!", uri))];
            metrics::counter!("unauthenticated_calls_count", &labels).increment(1);
            (StatusCode::UNAUTHORIZED, "Unauthorised".to_string())
        })?;

    let repo = crate::repository::SettingsRepository::new();
    let config = std::sync::Arc::new(crate::config::LinkConfig {
        max_retries: 3,
        default_cache_control: String::new(),
        query_timeout: std::time::Duration::from_millis(300),
    });
    let auth_service = crate::services::AuthService::new(repo, config);

    auth_service
        .verify_api_key(&pool, api_key, &uri)
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Unauthorised".to_string()))?;

    Ok(next.run(req).await)
}
