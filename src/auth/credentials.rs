use std::fmt::Debug;

use getset::Getters;
use thiserror::Error;
use tracing::{debug, trace};

use crate::http::{header, Request};

#[derive(Debug, Error)]
pub enum Error {
    #[error("required authorization header does not exists in request")]
    MissingHeader,

    #[error("failed to parse provided data into desired format")]
    InvalidFormat { n: usize },

    #[error("unknown error")]
    Unknown,
}

// TODO: Implement its own Debug / Fmt traits to mask credentials for security
#[derive(Clone, Debug, PartialEq, Eq, Getters)]
pub struct Credentials {
    #[getset(get = "pub")]
    scheme: String,

    #[getset(get = "pub")]
    credentials: String,
}

impl Credentials {
    pub fn new<S>(scheme: S, credentials: S) -> Self
    where
        S: AsRef<str>,
    {
        Self {
            scheme: scheme.as_ref().to_string(),
            credentials: credentials.as_ref().to_string(),
        }
    }
}

impl TryFrom<&Request> for Credentials {
    type Error = Error;

    /// Extract credentials from proxy authorization header in request.
    fn try_from(request: &Request) -> Result<Self, Self::Error> {
        match request.headers.get(header::PROXY_AUTHORIZATION) {
            Some(value) => {
                // Split scheme and credentials fields
                // NOTE: No base64 handling here (yet?)
                let arr: [&str; 2] = value
                    .to_str()
                    .expect("failed to convert header value to string")
                    .split_whitespace()
                    .collect::<Vec<&str>>()
                    .try_into()
                    .map_err(|v: Vec<&str>| Error::InvalidFormat { n: v.len() })?;

                let (scheme, credentials) = (arr[0], arr[1]);
                trace!(
                    "parsed credentials with scheme {scheme} and {n}-length credentials data",
                    n = credentials.len()
                );

                Ok(Credentials::new(scheme, credentials))
            }
            None => {
                {
                    let keys = request
                        .headers
                        .keys()
                        .map(|k| k.as_str())
                        .collect::<Vec<&str>>()
                        .join(", ");

                    debug!(
                        "required header \"Proxy-Authorization\" does not exists, there: {keys}",
                    );
                };

                Err(Error::MissingHeader)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::Credentials;
    use crate::http::{header, Request};

    #[test]
    fn try_from() -> Result<()> {
        let req = Request::builder()
            .header(
                header::PROXY_AUTHORIZATION,
                "Basic dXNlcm5hbWU6cGFzc3dvcmQ=".parse().unwrap(),
            )
            .build()
            .unwrap();

        assert_eq!(
            Credentials::try_from(&req)?,
            Credentials::new("Basic", "dXNlcm5hbWU6cGFzc3dvcmQ=")
        );

        Ok(())
    }

    #[test]
    fn try_from_header_not_set() -> Result<()> {
        let err = Credentials::try_from(&Request::default()).err().unwrap();

        assert!(matches!(err, super::Error::MissingHeader));

        Ok(())
    }

    #[test]
    fn try_from_fields_lacking() -> Result<()> {
        let req = Request::builder()
            .header(header::PROXY_AUTHORIZATION, "Scheme".parse().unwrap())
            .build()
            .unwrap();
        let err = Credentials::try_from(&req).err().unwrap();

        assert!(matches!(err, super::Error::InvalidFormat { n: 1, .. }));

        Ok(())
    }

    #[test]
    fn try_from_too_many_fields() -> Result<()> {
        let req = Request::builder()
            .header(
                header::PROXY_AUTHORIZATION,
                "Scheme Value Extra".parse().unwrap(),
            )
            .build()
            .unwrap();
        let err = Credentials::try_from(&req).err().unwrap();

        assert!(matches!(err, super::Error::InvalidFormat { n: 3, .. }));

        Ok(())
    }
}
