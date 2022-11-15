use lazy_static::lazy_static;
use metrics::{register_counter, register_histogram, Counter, Histogram};

// Prometheus metrics; check args in `opts!` for detail
lazy_static! {
    pub static ref HTTP_REQ_COUNTER: Counter = register_counter!("http_requests_total");
    pub static ref HTTP_REQ_HISTOGRAM: Histogram =
        register_histogram!("http_request_duration_seconds");
}
