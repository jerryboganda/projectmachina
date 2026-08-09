//! Outgoing request description. Bodies are pre-buffered `Bytes` -- a
//! deliberate scope decision documented in the evidence file: request
//! bodies for the navigation/fetch flows this loader serves are small
//! (forms, JSON payloads), so buffering them once makes them trivially
//! replayable across a 307/308 redirect without a one-shot-body-consumed
//! failure mode. This is unrelated to the *response* streaming guarantee
//! (no mandatory full buffering), which this crate does not relax anywhere.

use bytes::Bytes;
use http::{HeaderMap, Method};

use crate::url::NormalizedUrl;

#[derive(Clone, Debug)]
pub struct RequestSpec {
    pub method: Method,
    pub url: NormalizedUrl,
    pub headers: HeaderMap,
    pub body: Bytes,
}

impl RequestSpec {
    pub fn get(url: NormalizedUrl) -> Self {
        Self {
            method: Method::GET,
            url,
            headers: HeaderMap::new(),
            body: Bytes::new(),
        }
    }

    pub fn new(method: Method, url: NormalizedUrl, headers: HeaderMap, body: Bytes) -> Self {
        Self {
            method,
            url,
            headers,
            body,
        }
    }
}
