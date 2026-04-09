use axum::body::Body;
use axum::http::StatusCode;
use axum::response::Response;
use governor::clock::QuantaInstant;
use governor::middleware::NoOpMiddleware;
use std::sync::Arc;
use tower_governor::governor::{GovernorConfig, GovernorConfigBuilder};
use tower_governor::key_extractor::SmartIpKeyExtractor;
use tower_governor::{GovernorError, GovernorLayer};

pub type AppGovernorConfig = GovernorConfig<SmartIpKeyExtractor, NoOpMiddleware<QuantaInstant>>;

pub fn create_rate_limit_layer(
    requests_per_second: u64,
    burst_size: u32,
) -> GovernorLayer<SmartIpKeyExtractor, NoOpMiddleware<QuantaInstant>, Body> {
    let config = GovernorConfigBuilder::default()
        .per_second(requests_per_second)
        .burst_size(burst_size)
        .key_extractor(SmartIpKeyExtractor)
        .finish()
        .expect("Failed to initialize GovernorConfig for rate limiting");

    GovernorLayer::new(Arc::new(config)).error_handler(|err: GovernorError| {
        metrics::counter!("rate_limited_requests_count").increment(1);
        tracing::warn!("Rate limit exceeded: {}", err);
        let mut resp = Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"error":"Too Many Requests"}"#))
            .unwrap();

        if let GovernorError::TooManyRequests { wait_time, .. } = &err {
            let retry_after = wait_time.max(&1).to_string();
            if let Ok(header_val) = retry_after.parse() {
                resp.headers_mut().insert("Retry-After", header_val);
            }
        }

        resp
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::http::Request;
    use axum::routing::get;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_rate_limiting_enforcement() {
        let app = Router::new()
            .route("/test", get(|| async { "ok" }))
            .layer(create_rate_limit_layer(1, 1));

        let make_request = || {
            let mut req = Request::builder().uri("/test").body(Body::empty()).unwrap();
            req.headers_mut()
                .insert("x-forwarded-for", "192.168.1.100".parse().unwrap());
            req
        };

        let res1 = app.clone().oneshot(make_request()).await.unwrap();
        assert_eq!(res1.status(), StatusCode::OK);

        let res2 = app.clone().oneshot(make_request()).await.unwrap();
        assert_eq!(res2.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(res2.headers().contains_key("retry-after"));
    }

    #[tokio::test]
    async fn test_different_ips_separate_buckets() {
        let app = Router::new()
            .route("/test", get(|| async { "ok" }))
            .layer(create_rate_limit_layer(1, 1));

        let req1 = Request::builder()
            .uri("/test")
            .header("x-forwarded-for", "10.0.0.1")
            .body(Body::empty())
            .unwrap();

        let req2 = Request::builder()
            .uri("/test")
            .header("x-forwarded-for", "10.0.0.2")
            .body(Body::empty())
            .unwrap();

        let res1 = app.clone().oneshot(req1).await.unwrap();
        assert_eq!(res1.status(), StatusCode::OK);

        let res2 = app.clone().oneshot(req2).await.unwrap();
        assert_eq!(res2.status(), StatusCode::OK);
    }
}
