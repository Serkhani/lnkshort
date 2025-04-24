mod routes;

use std::error::Error;

use axum::{Router, routing::get};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::routes::health;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::registry()
    .with(
        tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "lnkshort=debug".into())
    )
    .with(tracing_subscriber::fmt::layer())
    .init();

    let app = Router::new()
    .route("/health", get(health));

    let listener = tokio::net::TcpListener::bind("[IP_ADDRESS]:3000").await.expect("Could not start server");

    tracing::debug!("Server running on: http://{}", listener.local_addr().expect("Could not get local address"));

    axum::serve(listener, app)
    .await
    .expect("Could not start server");
    
    Ok(())
}
