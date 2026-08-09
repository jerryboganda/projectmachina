//! Malformed-response fast gate: a minimal raw-TCP fixture (per the M2-T02
//! design section 8) for wire-level malformations Node's `http` module
//! cannot be coaxed into producing (it enforces valid framing). Each case
//! writes exact bytes a conforming server would never send and asserts the
//! loader fails closed with a typed error rather than accepting a
//! best-effort parse.

mod support;

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use machina_network::{NetworkError, NormalizedUrl, RequestSpec};

fn spawn_raw_response(bytes: &'static [u8]) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral loopback port");
    let port = listener
        .local_addr()
        .expect("listener has a local address")
        .port();
    thread::spawn(move || {
        if let Ok((mut socket, _)) = listener.accept() {
            let mut buffer = [0_u8; 4096];
            // Drain (some of) the request; content doesn't matter for these
            // cases, only the response bytes we write back.
            let _ = socket.read(&mut buffer);
            let _ = socket.write_all(bytes);
            let _ = socket.flush();
            let _ = socket.shutdown(std::net::Shutdown::Write);
        }
    });
    port
}

fn fetch_from(port: u16) -> Result<(), NetworkError> {
    let client = support::test_client();
    let url = NormalizedUrl::parse(&format!("http://127.0.0.1:{port}/"), None)
        .expect("loopback url parses");
    let ctx = support::test_ctx();
    let (_, mut body) = client.fetch(RequestSpec::get(url), "session-malformed", &ctx)?;
    // Drain the body: some malformations only surface once streaming
    // begins (e.g. an invalid chunk-size line arrives after valid headers).
    let handle = client.handle();
    loop {
        match body.next_chunk_blocking(&handle) {
            Ok(Some(_)) => continue,
            Ok(None) => return Ok(()),
            Err(error) => return Err(error),
        }
    }
}

#[test]
fn garbage_status_line_is_rejected() {
    let port = spawn_raw_response(b"NOT A VALID HTTP RESPONSE AT ALL\r\n\r\n");
    let result = fetch_from(port);
    assert!(
        matches!(result, Err(NetworkError::ProtocolError(_))),
        "expected a protocol error, got {result:?}"
    );
}

#[test]
fn conflicting_content_length_and_transfer_encoding_is_rejected() {
    let port = spawn_raw_response(
        b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n5\r\nhello\r\n0\r\n\r\n",
    );
    let result = fetch_from(port);
    assert!(
        matches!(result, Err(NetworkError::ProtocolError(_))),
        "smuggling-shaped response must be rejected, got {result:?}"
    );
}

#[test]
fn invalid_chunk_size_hex_is_rejected() {
    let port = spawn_raw_response(
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\nZZZZ\r\nhello\r\n0\r\n\r\n",
    );
    let result = fetch_from(port);
    assert!(
        matches!(result, Err(NetworkError::ProtocolError(_))),
        "invalid chunk-size line must be rejected, got {result:?}"
    );
}

#[test]
fn truncated_chunked_body_is_rejected() {
    // Declares a 20-byte chunk but the connection closes after 5 bytes.
    let port = spawn_raw_response(
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n14\r\nhello",
    );
    let result = fetch_from(port);
    assert!(
        matches!(result, Err(NetworkError::ProtocolError(_))),
        "truncated chunked body must be rejected, got {result:?}"
    );
}

#[test]
fn invalid_content_length_value_is_rejected() {
    let port = spawn_raw_response(
        b"HTTP/1.1 200 OK\r\nContent-Length: not-a-number\r\nConnection: close\r\n\r\nhello",
    );
    let result = fetch_from(port);
    assert!(
        matches!(result, Err(NetworkError::ProtocolError(_))),
        "non-numeric content-length must be rejected, got {result:?}"
    );
}

#[test]
fn oversized_header_block_is_rejected() {
    let mut response = String::from("HTTP/1.1 200 OK\r\n");
    // One absurdly large header value, larger than the loader's configured
    // header buffer ceiling (default 64 KiB).
    response.push_str("X-Fixture-Oversized: ");
    response.push_str(&"a".repeat(200 * 1024));
    response.push_str("\r\nConnection: close\r\n\r\nbody");
    let leaked: &'static [u8] = Box::leak(response.into_bytes().into_boxed_slice());
    let port = spawn_raw_response(leaked);
    let result = fetch_from(port);
    assert!(
        matches!(result, Err(NetworkError::ProtocolError(_))),
        "oversized header block must be rejected, got {result:?}"
    );
}

#[test]
fn unsupported_content_encoding_fails_closed_rather_than_passing_through() {
    let port = spawn_raw_response(
        b"HTTP/1.1 200 OK\r\nContent-Encoding: bogus-encoding\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello",
    );
    let result = fetch_from(port);
    assert!(
        matches!(result, Err(NetworkError::DecompressionFailed(_))),
        "unsupported content-encoding must fail closed, got {result:?}"
    );
}

#[test]
fn stacked_content_encoding_is_rejected() {
    let port = spawn_raw_response(
        b"HTTP/1.1 200 OK\r\nContent-Encoding: gzip, br\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello",
    );
    let result = fetch_from(port);
    assert!(
        matches!(result, Err(NetworkError::DecompressionFailed(_))),
        "stacked content-encoding must be rejected, got {result:?}"
    );
}

#[test]
fn well_formed_response_still_succeeds_against_the_same_fixture_shape() {
    // Sanity check the fixture harness itself: a well-formed response over
    // the same raw-TCP path must succeed, so failures above are provably
    // about the malformation, not the harness.
    let port = spawn_raw_response(
        b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello",
    );
    let result = fetch_from(port);
    assert!(
        result.is_ok(),
        "well-formed response must succeed, got {result:?}"
    );
}
