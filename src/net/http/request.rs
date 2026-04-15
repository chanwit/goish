// http::Request — incoming server request / outgoing client request.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   r.Method                            r.Method
//   r.URL.Path                          r.URL.Path
//   r.URL.Query().Get("k")              r.URL.Query().Get("k")
//   r.Header.Get("User-Agent")          r.Header.Get("User-Agent")
//   r.Body                              r.Body
//   r.RemoteAddr                        r.RemoteAddr
//   r.FormValue("name")                 r.FormValue("name")

use crate::net::http::body::Body;
use crate::net::url::URL;
use crate::types::{map, slice, string};

/// `http.Header` — case-insensitive multimap of header name → values.
/// Internally keys are stored in canonical (Header-Title-Case) form.
#[derive(Debug, Clone, Default)]
pub struct Header {
    // map<canonical-key, list-of-values>
    inner: map<string, slice<string>>,
}

impl Header {
    pub fn new() -> Self { Header::default() }

    #[allow(non_snake_case)]
    pub fn Get(&self, key: &str) -> string {
        let k = canonicalize(key);
        self.inner.get(&k).and_then(|v| v.first().cloned()).unwrap_or_default()
    }

    #[allow(non_snake_case)]
    pub fn Set(&mut self, key: &str, value: &str) {
        let k = canonicalize(key);
        self.inner.insert(k, vec![value.to_owned()]);
    }

    #[allow(non_snake_case)]
    pub fn Add(&mut self, key: &str, value: &str) {
        let k = canonicalize(key);
        self.inner.entry(k).or_default().push(value.to_owned());
    }

    #[allow(non_snake_case)]
    pub fn Del(&mut self, key: &str) {
        let k = canonicalize(key);
        self.inner.remove(&k);
    }

    #[allow(non_snake_case)]
    pub fn Values(&self, key: &str) -> slice<string> {
        let k = canonicalize(key);
        self.inner.get(&k).cloned().unwrap_or_default()
    }

    /// Iterate over all (key, values) pairs. Used by the server/client
    /// wire formatters.
    pub fn iter(&self) -> impl Iterator<Item = (&string, &slice<string>)> {
        self.inner.iter()
    }

    pub fn len(&self) -> usize { self.inner.len() }
}

pub(crate) fn canonicalize(k: &str) -> string {
    // Go's MIME canonical form: first char upper, after every `-` upper,
    // rest lower. `content-type` → `Content-Type`.
    let mut out = String::with_capacity(k.len());
    let mut upper_next = true;
    for c in k.chars() {
        if c == '-' {
            out.push('-');
            upper_next = true;
        } else if upper_next {
            out.extend(c.to_uppercase());
            upper_next = false;
        } else {
            out.extend(c.to_lowercase());
        }
    }
    out
}

/// `http.Request` — represents an incoming server request or an outgoing
/// client request.
pub struct Request {
    pub Method: string,
    pub URL: URL,
    pub Proto: string,
    pub Header: Header,
    pub Body: Body,
    pub Host: string,
    pub RemoteAddr: string,
    pub ContentLength: crate::types::int64,
    ctx: crate::context::Context,
}

/// Argument-converter for request bodies. Lets `NewRequest(method, url, body)`
/// accept `nil`, `&[u8]`, `Vec<u8>`, or `&str` — matching Go's `nil` /
/// `strings.NewReader(s)` / `bytes.NewReader(b)` call-site variants.
pub trait IntoReqBody {
    fn into_req_body(self) -> Vec<u8>;
}
impl IntoReqBody for &[u8] { fn into_req_body(self) -> Vec<u8> { self.to_vec() } }
impl IntoReqBody for Vec<u8> { fn into_req_body(self) -> Vec<u8> { self } }
impl IntoReqBody for &str { fn into_req_body(self) -> Vec<u8> { self.as_bytes().to_vec() } }
impl IntoReqBody for &String { fn into_req_body(self) -> Vec<u8> { self.as_bytes().to_vec() } }
// `nil` (errors::error with None payload) means "no body".
impl IntoReqBody for crate::errors::error {
    fn into_req_body(self) -> Vec<u8> { Vec::new() }
}

impl Request {
    /// `http.NewRequest(method, url, body)` — build an outgoing request.
    /// Body accepts `nil`, `&[u8]`, `Vec<u8>`, or `&str` — see `IntoReqBody`.
    #[allow(non_snake_case)]
    pub fn new<B: IntoReqBody>(method: &str, target: &str, body: B) -> (Request, crate::errors::error) {
        let body_bytes = body.into_req_body();
        let (u, err) = crate::net::url::Parse(target);
        if err != crate::errors::nil {
            return (
                Request {
                    Method: method.to_owned(),
                    URL: URL::default(),
                    Proto: "HTTP/1.1".to_owned(),
                    Header: Header::new(),
                    Body: Body::empty(),
                    Host: String::new(),
                    RemoteAddr: String::new(),
                    ContentLength: 0,
                    ctx: crate::context::Background(),
                },
                err,
            );
        }
        let host = u.Host.clone();
        let cl = body_bytes.len() as crate::types::int64;
        (
            Request {
                Method: method.to_owned(),
                URL: u,
                Proto: "HTTP/1.1".to_owned(),
                Header: Header::new(),
                Body: Body::from_bytes(body_bytes),
                Host: host,
                RemoteAddr: String::new(),
                ContentLength: cl,
                ctx: crate::context::Background(),
            },
            crate::errors::nil,
        )
    }

    /// `r.Context()` — returns the request's context (for cancellation /
    /// deadline propagation). Mirrors Go's `*http.Request.Context`.
    #[allow(non_snake_case)]
    pub fn Context(&self) -> crate::context::Context {
        self.ctx.clone()
    }

    /// `r.WithContext(ctx)` — returns a shallow-copied request bound to
    /// the given context.
    #[allow(non_snake_case)]
    pub fn WithContext(mut self, ctx: crate::context::Context) -> Request {
        self.ctx = ctx;
        self
    }

    /// Internal: let the server wire a per-connection context.
    pub(crate) fn set_context(&mut self, ctx: crate::context::Context) {
        self.ctx = ctx;
    }

    /// Internal constructor used by the server when translating an
    /// incoming hyper request into a goish `Request`.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_parts(
        method: string,
        url: URL,
        proto: string,
        header: Header,
        body: Body,
        host: string,
        remote_addr: string,
        content_length: crate::types::int64,
    ) -> Request {
        Request {
            Method: method,
            URL: url,
            Proto: proto,
            Header: header,
            Body: body,
            Host: host,
            RemoteAddr: remote_addr,
            ContentLength: content_length,
            ctx: crate::context::Background(),
        }
    }

    /// `r.FormValue(key)` — returns the first value for the named query
    /// parameter (form-body parsing not yet implemented; URL query only).
    #[allow(non_snake_case)]
    pub fn FormValue(&self, key: &str) -> string {
        self.URL.Query().Get(key)
    }

    /// `r.SetBasicAuth(user, pass)` — sets the Authorization header.
    #[allow(non_snake_case)]
    pub fn SetBasicAuth(&mut self, username: &str, password: &str) {
        use crate::base64;
        let token = format!("{}:{}", username, password);
        let enc = base64::StdEncoding.EncodeToString(token.as_bytes());
        self.Header.Set("Authorization", &format!("Basic {}", enc));
    }

    /// `r.BasicAuth()` — returns (username, password, ok) from the Authorization header.
    #[allow(non_snake_case)]
    pub fn BasicAuth(&self) -> (string, string, bool) {
        let auth = self.Header.Get("Authorization");
        if !auth.starts_with("Basic ") {
            return (String::new(), String::new(), false);
        }
        let (user, pass, ok) = parse_basic_auth(&auth);
        (user, pass, ok)
    }
}

/// Shared parser for `Basic dXNlcjpwYXNz` header values.
pub(crate) fn parse_basic_auth(auth: &str) -> (string, string, bool) {
    if !auth.starts_with("Basic ") { return (String::new(), String::new(), false); }
    let token = &auth[6..];
    let (decoded, err) = crate::base64::StdEncoding.DecodeString(token);
    if err != crate::errors::nil { return (String::new(), String::new(), false); }
    let s = match std::str::from_utf8(&decoded) {
        Ok(s) => s,
        Err(_) => return (String::new(), String::new(), false),
    };
    match s.find(':') {
        Some(i) => (s[..i].to_string(), s[i+1..].to_string(), true),
        None => (String::new(), String::new(), false),
    }
}

/// ParseHTTPVersion parses an HTTP version string according to RFC 7230, section 2.6.
/// Examples: "HTTP/1.0" → (1, 0, true), "HTTP/1.1" → (1, 1, true).
#[allow(non_snake_case)]
pub fn ParseHTTPVersion(vers: &str) -> (crate::types::int, crate::types::int, bool) {
    const BIG: u32 = 1_000_000;
    if !vers.starts_with("HTTP/") { return (0, 0, false); }
    let rest = &vers[5..];
    let dot = match rest.find('.') { Some(i) => i, None => return (0, 0, false) };
    let (major_s, rest2) = rest.split_at(dot);
    let minor_s = &rest2[1..];
    // Disallow leading zeros except "0".
    if major_s.is_empty() || minor_s.is_empty() { return (0, 0, false); }
    if major_s.len() > 1 && major_s.starts_with('0') { return (0, 0, false); }
    if minor_s.len() > 1 && minor_s.starts_with('0') { return (0, 0, false); }
    let major: u32 = match major_s.parse() { Ok(n) if n < BIG => n, _ => return (0, 0, false) };
    let minor: u32 = match minor_s.parse() { Ok(n) if n < BIG => n, _ => return (0, 0, false) };
    (major as crate::types::int, minor as crate::types::int, true)
}

