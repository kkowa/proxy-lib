mod credentials;

use std::fmt::Debug;

use async_trait::async_trait;
use thiserror::Error;
use tracing::{debug, trace};

pub use self::credentials::Credentials;

#[derive(Debug, Error)]
pub enum Error {
    #[error("unexpected scheme {got}, while expecting {expect}")]
    InvalidScheme { got: String, expect: String },

    #[error("provided auth credentials data is wrong format")]
    InvalidFormat { n: usize },

    #[error("authentication failed")]
    NotAuthenticated,
}

#[async_trait]
pub trait Authenticator: Debug {
    async fn authenticate(&self, credentials: &Credentials) -> Result<(), Error>;
}

/// Simple static HTTP basic authenticator.
#[derive(Debug)]
pub struct HTTPBasic {
    username: String,
    password: String,
}

impl HTTPBasic {
    pub fn new<S>(username: S, password: S) -> Self
    where
        S: AsRef<str>,
    {
        Self {
            username: username.as_ref().to_string(),
            password: password.as_ref().to_string(),
        }
    }
}

#[async_trait]
impl Authenticator for HTTPBasic {
    async fn authenticate(&self, credentials: &Credentials) -> Result<(), Error> {
        if credentials.scheme().to_lowercase() != "basic" {
            trace!(
                "scheme expected \"basic\" but got \"{got}\"",
                got = credentials.scheme()
            );
            return Err(Error::InvalidScheme {
                got: credentials.scheme().to_string(),
                expect: "basic".to_string(),
            });
        }

        // Base64 Decode credential field as this scheme expects base64 encoded credentials
        // in format of "<username>:<password>"
        let decoded = base64::decode(credentials.credentials())
            .expect("failed to base64 decode HTTP basic credentials");
        let decoded = String::from_utf8_lossy(&decoded);
        let v: Vec<&str> = decoded.split_terminator(':').collect();

        let n = v.len();
        if n != 2 {
            debug!(
                "credentials data contains {n} fields separate by colon, while expecting only 2"
            );
            return Err(Error::InvalidFormat { n });
        }

        let (username, password) = (v[0].to_string(), v[1].to_string());
        if username == self.username && password == self.password {
            return Ok(());
        }

        Err(Error::NotAuthenticated)
    }
}

/// Simple static HTTP bearer authenticator.
#[derive(Debug)]
pub struct HTTPBearer {
    token: String,
}

impl HTTPBearer {
    pub fn new<S>(token: S) -> Self
    where
        S: AsRef<str>,
    {
        Self {
            token: token.as_ref().to_string(),
        }
    }
}

#[async_trait]
impl Authenticator for HTTPBearer {
    async fn authenticate(&self, credentials: &Credentials) -> Result<(), Error> {
        if credentials.scheme().to_lowercase() != "bearer" {
            trace!(
                "scheme expected \"bearer\" but got \"{got}\"",
                got = credentials.scheme()
            );
            return Err(Error::InvalidScheme {
                got: credentials.scheme().to_string(),
                expect: "bearer".to_string(),
            });
        }

        if *credentials.credentials() == self.token {
            return Ok(());
        }

        Err(Error::NotAuthenticated)
    }
}

#[cfg(test)]
mod tests {
    use super::{Credentials, Error, HTTPBasic, HTTPBearer};
    use crate::auth::Authenticator;

    #[tokio::test]
    async fn httpbasic() {
        assert!(HTTPBasic::new("username", "password")
            .authenticate(&Credentials::new("Basic", "dXNlcm5hbWU6cGFzc3dvcmQ=")) // username:password
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn httpbasic_invalid_scheme() {
        assert!(matches!(
            HTTPBasic::new("username", "password")
                .authenticate(&Credentials::new("Base", "dXNlcm5hbWU6cGFzc3dvcmQ=")) // username:password
                .await,
            Err(Error::InvalidScheme { .. }) // FIXME: Can't try match using string types
        ));
    }

    #[tokio::test]
    async fn httpbasic_invalid_format() {
        assert!(matches!(
            HTTPBasic::new("username", "password")
                .authenticate(&Credentials::new("Basic", "b25lOnR3bzp0aHJlZQ==")) // one:two:three
                .await,
            Err(Error::InvalidFormat { n: 3 })
        ));
    }

    #[tokio::test]
    async fn httpbasic_unauthenticated() {
        assert!(matches!(
            HTTPBasic::new("username", "password")
                .authenticate(&Credentials::new("Basic", "cGFzc3dvcmQ6dXNlcm5hbWU=")) // password:username
                .await,
            Err(Error::NotAuthenticated)
        ));
    }

    #[tokio::test]
    async fn httpbearer() {
        assert!(HTTPBearer::new("token")
            .authenticate(&Credentials::new("Bearer", "token"))
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn httpbearer_invalid_scheme() {
        assert!(matches!(
            HTTPBearer::new("token")
                .authenticate(&Credentials::new("Token", "token"))
                .await,
            Err(Error::InvalidScheme { .. }) // FIXME: Can't try match using string types
        ));
    }

    #[tokio::test]
    async fn httpbearer_unauthenticated() {
        assert!(matches!(
            HTTPBearer::new("token")
                .authenticate(&Credentials::new("Bearer", "nekot"))
                .await,
            Err(Error::NotAuthenticated)
        ));
    }
}
