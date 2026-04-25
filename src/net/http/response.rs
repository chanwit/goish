// http::Response (client side) + http::ResponseWriter (server side).

use crate::errors::{error, nil};
use crate::net::http::body::Body;
use crate::net::http::request::Header;
use crate::types::{byte, int, int64, string};

/// `http.Response` — a client's view of a server's response.
///
///   (resp, err) := http.Get(url)
///   resp.StatusCode
///   resp.Header.Get("Content-Type")
///   resp.Body                     (io.ReadCloser)
pub struct Response {
    pub Status: string,       // "200 OK"
    pub StatusCode: int,      // 200
    pub Proto: string,        // "HTTP/1.1"
    pub Header: Header,
    pub Body: Body,
    pub ContentLength: int64,
    pub Request: Option<Box<crate::net::http::Request>>,
}

impl Response {
    pub(crate) fn empty(code: int) -> Response {
        Response {
            Status: format!("{} {}", code, crate::net::http::StatusText(code)).into(),
            StatusCode: code,
            Proto: "HTTP/1.1".into(),
            Header: Header::new(),
            Body: Body::empty(),
            ContentLength: 0,
            Request: None,
        }
    }
}

/// `http.ResponseWriter` — what a handler writes its response through.
///
///   fn handler(mut w: http::ResponseWriter, r: http::Request) {
///       w.Header().Set("Content-Type", "text/plain");
///       w.WriteHeader(200);
///       fmt::Fprintf!(&mut w, "hello %s", r.URL.Path);
///   }
pub struct ResponseWriter {
    pub(crate) header: Header,
    pub(crate) status: int,
    pub(crate) body: Vec<byte>,
    pub(crate) wrote_header: bool,
}

impl ResponseWriter {
    pub(crate) fn new() -> Self {
        ResponseWriter {
            header: Header::new(),
            status: 200,
            body: Vec::new(),
            wrote_header: false,
        }
    }

    /// `w.Header()` — return a mutable handle to the response headers.
    /// Mutations must happen BEFORE the first `Write` / `WriteHeader`.
    #[allow(non_snake_case)]
    pub fn Header(&mut self) -> &mut Header {
        &mut self.header
    }

    /// `w.WriteHeader(code)` — send the status line. If not called,
    /// the first `Write` implicitly sends 200.
    #[allow(non_snake_case)]
    pub fn WriteHeader(&mut self, code: int) {
        if !self.wrote_header {
            self.status = code;
            self.wrote_header = true;
        }
    }

    /// `w.Write(bytes)` — append to the response body. Implicitly sends
    /// status 200 on first call if `WriteHeader` wasn't used.
    #[allow(non_snake_case)]
    pub fn Write(&mut self, p: impl AsRef<[byte]>) -> (int, error) {
        let p = p.as_ref();
        if !self.wrote_header { self.wrote_header = true; }
        self.body.extend_from_slice(p);
        (p.len() as int, nil)
    }
}

// Blanket `std::io::Write` so `fmt::Fprintf!(&mut w, ...)` works on
// a ResponseWriter exactly like it does on a `bytes::Buffer`.
impl std::io::Write for ResponseWriter {
    fn write(&mut self, p: &[u8]) -> std::io::Result<usize> {
        if !self.wrote_header { self.wrote_header = true; }
        self.body.extend_from_slice(p);
        Ok(p.len())
    }
    fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
}
