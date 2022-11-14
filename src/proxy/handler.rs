//! Module for base handler constraint.
use std::fmt::Debug;

use async_trait::async_trait;

use super::Flow;
use crate::http::{Request, Response};

/// Enum for handler actions on forward direction (a request, from client to proxy).
pub enum Forward {
    /// For when handler made no changes on request.
    DoNothing,

    /// Make changes on request and pass to next handler.
    Modify(Box<Request>),

    /// Early return response without making requests to remote destination, skipping all remaining handlers.
    Reply(Box<Response>),
}

/// Enum for handler actions on reverse direction (a response, from proxy to client).
pub enum Reverse {
    /// For when handler made no changes on response.
    DoNothing,

    /// Make changes on response and pass to next handler.
    Modify(Box<Response>),

    /// Return given response to client, skipping all remaining handlers.
    Replace(Box<Response>),
}

/// Basic handler trait.
#[async_trait]
pub trait Handler: Debug + Sync {
    async fn on_request(&self, _flow: &Flow, _req: Request) -> Forward {
        Forward::DoNothing
    }

    async fn on_response(&self, _flow: &Flow, _resp: Response) -> Reverse {
        Reverse::DoNothing
    }
}

/// Simple handler that does nothing.
#[derive(Debug)]
pub struct Dummy;

#[async_trait]
impl Handler for Dummy {}

#[cfg(test)]
mod tests {
    use std::{net::SocketAddr, str::FromStr};

    use anyhow::Result;

    use super::{Dummy, Handler, Response};
    use crate::{proxy::{Forward, Reverse},
                Proxy};

    #[tokio::test]
    async fn dummy() -> Result<()> {
        let app = Proxy::default();
        let flow = app.flow(SocketAddr::from_str("127.0.0.1:65535")?);
        let response = Response::default();

        assert!(matches!(
            Dummy.on_request(&flow, response.request.clone()).await,
            Forward::DoNothing
        ));
        assert!(matches!(
            Dummy.on_response(&flow, response).await,
            Reverse::DoNothing
        ));

        Ok(())
    }
}
