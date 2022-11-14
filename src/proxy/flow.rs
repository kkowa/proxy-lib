//! Context module for handlers.

use std::{net::SocketAddr,
          sync::{atomic::Ordering, Arc}};

use getset::{Getters, MutGetters};

use super::Proxy;
use crate::auth::Credentials;

/// Shared state of application context across handlers.
#[derive(Clone, Debug, Getters, MutGetters)]
pub struct Flow {
    /// Current flow's numeric sequence ID.
    #[getset(get = "pub")]
    id: u64,

    /// Parent app which current context have derive from.
    app: Arc<Proxy>,

    /// Incoming request source address.
    #[getset(get = "pub")]
    client: SocketAddr,

    /// Proxy authentication credentials. First passed auth credentials will be set if multiple auth backends set.
    #[getset(get = "pub", get_mut = "pub")]
    auth: Option<Credentials>,
}

impl Flow {
    /// Create new flow.
    pub fn new(proxy: Proxy, client: SocketAddr) -> Self {
        Self {
            id: proxy.counter.fetch_add(1, Ordering::SeqCst),
            app: Arc::new(proxy),
            client,
            auth: None,
        }
    }

    pub fn app(&self) -> Arc<Proxy> {
        Arc::clone(&self.app)
    }
}

#[cfg(test)]
mod tests {}
