// http::Body — Go's http.Response.Body / io.ReadCloser.
//
// Go: body is an io.ReadCloser you must Close() to free the connection.
// goish: same shape. Implements io::Reader (goish's Go-shaped Reader
// trait) and exposes Close()/Bytes()/String() for ergonomic access.

use crate::errors::{error, nil};
use crate::types::string;
use std::sync::Mutex;

/// `resp.Body` — a streaming response body. Read via `io::ReadAll(&mut body)`
/// or the shortcut `body.Bytes()` / `body.String()`. `Close()` when done.
pub struct Body {
    inner: Mutex<BodyInner>,
}

enum BodyInner {
    /// Fully-buffered body. We read the whole response up-front (simpler,
    /// matches the way most Go code uses `http.Get` → `io.ReadAll`).
    Buffered { data: Vec<u8>, pos: usize, closed: bool },
}

impl Body {
    pub(crate) fn from_bytes(data: Vec<u8>) -> Self {
        Body { inner: Mutex::new(BodyInner::Buffered { data, pos: 0, closed: false }) }
    }

    pub(crate) fn empty() -> Self {
        Body::from_bytes(Vec::new())
    }

    /// Close the body. After close, subsequent reads return EOF.
    /// Go: `resp.Body.Close()`.
    #[allow(non_snake_case)]
    pub fn Close(&self) -> error {
        let mut g = self.inner.lock().unwrap();
        match &mut *g {
            BodyInner::Buffered { closed, .. } => { *closed = true; }
        }
        nil
    }

    /// Drain the body and return every remaining byte. Goish-specific
    /// shortcut — Go code typically writes `body, _ := io.ReadAll(resp.Body)`.
    /// `&self` thanks to interior `Mutex`, so `resp` doesn't need `mut`.
    #[allow(non_snake_case)]
    pub fn Bytes(&self) -> crate::types::slice<u8> {
        let mut out = Vec::new();
        self.drain_into(&mut out);
        out
    }

    /// Drain the body and decode as UTF-8 (lossy). Shortcut equivalent to
    /// `string(body)` after an `io.ReadAll` in Go.
    #[allow(non_snake_case)]
    pub fn String(&self) -> string {
        let b = self.Bytes();
        String::from_utf8_lossy(&b).into_owned()
    }

    fn drain_into(&self, out: &mut Vec<u8>) {
        let mut g = self.inner.lock().unwrap();
        match &mut *g {
            BodyInner::Buffered { data, pos, closed } => {
                if *closed { return; }
                out.extend_from_slice(&data[*pos..]);
                *pos = data.len();
            }
        }
    }
}

// Implement std::io::Read so `io::ReadAll(&mut body)` / `io::Copy(&mut dst, &mut body)`
// keep working via goish's blanket Reader impl. std::io::Read requires
// `&mut self`, so reads through the stream interface carry the `mut`;
// the Go-shaped `.Bytes()` / `.String()` shortcuts above don't.
impl std::io::Read for Body {
    fn read(&mut self, p: &mut [u8]) -> std::io::Result<usize> {
        let mut g = self.inner.lock().unwrap();
        match &mut *g {
            BodyInner::Buffered { data, pos, closed } => {
                if *closed { return Ok(0); }
                let remaining = data.len() - *pos;
                if remaining == 0 { return Ok(0); }
                let n = remaining.min(p.len());
                p[..n].copy_from_slice(&data[*pos..*pos + n]);
                *pos += n;
                Ok(n)
            }
        }
    }
}

