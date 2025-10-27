use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::IntoResponse,
};
use metrics::counter;
use sha3::{Digest, Sha3_256};
use sqlx::PgPool;

use crate::utils::internal_error;

struct Setting {
    #[allow(dead_code)]
    id: String,
    encrypted_global_api_key: String,
}

pub async fn auth(
    State(pool): State<PgPool>,
    req: Request,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let labels = [("uri", format!("{}!", req.uri()))];

    let api_key = req
        .headers()
        .get("x-api-key")
        .map(|val| val.to_str().unwrap_or_default())
        .ok_or_else(|| {
            tracing::error!("unauthorised api cal: No key received");
            counter!("unauthenticated_calls_count", &labels).increment(1);
            (StatusCode::UNAUTHORIZED, "Unauthorised".into())
        })?;

    let fetch_settings_timeout = tokio::time::Duration::from_millis(300);

    let setting = tokio::time::timeout(
        fetch_settings_timeout,
        sqlx::query_as!(
            Setting,
            "select id, encrypted_global_api_key from settings where id = $1",
            "DEFAULT_SETTINGS"
        )
        .fetch_one(&pool),
    )
    .await
    .map_err(internal_error)?
    .map_err(internal_error)?;

    let mut hasher = Sha3_256::new();
    hasher.update(api_key.as_bytes());
    let provided_api_key = hasher.finalize();

    // if setting.encrypted_global_api_key != format!("{provided_api_key:x}") {
    println!(
        "{} : {}",
        setting.encrypted_global_api_key,
        hex::encode(provided_api_key)
    );
    if setting.encrypted_global_api_key != hex::encode(provided_api_key) {
        tracing::error!("unauthenticated call to API: Incorrect key supplied");
        counter!("unauthenticated_calls_count", &labels).increment(1);
        return Err((StatusCode::UNAUTHORIZED, "Unauthorised".into()));
    }

    Ok(next.run(req).await)
}
