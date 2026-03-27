use axum::{
    Router,
    middleware::from_fn_with_state,
    routing::{get, post, put},
};
use axum_prometheus::PrometheusMetricLayer;
use metrics_exporter_prometheus::PrometheusHandle;
use tower_http::trace::TraceLayer;

use crate::api::health::health_handler;
use crate::api::links::{
    create_link_handler, get_stats_handler, redirect_handler, update_link_handler,
};
use crate::api::middleware::auth_middleware;
use crate::state::AppState;

pub mod health;
pub mod links;
pub mod middleware;

pub fn create_router(
    state: AppState,
    prom_layer: PrometheusMetricLayer<'static>,
    metric_handle: PrometheusHandle,
) -> Router {
    Router::new()
        .route("/create", post(create_link_handler))
        .route("/{:id}/stats", get(get_stats_handler))
        .route_layer(from_fn_with_state(state.clone(), auth_middleware))
        .route("/health", get(health_handler))
        .route(
            "/{:id}",
            put(update_link_handler)
                .route_layer(from_fn_with_state(state.clone(), auth_middleware))
                .get(redirect_handler),
        )
        .route("/metrics", get(move || async move { metric_handle.render() }))
        .layer(TraceLayer::new_for_http())
        .layer(prom_layer)
        .with_state(state)
}
