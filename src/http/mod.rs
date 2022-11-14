pub mod request;
pub mod response;

pub use http::{header::HeaderName, HeaderMap, HeaderValue};
pub use hyper::{header, Method, StatusCode, Uri, Version};

pub use self::{request::Request, response::Response};

pub type Headers = HeaderMap<HeaderValue>;
pub type Payload = Vec<u8>;

/// Hop-by-hop headers to remove right before send request to remote
/// https://www.rfc-editor.org/rfc/rfc2616#section-13.5.1
const HOP_BY_HOP_HEADERS: &[HeaderName] = &[
    header::PROXY_AUTHENTICATE,
    header::PROXY_AUTHORIZATION,
    header::CONNECTION,
    header::TE,
    header::TRAILER,
    header::TRANSFER_ENCODING,
    header::UPGRADE,
];

/// Non-standard hop-by-hop headers not listed in HTTP header module
const HOP_BY_HOP_HEADERS_NONSTD: &[&str] = &["Proxy-Connection", "Keep-Alive"];

/// Remove all hop-by-hop headers, in-place.
pub fn remove_hop_by_hop_headers(headers: &mut Headers) {
    for k in HOP_BY_HOP_HEADERS {
        let _ = headers.remove(k);
    }
    for k in HOP_BY_HOP_HEADERS_NONSTD {
        let _ = headers.remove(k.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::{header, HeaderName, Headers};

    #[test]
    fn remove_hop_by_hop_headers() {
        let mut headers = Headers::new();
        headers.append(header::CONTENT_TYPE, "application/json".parse().unwrap());
        headers.append(header::PROXY_AUTHORIZATION, "Bearer TOKEN".parse().unwrap());
        headers.append("Proxy-Connection", "keep-alive".parse().unwrap());

        super::remove_hop_by_hop_headers(&mut headers);

        assert_eq!(
            headers.keys().collect::<Vec<&HeaderName>>(),
            vec![header::CONTENT_TYPE]
        );
    }

    #[test]
    fn remove_hop_by_hop_headers_multiple_keys() {
        let mut headers = Headers::new();
        headers.append(header::CONTENT_TYPE, "application/json".parse().unwrap());
        headers.append(header::PROXY_AUTHENTICATE, "Basic".parse().unwrap());
        headers.append(header::PROXY_AUTHENTICATE, "Bearer".parse().unwrap());

        super::remove_hop_by_hop_headers(&mut headers);

        assert_eq!(
            headers.keys().collect::<Vec<&HeaderName>>(),
            vec![header::CONTENT_TYPE]
        );
    }
}
