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
use crate::api::middleware::{auth_middleware, create_rate_limit_layer};
use crate::state::AppState;

pub mod health;
pub mod links;
pub mod middleware;

pub fn create_router(
    state: AppState,
    prom_layer: PrometheusMetricLayer<'static>,
    metric_handle: PrometheusHandle,
) -> Router {
    let mut api_routes = Router::new()
        .route("/create", post(create_link_handler))
        .route("/{:id}/stats", get(get_stats_handler))
        .route_layer(from_fn_with_state(state.clone(), auth_middleware))
        .route(
            "/{:id}",
            put(update_link_handler)
                .route_layer(from_fn_with_state(state.clone(), auth_middleware))
                .get(redirect_handler),
        );

    if state.config.rate_limit.enabled {
        api_routes = api_routes.layer(create_rate_limit_layer(
            state.config.rate_limit.requests_per_second,
            state.config.rate_limit.burst_size,
        ));
    }

    Router::new()
        .merge(api_routes)
        .route("/health", get(health_handler))
        .route(
            "/metrics",
            get(move || async move { metric_handle.render() }),
        )
        .layer(TraceLayer::new_for_http())
        .layer(prom_layer)
        .with_state(state)
}
