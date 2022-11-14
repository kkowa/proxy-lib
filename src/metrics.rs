use lazy_static::lazy_static;
use prometheus::{labels, opts, register_histogram_vec, register_int_counter, HistogramVec,
                 IntCounter};

// Prometheus metrics; check args in `opts!` for detail
lazy_static! {
    pub static ref HTTP_REQ_COUNTER: IntCounter = register_int_counter!(opts!(
        "http_requests_total",
        "Number of HTTP requests made.",
        labels! {"handler" => "all"}
    ))
    .unwrap();
    pub static ref HTTP_REQ_HISTOGRAM: HistogramVec = register_histogram_vec!(
        "http_request_duration_seconds",
        "The HTTP request latencies in seconds.",
        &["handler"]
    )
    .unwrap();
}
