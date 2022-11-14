use std::{convert::Infallible, net::SocketAddr};

use hyper::{header,
            service::{make_service_fn, service_fn},
            Error, Method, StatusCode};
use prometheus::{Encoder, TextEncoder};
use tracing::info;

/// HTTP server instance for internal purpose, such as serving health checks, metrics, etc.
#[derive(Clone, Debug, Default)]
pub struct Web {}

impl Web {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn run(&self, addr: &SocketAddr) -> Result<(), Error> {
        let make_service = make_service_fn(move |_| async move {
            let service = service_fn(serve);

            Ok::<_, Infallible>(service)
        });

        hyper::Server::bind(addr)
            .serve(make_service)
            .with_graceful_shutdown(self.graceful_shutdown())
            .await
    }

    async fn graceful_shutdown(&self) {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install CTRL+C signal handler");

        // Do shutdown tasks here
    }
}

async fn serve(
    req: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, hyper::Error> {
    let (version, method, uri) = (
        req.version(),
        req.method().to_owned(),
        req.uri().to_string(),
    );
    let uri = uri.as_str();
    info!("{version:?} {method} {uri}");

    // Simple route implementation

    match (method, uri) {
        // GET /(ht|healthz)
        (Method::GET, "/ht" | "/healthz") => healthz().await,

        // GET /metrics
        (Method::GET, "/metrics") => metrics().await,

        // Fallback
        (_, _) => not_found().await,
    }
}

async fn healthz() -> Result<hyper::Response<hyper::Body>, hyper::Error> {
    Ok(hyper::Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/plain")
        .body("OK".into())
        .unwrap())
}

async fn metrics() -> Result<hyper::Response<hyper::Body>, hyper::Error> {
    let encoder = TextEncoder::new();
    let metrics_families = prometheus::gather();
    let mut buffer = vec![];
    encoder.encode(&metrics_families, &mut buffer).unwrap();

    let response = hyper::Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, encoder.format_type())
        .body(buffer.into())
        .unwrap();

    Ok(response)
}

async fn not_found() -> Result<hyper::Response<hyper::Body>, hyper::Error> {
    Ok(hyper::Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body("Not found".into())
        .unwrap())
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use hyper::{body::to_bytes, header, StatusCode};

    #[tokio::test]
    async fn healthz() -> Result<()> {
        let resp = super::healthz().await?;

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(to_bytes(resp.into_body()).await?.to_vec(), b"OK");

        Ok(())
    }

    #[tokio::test]
    async fn metrics() -> Result<()> {
        crate::metrics::HTTP_REQ_COUNTER.inc();
        let resp = super::metrics().await?;

        assert_eq!(resp.status(), StatusCode::OK);
        assert!(resp.headers().contains_key(header::CONTENT_TYPE));
        assert!(to_bytes(resp.into_body())
            .await?
            .to_vec()
            .starts_with(b"# HELP"));

        Ok(())
    }

    #[tokio::test]
    async fn not_found() -> Result<()> {
        let resp = super::not_found().await?;

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        assert_eq!(to_bytes(resp.into_body()).await?.to_vec(), b"Not found");

        Ok(())
    }
}
