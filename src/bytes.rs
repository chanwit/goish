// bytes: Go's bytes package — a growable byte buffer.
//
//   Go                                goish
//   ───────────────────────────────   ──────────────────────────────────
//   var buf bytes.Buffer              let mut buf = bytes::Buffer::new();
//   buf.WriteString("hi")             buf.WriteString("hi");
//   buf.WriteByte('!')                buf.WriteByte(b'!');
//   fmt.Fprintf(&buf, "%d", n)        fmt::Fprintf!(&mut buf, "%d", n);
//   s := buf.String()                 let s = buf.String();
//   n := buf.Len()                    let n = buf.Len();
//   buf.Reset()                       buf.Reset();
//
// Buffer implements `io::Write`, so any goish Fprintf! / write! call works
// directly. It also implements Display so `fmt::Println!("buf:", buf)`
// prints the contents (lossy for non-UTF-8 bytes).

use crate::errors::{error, nil};
use crate::types::{byte, int};
use std::io;

#[derive(Debug, Clone, Default)]
pub struct Buffer {
    inner: Vec<u8>,
}

impl Buffer {
    pub fn new() -> Self {
        Buffer::default()
    }

    /// buf.Bytes() — borrowed view of the contents.
    pub fn Bytes(&self) -> &[byte] {
        &self.inner
    }

    /// buf.String() — UTF-8 contents as a string (lossy for invalid bytes).
    pub fn String(&self) -> String {
        String::from_utf8_lossy(&self.inner).into_owned()
    }

    /// buf.Len() — number of bytes currently held.
    pub fn Len(&self) -> int {
        self.inner.len() as int
    }

    /// Lowercase alias for the polymorphic `len!()` macro.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// buf.Cap() — current capacity.
    pub fn Cap(&self) -> int {
        self.inner.capacity() as int
    }

    /// buf.Reset() — discard all contents (capacity preserved).
    pub fn Reset(&mut self) {
        self.inner.clear();
    }

    /// buf.Grow(n) — ensure room for at least n more bytes without reallocating.
    pub fn Grow(&mut self, n: int) {
        if n > 0 {
            self.inner.reserve(n as usize);
        }
    }

    /// buf.Truncate(n) — keep the first n bytes, drop the rest.
    pub fn Truncate(&mut self, n: int) {
        self.inner.truncate(n as usize);
    }

    /// buf.Write(p) — append a byte slice. Always returns nil error.
    pub fn Write(&mut self, p: &[byte]) -> (int, error) {
        self.inner.extend_from_slice(p);
        (p.len() as int, nil)
    }

    /// buf.WriteString(s) — append a string's bytes.
    pub fn WriteString(&mut self, s: &str) -> (int, error) {
        self.inner.extend_from_slice(s.as_bytes());
        (s.len() as int, nil)
    }

    /// buf.WriteByte(b) — append a single byte.
    pub fn WriteByte(&mut self, b: byte) -> error {
        self.inner.push(b);
        nil
    }
}

impl io::Write for Buffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl std::fmt::Display for Buffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(&self.inner))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_string_appends() {
        let mut b = Buffer::new();
        b.WriteString("hello ");
        b.WriteString("world");
        assert_eq!(b.String(), "hello world");
        assert_eq!(b.Len(), 11);
    }

    #[test]
    fn write_byte_appends() {
        let mut b = Buffer::new();
        b.WriteByte(b'X');
        b.WriteByte(b'Y');
        assert_eq!(b.String(), "XY");
    }

    #[test]
    fn reset_clears_contents() {
        let mut b = Buffer::new();
        b.WriteString("data");
        b.Reset();
        assert_eq!(b.Len(), 0);
        assert_eq!(b.String(), "");
    }

    #[test]
    fn truncate_keeps_prefix() {
        let mut b = Buffer::new();
        b.WriteString("hello world");
        b.Truncate(5);
        assert_eq!(b.String(), "hello");
    }

    #[test]
    fn fprintf_to_buffer() {
        let mut b = Buffer::new();
        let _ = crate::Fprintf!(&mut b, "n=%d %s", 42, "ok");
        assert_eq!(b.String(), "n=42 ok");
    }

    #[test]
    fn display_prints_contents() {
        let mut b = Buffer::new();
        b.WriteString("displayed");
        assert_eq!(format!("{}", b), "displayed");
    }
}
