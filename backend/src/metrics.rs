use axum::{Router, response::IntoResponse, routing::get};
use metrics::{counter, describe_counter, describe_histogram, histogram};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use std::time::Instant;

pub fn init_metrics() -> PrometheusHandle {
    let builder = PrometheusBuilder::new()
        .set_buckets_for_metric(
            Matcher::Full("http_request_duration_seconds".to_string()),
            &[
                0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ],
        )
        .unwrap();

    let handle = builder
        .install_recorder()
        .expect("failed to install prometheus recorder");

    describe_counter!(
        "fetch_requests_total",
        "Total number of fetch requests by status (success/error/blocked)"
    );
    describe_counter!(
        "rate_limit_exceeded_total",
        "Total number of rate limit exceeded events by scope (login/public)"
    );
    describe_histogram!(
        "http_request_duration_seconds",
        "HTTP request duration in seconds"
    );
    describe_histogram!(
        "fetch_duration_seconds",
        "Fetch operation duration in seconds"
    );

    handle
}

pub fn metrics_router(handle: PrometheusHandle) -> Router {
    Router::new().route(
        "/metrics",
        get(move || async move { handle.render().into_response() }),
    )
}

pub fn record_fetch_request(status: &'static str) {
    counter!("fetch_requests_total", "status" => status).increment(1);
}

pub fn record_rate_limit_exceeded(scope: &'static str) {
    counter!("rate_limit_exceeded_total", "scope" => scope).increment(1);
}

pub struct RequestTimer {
    start: Instant,
}

impl Default for RequestTimer {
    fn default() -> Self {
        Self::new()
    }
}

impl RequestTimer {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}

impl Drop for RequestTimer {
    fn drop(&mut self) {
        let duration = self.start.elapsed().as_secs_f64();
        histogram!("http_request_duration_seconds").record(duration);
    }
}

pub struct FetchTimer {
    start: Instant,
}

impl Default for FetchTimer {
    fn default() -> Self {
        Self::new()
    }
}

impl FetchTimer {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}

impl Drop for FetchTimer {
    fn drop(&mut self) {
        let duration = self.start.elapsed().as_secs_f64();
        histogram!("fetch_duration_seconds").record(duration);
    }
}
