//! Fixture navigation: redirects, compression, chunking, and cancellation
//! against the shared Node loopback fixture server
//! (`scripts/test/fixture-server-cli.mjs` / `fixture-server.mjs`), per the
//! M2-T02 acceptance criteria and fast gate.

mod support;

use std::time::Duration;

use machina_network::{
    ClientConfigOptions, NetworkClient, NetworkError, NormalizedUrl, RequestSpec,
};
use support::fixture_process::FixtureProcess;

fn client() -> NetworkClient {
    support::test_client()
}

fn client_with_options(options: ClientConfigOptions) -> NetworkClient {
    support::test_client_with_options(options)
}

#[test]
fn multi_origin_navigation_succeeds_on_two_independent_origins() {
    let fixture = FixtureProcess::spawn(2);
    let client = client();
    for index in 0..2 {
        let url = NormalizedUrl::parse(&format!("{}/navigation", fixture.origin(index)), None)
            .expect("valid url");
        let ctx = support::test_ctx();
        let (head, mut body) = client
            .fetch(RequestSpec::get(url), "session-nav", &ctx)
            .unwrap_or_else(|error| panic!("origin {index} navigation should succeed: {error}"));
        assert_eq!(head.status, http::StatusCode::OK);
        let bytes = body
            .read_to_end_bounded(64 * 1024, &client.handle())
            .expect("bounded read succeeds");
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            text.contains("Navigation fixture"),
            "origin {index}: {text}"
        );
    }
}

#[test]
fn same_origin_redirect_chain_is_followed_and_recorded() {
    let fixture = FixtureProcess::spawn(1);
    let client = client();
    let url = NormalizedUrl::parse(&format!("{}/redirect-chain?n=3", fixture.origin(0)), None)
        .expect("valid url");
    let ctx = support::test_ctx();
    let (head, mut body) = client
        .fetch(RequestSpec::get(url), "session-redirect", &ctx)
        .expect("redirect chain resolves");
    assert_eq!(head.status, http::StatusCode::OK);
    assert_eq!(
        head.redirect_chain.len(),
        3,
        "expected exactly 3 redirect hops"
    );
    let bytes = body
        .read_to_end_bounded(4096, &client.handle())
        .expect("read body");
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("\"done\":true"), "{text}");
}

#[test]
fn redirect_count_cap_is_enforced() {
    let fixture = FixtureProcess::spawn(1);
    let options = ClientConfigOptions {
        max_redirects: 5,
        ..ClientConfigOptions::default()
    };
    let client = client_with_options(options);
    let url = NormalizedUrl::parse(&format!("{}/redirect-chain?n=999", fixture.origin(0)), None)
        .expect("valid url");
    let ctx = support::test_ctx();
    match client.fetch(RequestSpec::get(url), "session-cap", &ctx) {
        Err(NetworkError::TooManyRedirects) => {}
        Err(other) => panic!("expected too-many-redirects, got error {other:?}"),
        Ok(_) => panic!("expected too-many-redirects, got a successful response"),
    }
}

#[test]
fn redirect_loop_is_detected() {
    let fixture = FixtureProcess::spawn(1);
    let client = client();
    let url = NormalizedUrl::parse(&format!("{}/redirect-loop", fixture.origin(0)), None)
        .expect("valid url");
    let ctx = support::test_ctx();
    match client.fetch(RequestSpec::get(url), "session-loop", &ctx) {
        Err(NetworkError::RedirectLoop) => {}
        Err(other) => panic!("expected a detected redirect loop, got error {other:?}"),
        Ok(_) => panic!("expected a detected redirect loop, got a successful response"),
    }
}

#[test]
fn same_origin_redirect_preserves_sensitive_headers() {
    let fixture = FixtureProcess::spawn(1);
    let client = client();
    let url = NormalizedUrl::parse(&format!("{}/redirect-chain?n=1", fixture.origin(0)), None)
        .expect("valid url");
    let mut headers = http::HeaderMap::new();
    headers.insert(
        http::header::AUTHORIZATION,
        http::HeaderValue::from_static("Bearer token"),
    );
    let spec = RequestSpec::new(http::Method::GET, url, headers, bytes::Bytes::new());
    let ctx = support::test_ctx();
    let (_, mut body) = client
        .fetch(spec, "session-same-origin", &ctx)
        .expect("same-origin redirect resolves");
    let bytes = body
        .read_to_end_bounded(4096, &client.handle())
        .expect("read body");
    let text = String::from_utf8_lossy(&bytes);
    assert!(
        text.contains("\"received_authorization\":true"),
        "same-origin redirect must not strip Authorization: {text}"
    );
}

#[test]
fn cross_origin_redirect_strips_sensitive_headers() {
    let fixture = FixtureProcess::spawn(2);
    let client = client();
    let to = fixture.origin(1);
    let url = NormalizedUrl::parse(
        &format!(
            "{}/redirect-chain?n=1&to={}",
            fixture.origin(0),
            urlencoding_safe(&to)
        ),
        None,
    )
    .expect("valid url");
    let mut headers = http::HeaderMap::new();
    headers.insert(
        http::header::AUTHORIZATION,
        http::HeaderValue::from_static("Bearer token"),
    );
    headers.insert(
        http::header::COOKIE,
        http::HeaderValue::from_static("session=abc"),
    );
    let spec = RequestSpec::new(http::Method::GET, url, headers, bytes::Bytes::new());
    let ctx = support::test_ctx();
    let (head, mut body) = client
        .fetch(spec, "session-cross-origin", &ctx)
        .expect("cross-origin redirect resolves");
    assert_eq!(head.redirect_chain.len(), 1);
    let bytes = body
        .read_to_end_bounded(4096, &client.handle())
        .expect("read body");
    let text = String::from_utf8_lossy(&bytes);
    assert!(
        text.contains("\"received_authorization\":false")
            && text.contains("\"received_cookie\":false"),
        "cross-origin redirect must strip sensitive headers: {text}"
    );
}

fn urlencoding_safe(origin: &str) -> String {
    origin.replace(':', "%3A").replace('/', "%2F")
}

#[test]
fn gzip_compressed_response_decodes_to_the_declared_length() {
    assert_compressed_roundtrip("gzip");
}

#[test]
fn deflate_compressed_response_decodes_to_the_declared_length() {
    assert_compressed_roundtrip("deflate");
}

#[test]
fn brotli_compressed_response_decodes_to_the_declared_length() {
    assert_compressed_roundtrip("br");
}

fn assert_compressed_roundtrip(encoding: &str) {
    let fixture = FixtureProcess::spawn(1);
    let client = client();
    let url = NormalizedUrl::parse(
        &format!("{}/compressed/{encoding}", fixture.origin(0)),
        None,
    )
    .expect("valid url");
    let ctx = support::test_ctx();
    let (head, mut body) = client
        .fetch(RequestSpec::get(url), "session-compressed", &ctx)
        .unwrap_or_else(|error| panic!("{encoding} response should decode: {error}"));
    let expected_length: usize = head
        .header_str("x-machina-decompressed-length")
        .expect("fixture reports decompressed length")
        .parse()
        .expect("numeric length");
    let bytes = body
        .read_to_end_bounded(1024 * 1024, &client.handle())
        .unwrap_or_else(|error| panic!("{encoding} decode should succeed: {error}"));
    assert_eq!(
        bytes.len(),
        expected_length,
        "decoded length mismatch for {encoding}"
    );
    assert!(String::from_utf8_lossy(&bytes).contains("machina-fixture-compressible-payload"));
}

#[test]
fn chunked_response_streams_every_chunk_without_full_buffering() {
    let fixture = FixtureProcess::spawn(1);
    let client = client();
    let url =
        NormalizedUrl::parse(&format!("{}/chunked", fixture.origin(0)), None).expect("valid url");
    let ctx = support::test_ctx();
    let (_, mut body) = client
        .fetch(RequestSpec::get(url), "session-chunked", &ctx)
        .expect("chunked response succeeds");
    let mut chunk_count = 0;
    let mut collected = Vec::new();
    let handle = client.handle();
    while let Some(chunk) = body
        .next_chunk_blocking(&handle)
        .expect("chunk read succeeds")
    {
        chunk_count += 1;
        collected.extend_from_slice(&chunk);
    }
    assert!(chunk_count >= 1, "expected at least one streamed chunk");
    let text = String::from_utf8_lossy(&collected);
    assert_eq!(text, "first-chunk;second-chunk;third-chunk;fourth-chunk");
}

#[test]
fn slow_trickle_is_aborted_at_the_configured_deadline() {
    let fixture = FixtureProcess::spawn(1);
    let client = client();
    // 20 chunks * 200ms = 4s total; a 250ms deadline must abort mid-stream.
    let url = NormalizedUrl::parse(
        &format!("{}/slow-trickle?delay_ms=200&chunks=20", fixture.origin(0)),
        None,
    )
    .expect("valid url");
    let ctx = support::test_ctx_with_timeout(Duration::from_millis(250));
    let result = client.fetch(RequestSpec::get(url), "session-deadline", &ctx);
    let handle = client.handle();
    let outcome = match result {
        Ok((_, mut body)) => loop {
            match body.next_chunk_blocking(&handle) {
                Ok(Some(_)) => continue,
                Ok(None) => break Ok(()),
                Err(error) => break Err(error),
            }
        },
        Err(error) => Err(error),
    };
    assert!(
        matches!(outcome, Err(NetworkError::DeadlineExceeded)),
        "expected the deadline to fire mid-stream, got {outcome:?}"
    );
}

#[test]
fn slow_trickle_is_aborted_on_explicit_cancellation() {
    let fixture = FixtureProcess::spawn(1);
    let client = client();
    let url = NormalizedUrl::parse(
        &format!("{}/slow-trickle?delay_ms=200&chunks=50", fixture.origin(0)),
        None,
    )
    .expect("valid url");
    let ctx = support::test_ctx_with_timeout(Duration::from_secs(30));
    // Fetch headers first (uncancelled), then cancel partway through the
    // still-trickling body -- isolates "cancellation aborts an in-flight
    // body read" from connect/handshake timing, which is what this test is
    // about.
    let (_, mut body) = client
        .fetch(RequestSpec::get(url), "session-cancel", &ctx)
        .expect("headers arrive before any cancellation");
    let token = ctx.cancellation.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(150));
        token.cancel();
    });
    let handle = client.handle();
    let outcome = loop {
        match body.next_chunk_blocking(&handle) {
            Ok(Some(_)) => continue,
            Ok(None) => break Ok(()),
            Err(error) => break Err(error),
        }
    };
    assert!(
        matches!(outcome, Err(NetworkError::Cancelled)),
        "expected explicit cancellation to abort the stream, got {outcome:?}"
    );
}

#[test]
fn form_submission_round_trips_through_a_fixture_origin() {
    let fixture = FixtureProcess::spawn(1);
    let client = client();
    let url =
        NormalizedUrl::parse(&format!("{}/form", fixture.origin(0)), None).expect("valid url");
    let mut headers = http::HeaderMap::new();
    headers.insert(
        http::header::CONTENT_TYPE,
        http::HeaderValue::from_static("application/x-www-form-urlencoded"),
    );
    let spec = RequestSpec::new(
        http::Method::POST,
        url,
        headers,
        bytes::Bytes::from_static(b"name=fixture"),
    );
    let ctx = support::test_ctx();
    let (head, mut body) = client
        .fetch(spec, "session-form", &ctx)
        .expect("form submission succeeds");
    assert_eq!(head.status, http::StatusCode::OK);
    let bytes = body
        .read_to_end_bounded(4096, &client.handle())
        .expect("read body");
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("\"accepted\":true"), "{text}");
}
