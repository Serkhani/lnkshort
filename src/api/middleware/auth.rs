use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use metrics::counter;

use crate::error::AppError;
use crate::state::AppState;

pub async fn auth_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let uri_str = req.uri().to_string();
    let labels = [("uri", format!("{}!", uri_str))];

    let api_key = req
        .headers()
        .get("x-api-key")
        .and_then(|val| val.to_str().ok())
        .ok_or_else(|| {
            tracing::error!("unauthorised api cal: No key received");
            counter!("unauthenticated_calls_count", &labels).increment(1);
            AppError::Unauthorized("Unauthorised".to_string())
        })?;

    state
        .auth_service
        .verify_api_key(&state.db, api_key, &uri_str)
        .await?;

    Ok(next.run(req).await)
}
