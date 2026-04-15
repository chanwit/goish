// http::Client — Go-shaped HTTP client backed by hyper 1.x.
//
//   (resp, err) := http.Get(url)
//   (resp, err) := http.Post(url, "application/json", body)
//   client := http.Client{ Timeout: 10*time.Second }
//   (resp, err) := client.Do(req)

use crate::errors::{error, nil, New};
use crate::net::http::body::Body;
use crate::net::http::request::{Header, Request};
use crate::net::http::response::Response;
use crate::net::url::URL;
use crate::types::{byte, int, int64, string};
use std::sync::OnceLock;
use std::time::Duration;

/// `http.Client` — reusable HTTP client. Holds a per-client Timeout and
/// (eventually) per-client transport config. `DefaultClient` is the
/// process-wide shared instance reachable as `http::DefaultClient()`.
#[derive(Clone, Default)]
pub struct Client {
    /// Timeout for the entire request. `0` means no timeout.
    pub Timeout: crate::time::Duration,
}

impl Client {
    pub fn new() -> Self { Client { Timeout: crate::time::Duration::from_nanos(0) } }

    /// `client.Get(url)` — issue a GET. Shortcut for `Do(NewRequest("GET", url, nil))`.
    #[allow(non_snake_case)]
    pub fn Get(&self, url: &str) -> (Response, error) {
        let (req, err) = Request::new("GET", url, &[]);
        if err != nil { return (Response::empty(0), err); }
        self.Do(req)
    }

    /// `client.Head(url)` — issue a HEAD.
    #[allow(non_snake_case)]
    pub fn Head(&self, url: &str) -> (Response, error) {
        let (req, err) = Request::new("HEAD", url, &[]);
        if err != nil { return (Response::empty(0), err); }
        self.Do(req)
    }

    /// `client.Post(url, contentType, body)` — issue a POST with the
    /// given body bytes.
    #[allow(non_snake_case)]
    pub fn Post(&self, url: &str, content_type: &str, body: &[byte]) -> (Response, error) {
        let (mut req, err) = Request::new("POST", url, body);
        if err != nil { return (Response::empty(0), err); }
        req.Header.Set("Content-Type", content_type);
        self.Do(req)
    }

    /// `client.PostForm(url, values)` — form-urlencoded POST.
    #[allow(non_snake_case)]
    pub fn PostForm(&self, url: &str, values: &crate::net::url::Values) -> (Response, error) {
        let body = values.Encode();
        self.Post(url, "application/x-www-form-urlencoded", body.as_bytes())
    }

    /// `client.Do(req)` — send `req` and read the full response. Blocks
    /// the caller; safely handles being called from within a tokio
    /// context (e.g. inside another HTTP handler). Honors both
    /// `client.Timeout` and the request's bound context (cancel / deadline).
    #[allow(non_snake_case)]
    pub fn Do(&self, mut req: Request) -> (Response, error) {
        let timeout = self.Timeout;
        let ctx = req.Context();
        super::block_on(async move {
            let fut = do_request_with_ctx(&mut req, ctx.clone());
            if timeout.Nanoseconds() > 0 {
                let std_dur = Duration::from_nanos(timeout.Nanoseconds() as u64);
                match tokio::time::timeout(std_dur, fut).await {
                    Ok(r) => r,
                    Err(_) => (Response::empty(0), New("http: request timeout")),
                }
            } else {
                fut.await
            }
        })
    }
}

/// `http.DefaultClient` — Go's shared, zero-timeout client.
#[allow(non_snake_case)]
pub fn DefaultClient() -> &'static Client {
    static C: OnceLock<Client> = OnceLock::new();
    C.get_or_init(Client::new)
}

/// `http.Get(url)` — shortcut for `DefaultClient().Get(url)`.
#[allow(non_snake_case)]
pub fn Get(url: &str) -> (Response, error) { DefaultClient().Get(url) }

/// `http.Head(url)`
#[allow(non_snake_case)]
pub fn Head(url: &str) -> (Response, error) { DefaultClient().Head(url) }

/// `http.Post(url, contentType, body)`
#[allow(non_snake_case)]
pub fn Post(url: &str, content_type: &str, body: &[byte]) -> (Response, error) {
    DefaultClient().Post(url, content_type, body)
}

/// `http.PostForm(url, values)`
#[allow(non_snake_case)]
pub fn PostForm(url: &str, values: &crate::net::url::Values) -> (Response, error) {
    DefaultClient().PostForm(url, values)
}

/// `http.Do(req)` — convenience for `DefaultClient().Do(req)`.
#[allow(non_snake_case)]
pub fn Do(req: Request) -> (Response, error) { DefaultClient().Do(req) }

/// `http.NewRequest(method, url, body)` — re-exported at the http::
/// namespace alongside the client funcs for call-site parity with Go.
#[allow(non_snake_case)]
pub fn NewRequest(method: &str, url: &str, body: &[byte]) -> (Request, error) {
    Request::new(method, url, body)
}

/// `http.NewRequestWithContext(ctx, method, url, body)` — identical to
/// `NewRequest` but binds the given context to the request. When the
/// context is cancelled, any in-flight `Do(req)` terminates with an
/// error. Mirrors Go's `http.NewRequestWithContext`.
#[allow(non_snake_case)]
pub fn NewRequestWithContext(
    ctx: crate::context::Context,
    method: &str,
    url: &str,
    body: &[byte],
) -> (Request, error) {
    let (mut req, err) = Request::new(method, url, body);
    if err != nil { return (req, err); }
    req = req.WithContext(ctx);
    (req, nil)
}

// ── the actual async transport ────────────────────────────────────────

/// Wraps `do_request` with a race against `ctx.Done()` so a cancelled
/// context aborts the in-flight request exactly like Go's client.
async fn do_request_with_ctx(
    req: &mut Request,
    ctx: crate::context::Context,
) -> (Response, error) {
    // Short-circuit on already-cancelled contexts.
    if ctx.Err() != nil { return (Response::empty(0), ctx.Err()); }
    let done = ctx.Done();
    tokio::select! {
        res = do_request(req) => res,
        _ = done.recv() => (Response::empty(0), ctx.Err()),
    }
}

async fn do_request(req: &mut Request) -> (Response, error) {
    use http_body_util::{BodyExt, Full};
    use hyper::Uri;
    use hyper_util::rt::TokioIo;
    use tokio::net::TcpStream;

    // Reconstruct a URI hyper can consume. Our URL has Scheme/Host/Path/RawQuery.
    let uri_str = build_uri(&req.URL);
    let uri: Uri = match uri_str.parse() {
        Ok(u) => u,
        Err(e) => return (Response::empty(0), New(&format!("http: invalid url: {}", e))),
    };
    if uri.scheme_str() != Some("http") {
        return (
            Response::empty(0),
            New("http: only http:// scheme is supported in v0.5.0 (no TLS yet)"),
        );
    }

    let host = match uri.host() {
        Some(h) => h.to_owned(),
        None => return (Response::empty(0), New("http: missing host in URL")),
    };
    let port = uri.port_u16().unwrap_or(80);
    let authority = format!("{}:{}", host, port);

    let stream = match TcpStream::connect(&authority).await {
        Ok(s) => s,
        Err(e) => return (Response::empty(0), New(&format!("http: dial {}: {}", authority, e))),
    };
    let io = TokioIo::new(stream);

    let (mut sender, conn) = match hyper::client::conn::http1::handshake(io).await {
        Ok(p) => p,
        Err(e) => return (Response::empty(0), New(&format!("http: handshake: {}", e))),
    };
    tokio::spawn(async move {
        let _ = conn.await;
    });

    // Build the hyper request.
    let mut builder = hyper::Request::builder()
        .method(req.Method.as_str())
        .uri(path_and_query(&req.URL));
    // Host header is mandatory for HTTP/1.1.
    builder = builder.header("Host", &authority);

    for (k, vs) in req.Header.iter() {
        for v in vs {
            builder = builder.header(k.as_str(), v);
        }
    }

    // Drain the outbound request body.
    let body_bytes = req.Body.Bytes();
    let body = Full::new(bytes::Bytes::from(body_bytes));

    let hyper_req = match builder.body(body) {
        Ok(r) => r,
        Err(e) => return (Response::empty(0), New(&format!("http: build request: {}", e))),
    };

    let hyper_resp = match sender.send_request(hyper_req).await {
        Ok(r) => r,
        Err(e) => return (Response::empty(0), New(&format!("http: send: {}", e))),
    };

    let status = hyper_resp.status();
    let proto = format!("{:?}", hyper_resp.version());
    let mut hdr = Header::new();
    for (name, value) in hyper_resp.headers().iter() {
        if let Ok(s) = value.to_str() {
            hdr.Add(name.as_str(), s);
        }
    }

    let bytes_body = match hyper_resp.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => return (Response::empty(0), New(&format!("http: read body: {}", e))),
    };
    let cl = bytes_body.len() as int64;

    (
        Response {
            Status: format!("{} {}", status.as_u16(), status.canonical_reason().unwrap_or("")),
            StatusCode: status.as_u16() as int,
            Proto: proto,
            Header: hdr,
            Body: Body::from_bytes(bytes_body.to_vec()),
            ContentLength: cl,
            Request: None,
        },
        nil,
    )
}

fn build_uri(u: &URL) -> string {
    let scheme = if u.Scheme.is_empty() { "http" } else { u.Scheme.as_str() };
    let mut s = format!("{}://{}", scheme, u.Host);
    if u.Path.is_empty() {
        s.push('/');
    } else {
        s.push_str(&u.Path);
    }
    if !u.RawQuery.is_empty() {
        s.push('?');
        s.push_str(&u.RawQuery);
    }
    s
}

fn path_and_query(u: &URL) -> string {
    let mut s = if u.Path.is_empty() { "/".to_owned() } else { u.Path.clone() };
    if !u.RawQuery.is_empty() {
        s.push('?');
        s.push_str(&u.RawQuery);
    }
    s
}
