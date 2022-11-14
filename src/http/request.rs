use derive_builder::Builder;

use super::{HeaderName, HeaderValue, Headers, Method, Payload, Uri, Version};

#[derive(Clone, Debug, Default, PartialEq, Eq, Builder)]
#[builder(default)]
pub struct Request {
    pub method: Method,
    pub uri: Uri,
    pub version: Version,

    #[builder(setter(custom))]
    pub headers: Headers,

    pub payload: Payload,
}

impl Request {
    /// Create new request with default params.
    pub fn new<P>(method: Method, uri: Uri, version: Version, headers: Headers, payload: P) -> Self
    where
        P: Into<Payload>,
    {
        Self {
            method,
            uri,
            version,
            headers,
            payload: payload.into(),
        }
    }

    pub fn builder() -> RequestBuilder {
        RequestBuilder::default()
    }

    pub async fn from(req: hyper::Request<hyper::Body>) -> Self {
        let (parts, body) = req.into_parts();
        let bytes = hyper::body::to_bytes(body)
            .await
            .expect("failed to read bytes");

        Self::new(parts.method, parts.uri, parts.version, parts.headers, bytes)
    }
}

impl From<Request> for hyper::Request<hyper::Body> {
    fn from(val: Request) -> Self {
        let mut builder = hyper::Request::builder()
            .method(val.method)
            .uri(val.uri)
            .version(val.version);

        *(builder.headers_mut().unwrap()) = val.headers;

        builder.body(val.payload.into()).unwrap()
    }
}

impl RequestBuilder {
    pub fn header(&mut self, key: HeaderName, value: HeaderValue) -> &mut Self {
        self.headers
            .get_or_insert_with(Headers::default)
            .insert(key, value);
        self
    }

    pub fn headers(&mut self, headers: Headers) -> &mut Self {
        self.headers
            .get_or_insert_with(Headers::default)
            .extend(headers);
        self
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use http::{Method, Uri, Version};

    use super::Request;

    #[tokio::test]
    async fn request_from_hyper() -> Result<()> {
        let hyper_req = hyper::Request::new(hyper::Body::from("Hello World!"));
        let req = Request::from(hyper_req).await;

        assert_eq!(req.method, Method::GET);
        assert_eq!(req.uri, Uri::from_static("/"));
        assert_eq!(req.version, Version::HTTP_11);
        assert!(req.headers.is_empty());
        assert_eq!(req.payload, b"Hello World!");

        Ok(())
    }

    #[tokio::test]
    async fn request_into_hyper() -> Result<()> {
        let req = Request::default();
        let hyper_req: hyper::Request<hyper::body::Body> = req.into();

        assert_eq!(*hyper_req.method(), Method::GET);
        assert_eq!(*hyper_req.uri(), Uri::from_static("/"));
        assert_eq!(hyper_req.version(), Version::HTTP_11);
        assert!(hyper_req.headers().is_empty());
        assert!(hyper_req.extensions().is_empty());
        assert_eq!(hyper::body::to_bytes(hyper_req.into_body()).await?, vec![]);

        Ok(())
    }
}
