//! Core app implementation module.

mod flow;
pub mod handler;

use std::{convert::Infallible, fmt::Debug, net::SocketAddr, sync::atomic::AtomicU64};

use async_std::sync::Arc;
use derive_builder::Builder;
use hyper::{service::{make_service_fn, service_fn},
            upgrade::Upgraded};
use tokio::net::TcpStream;
use tracing::{debug, error, info, warn};

pub use self::{flow::Flow,
               handler::{Forward, Handler, Reverse}};
use crate::{auth::{Authenticator, Credentials},
            http::{header, remove_hop_by_hop_headers, Method, Request, Response, StatusCode},
            metrics};

type Client = hyper::Client<hyper::client::HttpConnector>;

/// Main proxy application.
#[derive(Clone, Debug, Default, Builder)]
#[builder(default)]
pub struct Proxy {
    #[builder(default = r#""proxy""#)]
    id: &'static str,

    counter: Arc<AtomicU64>,
    client: Client,
    auths: Arc<Vec<Box<dyn Authenticator + Send + Sync>>>,
    handlers: Arc<Vec<Box<dyn Handler + Send + Sync>>>,
}

impl Proxy {
    pub fn new(
        id: &'static str,
        client: Client,
        auths: Vec<Box<dyn Authenticator + Send + Sync>>,
        handlers: Vec<Box<dyn Handler + Send + Sync>>,
    ) -> Self {
        Self {
            id,
            counter: Arc::new(AtomicU64::new(0)),
            client,
            auths: Arc::new(auths),
            handlers: Arc::new(handlers),
        }
    }

    pub fn builder() -> ProxyBuilder {
        ProxyBuilder::default()
    }

    pub async fn run(&self, addr: &SocketAddr) -> Result<(), hyper::Error> {
        hyper::Server::bind(addr)
            .http1_title_case_headers(true)
            .http1_preserve_header_case(true)
            .serve(make_service_fn(
                move |socket: &hyper::server::conn::AddrStream| {
                    let flow = self.flow(socket.remote_addr());
                    async move {
                        Ok::<_, Infallible>(service_fn(move |req| {
                            serve(flow.clone(), req)
                        }))
                    }
                },
            ))
            .with_graceful_shutdown(self.shutdown_signal())
            .await
    }

    async fn shutdown_signal(&self) {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install CTRL+C signal handler");
    }

    pub(crate) fn flow(&self, client: SocketAddr) -> Flow {
        Flow::new(self.clone(), client)
    }
}

#[tracing::instrument(skip_all, fields(app = flow.app().id, flow = flow.id()))]
async fn serve(
    flow: Flow,
    req: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, hyper::Error> {
    metrics::HTTP_REQ_COUNTER.increment(1);

    // Measure request duration
    let start = std::time::Instant::now();

    let (version, method, uri) = (
        req.version(),
        req.method().to_owned(),
        req.uri().to_string(),
    );
    let uri = uri.as_str();
    info!(
        client = flow.client().to_string(),
        version = format!("{version:?}"),
        method = method.to_string(),
        uri = uri
    );

    // Simple route implementation
    let result = match (method, uri) {
        // CONNECT *
        // TODO: Only tunneling for now, WebSocket and HTTPS interception currently not supported
        (Method::CONNECT, _) => connect(req).await,

        // Fallback; delegate to proxy
        (_, _) => proxy(flow, req).await,
    };

    metrics::HTTP_REQ_HISTOGRAM.record(start.elapsed().as_secs_f64());

    result
}

// BUG: CONNECT tunnel does not enforce proxy authorization for now (handler called at `proxy()` only)
// TODO: Merge into `proxy()` when implementing HTTPS interception
async fn connect(
    req: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, hyper::Error> {
    let uri = req.uri();
    let authority = uri.authority().map(|auth| auth.to_string());
    if let Some(addr) = authority {
        tokio::task::spawn(async move {
            match hyper::upgrade::on(req).await {
                Ok(upgraded) => {
                    if let Err(e) = tunnel(upgraded, addr).await {
                        error!("server io error: {e}");
                    };
                }
                Err(e) => error!("upgrade error: {e}"),
            }
        });

        Ok(hyper::Response::new(hyper::Body::empty()))
    } else {
        warn!("CONNECT host must be socket addr, but got: {:?}", uri);
        let resp = hyper::Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body("CONNECT must be to a socket address.".into())
            .unwrap();

        Ok(resp)
    }
}

async fn tunnel(mut upgraded: Upgraded, addr: String) -> Result<(), std::io::Error> {
    let mut server = TcpStream::connect(addr).await?;
    let (from_client, from_server) =
        tokio::io::copy_bidirectional(&mut upgraded, &mut server).await?;

    debug!(
        "client wrote {from_client} bytes and received {from_server} bytes from server via tunnel"
    );

    Ok(())
}

async fn proxy(
    mut flow: Flow,
    req: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, hyper::Error> {
    // Check URI host part exists
    // NOTE: proxy requests are expected to have full URIs (https://httpbin.org/get), while ordinary HTTP requests have
    //       just path part (/get)
    req.uri().host().expect("URI has no host part");

    // Convert request into crate-specific one
    let mut req = Request::from(req).await;

    // Authenticate and authorize proxy user.
    if !flow.app().auths.is_empty() {
        match Credentials::try_from(&req) {
            Ok(credentials) => {
                for ab in flow.app().auths.iter() {
                    match ab.authenticate(&credentials).await {
                        Ok(_) => {
                            *flow.auth_mut() = Some(credentials);

                            break;
                        }
                        Err(err) => {
                            debug!("authentication failed: {err}");
                        }
                    }
                }
            }
            Err(_) => {
                return Ok(hyper::Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(hyper::Body::from("invalid proxy auth credentials"))
                    .unwrap())
            }
        }

        // Respond with 407 if no auth passed
        if flow.auth().is_none() {
            let builder = hyper::Response::builder()
                .status(StatusCode::PROXY_AUTHENTICATION_REQUIRED)
                .header(header::PROXY_AUTHENTICATE, "Bearer");

            // TODO: Loop over all authenticate backends available, and add headers for them

            let resp = builder.body(hyper::Body::empty()).unwrap();

            return Ok(resp);
        }
    }

    // Call handlers on request
    for h in flow.app().handlers.iter() {
        // TODO: Panic handling for handlers for isolation & debugging
        match h.on_request(&flow, req.clone()).await {
            Forward::DoNothing => {}
            Forward::Modify(modified) => {
                req = *modified;
            }
            Forward::Reply(resp) => return Ok((*resp).into()),
        }
    }
    remove_hop_by_hop_headers(&mut req.headers);

    // Forward request to server
    let resp = flow.app().client.request(req.clone().into()).await?;
    let mut resp = Response::from(resp, req).await;

    // Call handlers on response
    for h in flow.app().handlers.iter() {
        match h.on_response(&flow, resp.clone()).await {
            Reverse::DoNothing => {}
            Reverse::Modify(modified) => {
                resp = *modified;
            }
            Reverse::Replace(resp) => return Ok((*resp).into()),
        }
    }

    // Response back to client
    Ok(resp.into())
}

#[cfg(test)]
mod tests {
    use std::{net::SocketAddr, str::FromStr};

    use anyhow::Result;
    use httpmock::prelude::*;
    use hyper::{body::to_bytes, Body, Method, Request, StatusCode, Uri};

    #[tokio::test]
    async fn connect() -> Result<()> {
        // NOTE: It just tests the establishment of tunnel
        let server = MockServer::start();
        let uri = Uri::from_str(&server.address().to_string())?;
        let req = Request::builder()
            .method(Method::CONNECT)
            .uri(uri)
            .body(Body::empty())?;

        let resp = super::connect(req).await?;

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(to_bytes(resp.into_body()).await?.to_vec(), b"");

        Ok(())
    }

    #[tokio::test]
    async fn tunnel() -> Result<()> {
        // Skip this for now as it is tricky to test and connect handler may cover this

        Ok(())
    }

    #[tokio::test]
    async fn proxy() -> Result<()> {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method("GET").path("/hello-world");
            then.status(200).body(b"Good Evening");
        });
        let uri = Uri::from_str(&server.url("/hello-world"))?;
        let req = Request::builder()
            .method(Method::GET)
            .uri(uri)
            .body(hyper::Body::empty())?;

        let proxy = super::Proxy::default();
        let flow = proxy.flow(SocketAddr::from_str("127.0.0.1:65535")?);
        let resp = super::proxy(flow, req).await?;

        mock.assert();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            String::from_utf8(to_bytes(resp.into_body()).await?.to_vec())?,
            "Good Evening"
        );

        Ok(())
    }
}
