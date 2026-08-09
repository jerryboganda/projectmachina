//! Response head + the redirect chain that produced it.

use http::{HeaderMap, StatusCode, Version};

use crate::url::NormalizedUrl;

#[derive(Clone, Debug)]
pub struct RedirectHop {
    pub url: NormalizedUrl,
    pub status: StatusCode,
}

#[derive(Clone, Debug)]
pub struct ResponseHead {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub http_version: Version,
    pub final_url: NormalizedUrl,
    pub redirect_chain: Vec<RedirectHop>,
}

impl ResponseHead {
    pub fn header_str(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|value| value.to_str().ok())
    }
}
