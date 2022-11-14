use std::net::SocketAddr;

use anyhow::{Error, Result};
use httpmock::prelude::*;
use hyper::client::{Client, HttpConnector};
use hyper_proxy::{Intercept, ProxyConnector};
use kkowa_proxy_lib::{http::StatusCode, Proxy};
use portpicker::pick_unused_port;
use rstest::*;

type ProxyClient = Client<ProxyConnector<HttpConnector>>;

/// Fixture for mocking HTTP server.
#[fixture]
fn server() -> MockServer {
    MockServer::start()
}

/// Fixture for proxy service.
#[fixture]
fn proxy() -> String {
    let addr = SocketAddr::from((
        [127, 0, 0, 1],
        pick_unused_port().expect("no port available"),
    ));
    // FIXME: 127.0.0.1 fails with ConnectionRefused error; why?
    let url = format!(
        "http://{host}:{port}",
        host = "localhost",
        port = addr.port()
    );

    // Run server
    tokio::task::spawn(async move {
        let proxy = Proxy::builder().build().unwrap();
        proxy.run(&addr).await.unwrap()
    });

    url
}

/// Fixture for HTTP client with proxy configuration set.
#[fixture]
fn client(proxy: String) -> ProxyClient {
    let proxy = hyper_proxy::Proxy::new(Intercept::All, proxy.parse().unwrap());
    let http_connector = HttpConnector::new();
    let proxy_connector = ProxyConnector::from_proxy(http_connector, proxy).unwrap();

    Client::builder().build(proxy_connector)
}

/// Test proxy on ordinary HTTP connections.
#[rstest]
#[tokio::test]
async fn http_proxy(client: ProxyClient, server: MockServer) -> Result<(), Error> {
    server.mock(|when, then| {
        when.method(GET).path("/get");
        then.status(200).body("Hello World!");
    });
    let res = client.get(server.url("/get").parse()?).await?;

    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        hyper::body::to_bytes(res.into_body()).await?,
        "Hello World!"
    );

    Ok(())
}

/// Test proxy tunneling on HTTPS connections.
#[rstest]
#[tokio::test]
async fn https_tunnel(client: ProxyClient) -> Result<(), Error> {
    let res = client.get("https://httpbin.org/get".parse()?).await?;

    assert_eq!(res.status(), StatusCode::OK);

    Ok(())
}
