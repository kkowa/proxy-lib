use std::net::SocketAddr;

use anyhow::Result;
use hyper::{body::to_bytes,
            client::{Client, HttpConnector},
            header, StatusCode};
use kkowa_proxy_lib::Web;
use portpicker::pick_unused_port;
use rstest::*;

type HTTPClient = Client<HttpConnector>;

/// Fixture for web service.
#[fixture]
fn web() -> String {
    let addr = SocketAddr::from((
        [127, 0, 0, 1],
        pick_unused_port().expect("no port available to run web server"),
    ));
    let url = format!("http://localhost:{port}", port = addr.port());

    // Run web server
    tokio::task::spawn(async move {
        Web::default().run(&addr).await.unwrap();
    });

    url
}

/// Fixture for HTTP client.
#[fixture]
fn client() -> HTTPClient {
    HTTPClient::default()
}

#[rstest]
#[tokio::test]
async fn healthz(web: String, client: HTTPClient) -> Result<()> {
    let resp = client.get(format!("{web}/healthz").parse()?).await?;

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(to_bytes(resp.into_body()).await?.to_vec(), b"OK");

    Ok(())
}

#[rstest]
#[tokio::test]
async fn metrics(web: String, client: HTTPClient) -> Result<()> {
    kkowa_proxy_lib::metrics::HTTP_REQ_COUNTER.inc();
    let resp = client
        .get(format!("{web}/metrics").parse().unwrap())
        .await?;

    assert_eq!(resp.status(), StatusCode::OK);
    assert!(resp.headers().contains_key(header::CONTENT_TYPE));
    assert!(to_bytes(resp.into_body())
        .await?
        .to_vec()
        .starts_with(b"# HELP"));

    Ok(())
}

#[rstest]
#[tokio::test]
async fn not_found(web: String, client: HTTPClient) -> Result<()> {
    let resp = client
        .get(format!("{web}/invalid-path").parse().unwrap())
        .await?;

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    assert_eq!(to_bytes(resp.into_body()).await?.to_vec(), b"Not found");

    Ok(())
}
