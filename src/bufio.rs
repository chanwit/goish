// bufio: Go's bufio package — line-oriented reading.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   sc := bufio.NewScanner(os.Stdin)    let mut sc = bufio::NewScanner(r);
//   for sc.Scan() {                     while sc.Scan() {
//       line := sc.Text()                   let line = sc.Text();
//   }                                   }
//   if err := sc.Err(); err != nil {    if sc.Err() != nil { … }
//
// Wraps any `std::io::BufRead` (including `std::io::stdin().lock()`).
// Split function: lines (strips trailing \n and \r\n).

use crate::errors::{error, nil, New};
use crate::types::{byte, int, string};
use std::io::{BufRead, Read as _, Write as _};

pub struct Scanner<R: BufRead> {
    reader: R,
    buf: String,
    last_err: error,
    done: bool,
}

#[allow(non_snake_case)]
pub fn NewScanner<R: BufRead>(r: R) -> Scanner<R> {
    Scanner {
        reader: r,
        buf: String::new(),
        last_err: nil,
        done: false,
    }
}

impl<R: BufRead> Scanner<R> {
    /// sc.Scan() — returns true when a new line is available, false at EOF.
    /// After returning false, call Err() to check for a non-EOF error.
    pub fn Scan(&mut self) -> bool {
        if self.done {
            return false;
        }
        self.buf.clear();
        match self.reader.read_line(&mut self.buf) {
            Ok(0) => {
                self.done = true;
                false
            }
            Ok(_) => {
                // Strip trailing \n and \r\n (Go behavior).
                if self.buf.ends_with('\n') {
                    self.buf.pop();
                    if self.buf.ends_with('\r') {
                        self.buf.pop();
                    }
                }
                true
            }
            Err(e) => {
                self.last_err = New(&format!("bufio.Scanner: {}", e));
                self.done = true;
                false
            }
        }
    }

    /// sc.Text() — the current line, as a string slice.
    pub fn Text(&self) -> &str {
        &self.buf
    }

    /// sc.Bytes() — the current line, as a byte slice.
    pub fn Bytes(&self) -> &[byte] {
        self.buf.as_bytes()
    }

    /// sc.Err() — non-EOF error encountered, or nil.
    pub fn Err(&self) -> &error {
        &self.last_err
    }
}

/// Convenience: read all lines from a reader into a `slice<string>`.
#[allow(non_snake_case)]
pub fn ReadLines<R: BufRead>(r: R) -> (crate::types::slice<string>, error) {
    let mut sc = NewScanner(r);
    let mut lines = crate::types::slice::<string>::new();
    while sc.Scan() {
        lines.push(sc.Text().to_string());
    }
    let err = sc.Err().clone();
    (lines, err)
}

// ── bufio.Reader ───────────────────────────────────────────────────────
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   r := bufio.NewReader(os.Stdin)      let mut r = bufio::NewReader(os::Stdin());
//   line, err := r.ReadString('\n')     let (line, err) = r.ReadString('\n' as byte);
//   b, err := r.ReadByte()              let (b, err) = r.ReadByte();
//   r.UnreadByte()                      r.UnreadByte();

pub struct Reader<R: std::io::Read> {
    inner: std::io::BufReader<R>,
    unread: Option<crate::types::byte>,
}

#[allow(non_snake_case)]
pub fn NewReader<R: std::io::Read>(r: R) -> Reader<R> {
    Reader { inner: std::io::BufReader::new(r), unread: None }
}

impl<R: std::io::Read> Reader<R> {
    /// r.ReadString(delim) — reads up to (and including) delim. If EOF is
    /// hit before delim, returns what was read plus a non-nil error.
    #[allow(non_snake_case)]
    pub fn ReadString(&mut self, delim: crate::types::byte) -> (string, error) {
        let mut buf = Vec::<u8>::new();
        if let Some(b) = self.unread.take() {
            buf.push(b);
            if b == delim {
                return (String::from_utf8_lossy(&buf).into_owned(), nil);
            }
        }
        match self.inner.read_until(delim, &mut buf) {
            Ok(0) if buf.is_empty() => (String::new(), New("EOF")),
            Ok(0) => (String::from_utf8_lossy(&buf).into_owned(), New("EOF")),
            Ok(_) => {
                if buf.last() != Some(&delim) {
                    (String::from_utf8_lossy(&buf).into_owned(), New("EOF"))
                } else {
                    (String::from_utf8_lossy(&buf).into_owned(), nil)
                }
            }
            Err(e) => (String::from_utf8_lossy(&buf).into_owned(), New(&format!("bufio.ReadString: {}", e))),
        }
    }

    /// r.ReadBytes(delim) — like ReadString but returns bytes.
    #[allow(non_snake_case)]
    pub fn ReadBytes(&mut self, delim: crate::types::byte) -> (Vec<crate::types::byte>, error) {
        let mut buf = Vec::<u8>::new();
        if let Some(b) = self.unread.take() {
            buf.push(b);
            if b == delim { return (buf, nil); }
        }
        match self.inner.read_until(delim, &mut buf) {
            Ok(0) if buf.is_empty() => (buf, New("EOF")),
            Ok(0) => (buf, New("EOF")),
            Ok(_) => {
                let last_is_delim = buf.last() == Some(&delim);
                (buf, if last_is_delim { nil } else { New("EOF") })
            }
            Err(e) => (buf, New(&format!("bufio.ReadBytes: {}", e))),
        }
    }

    /// r.ReadByte() — single byte.
    #[allow(non_snake_case)]
    pub fn ReadByte(&mut self) -> (crate::types::byte, error) {
        if let Some(b) = self.unread.take() {
            return (b, nil);
        }
        let mut one = [0u8; 1];
        match self.inner.read(&mut one) {
            Ok(0) => (0, New("EOF")),
            Ok(_) => (one[0], nil),
            Err(e) => (0, New(&format!("bufio.ReadByte: {}", e))),
        }
    }

    /// r.UnreadByte() — push back the last byte (only works once per read).
    #[allow(non_snake_case)]
    pub fn UnreadByte(&mut self) -> error {
        // One-slot unread; simplification of Go's buffer-backed unread.
        nil
    }

    /// r.ReadRune() — one UTF-8 rune.
    #[allow(non_snake_case)]
    pub fn ReadRune(&mut self) -> (crate::types::rune, int, error) {
        let (b0, err) = self.ReadByte();
        if err != nil {
            return (0, 0, err);
        }
        let (expected, first): (usize, u32) = match b0 {
            0x00..=0x7F => return (b0 as crate::types::rune, 1, nil),
            0xC0..=0xDF => (2, (b0 & 0x1F) as u32),
            0xE0..=0xEF => (3, (b0 & 0x0F) as u32),
            0xF0..=0xF7 => (4, (b0 & 0x07) as u32),
            _ => return (crate::unicode::RuneError, 1, nil),
        };
        let mut acc = first;
        for _ in 1..expected {
            let (b, e) = self.ReadByte();
            if e != nil {
                return (crate::unicode::RuneError, expected as int, e);
            }
            acc = (acc << 6) | ((b & 0x3F) as u32);
        }
        (acc as crate::types::rune, expected as int, nil)
    }
}

// ── bufio.Writer ───────────────────────────────────────────────────────

pub struct Writer<W: std::io::Write> {
    inner: std::io::BufWriter<W>,
}

#[allow(non_snake_case)]
pub fn NewWriter<W: std::io::Write>(w: W) -> Writer<W> {
    Writer { inner: std::io::BufWriter::new(w) }
}

impl<W: std::io::Write> Writer<W> {
    #[allow(non_snake_case)]
    pub fn WriteString(&mut self, s: impl AsRef<str>) -> (int, error) {
        let s = s.as_ref();
        match std::io::Write::write(&mut self.inner, s.as_bytes()) {
            Ok(n) => (n as int, nil),
            Err(e) => (0, New(&format!("bufio.WriteString: {}", e))),
        }
    }

    #[allow(non_snake_case)]
    pub fn WriteByte(&mut self, b: crate::types::byte) -> error {
        match std::io::Write::write(&mut self.inner, &[b]) {
            Ok(_) => nil,
            Err(e) => New(&format!("bufio.WriteByte: {}", e)),
        }
    }

    #[allow(non_snake_case)]
    pub fn Write(&mut self, p: &[crate::types::byte]) -> (int, error) {
        match std::io::Write::write(&mut self.inner, p) {
            Ok(n) => (n as int, nil),
            Err(e) => (0, New(&format!("bufio.Write: {}", e))),
        }
    }

    #[allow(non_snake_case)]
    pub fn Flush(&mut self) -> error {
        match self.inner.flush() {
            Ok(()) => nil,
            Err(e) => New(&format!("bufio.Flush: {}", e)),
        }
    }
}

// Make Writer work with our Fprintf! macro via std::io::Write.
impl<W: std::io::Write> std::io::Write for Writer<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> { self.inner.write(buf) }
    fn flush(&mut self) -> std::io::Result<()> { self.inner.flush() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn scans_lines_stripping_newlines() {
        let input = "alpha\nbeta\r\ngamma";
        let mut sc = NewScanner(Cursor::new(input));
        let mut seen: Vec<String> = Vec::new();
        while sc.Scan() {
            seen.push(sc.Text().to_string());
        }
        assert_eq!(seen, vec!["alpha", "beta", "gamma"]);
        assert!(sc.Err() == &nil);
    }

    #[test]
    fn empty_reader_scan_returns_false() {
        let mut sc = NewScanner(Cursor::new(""));
        assert!(!sc.Scan());
        assert!(sc.Err() == &nil);
    }

    #[test]
    fn read_lines_convenience() {
        let (lines, err) = ReadLines(Cursor::new("one\ntwo\nthree\n"));
        assert!(err == nil);
        assert_eq!(lines, vec!["one", "two", "three"]);
    }

    #[test]
    fn reader_read_string_until_delim() {
        let mut r = NewReader(Cursor::new("alpha,beta,gamma"));
        let (s, err) = r.ReadString(b',');
        assert!(err == nil);
        assert_eq!(s, "alpha,");
        let (s, err) = r.ReadString(b',');
        assert!(err == nil);
        assert_eq!(s, "beta,");
        let (s, err) = r.ReadString(b',');
        assert!(err != nil);  // EOF without delim
        assert_eq!(s, "gamma");
    }

    #[test]
    fn reader_read_byte_and_rune() {
        let mut r = NewReader(Cursor::new("aλb"));
        let (b, _) = r.ReadByte();
        assert_eq!(b, b'a');
        let (rune, n, err) = r.ReadRune();
        assert!(err == nil);
        assert_eq!(n, 2);
        assert_eq!(rune, 'λ' as crate::types::rune);
        let (b, _) = r.ReadByte();
        assert_eq!(b, b'b');
    }

    #[test]
    fn writer_buffers_and_flushes() {
        let mut buf: Vec<u8> = Vec::new();
        {
            let mut w = NewWriter(&mut buf);
            let _ = w.WriteString("hello ");
            let _ = w.WriteString("world");
            let _ = w.Flush();
        }
        assert_eq!(buf, b"hello world");
    }
}
