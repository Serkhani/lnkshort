mod routes;
mod utils;

use std::error::Error;

use axum::{
    Router,
    routing::{get, post, put},
};
use axum_prometheus::PrometheusMetricLayer;
use sqlx::postgres::PgPoolOptions;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::routes::{create_link, health, redirect, update_link};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "lnkshort=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let db = PgPoolOptions::new()
        .max_connections(12)
        .connect(&db_url)
        .await?;

    let (promlayer, metric_handler) = PrometheusMetricLayer::pair();

    let app = Router::new()
        .route("/health", get(health))
        .route("/{:id}", get(redirect))
        .route("/update/{:oldLinkId}", put(update_link))
        .route("/create", post(create_link))
        .route("/metrics", get(|| async move { metric_handler.render() }))
        .layer(TraceLayer::new_for_http())
        .layer(promlayer)
        .with_state(db);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Could not start server");

    tracing::debug!(
        "Server running on: http://{}",
        listener.local_addr().expect("Could not get local address")
    );

    axum::serve(listener, app)
        .await
        .expect("Could not start server");

    Ok(())
}
