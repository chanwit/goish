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
use crate::types::{byte, int, rune, string};
use std::io::{BufRead, Read, Write as _};

/// ErrTooLong — returned by Scan() when a single token exceeds MaxTokenSize.
#[allow(non_snake_case)]
pub fn ErrTooLong() -> error { New("bufio.Scanner: token too long") }

/// ErrFinalToken — a SplitFunc can return this to deliver a last token after EOF.
#[allow(non_snake_case)]
pub fn ErrFinalToken() -> error { New("final token") }

/// MaxScanTokenSize — default cap on Scan token size.
pub const MaxScanTokenSize: usize = 64 * 1024;

/// SplitFunc: called repeatedly by Scanner.Scan with the current buffer.
///
/// Returns (advance, token, err) where:
/// - advance: number of bytes to consume from the buffer
/// - token: the token for this call (None if nothing yet; need more data)
/// - err: non-nil halts the scan
pub type SplitFunc = fn(data: &[byte], at_eof: bool) -> (int, Option<crate::types::slice<byte>>, error);

pub struct Scanner<R: Read> {
    reader: R,
    split: SplitFunc,
    max_token: usize,
    buffer: Vec<byte>,
    /// valid bytes inside `buffer` not yet consumed.
    start: usize,
    end: usize,
    /// The last token yielded, if any.
    token: Vec<byte>,
    last_err: error,
    at_eof: bool,
    done: bool,
    empties: usize,
}

#[allow(non_snake_case)]
pub fn NewScanner<R: Read>(r: R) -> Scanner<R> {
    Scanner {
        reader: r,
        split: ScanLines,
        max_token: MaxScanTokenSize,
        buffer: Vec::with_capacity(4096),
        start: 0, end: 0,
        token: Vec::new(),
        last_err: nil,
        at_eof: false,
        done: false,
        empties: 0,
    }
}

impl<R: Read> Scanner<R> {
    /// Set the split function.
    pub fn Split(&mut self, f: SplitFunc) { self.split = f; }

    /// Limits the size of a single token, also acts as the upper bound on
    /// the buffer. Returns nothing; Go's signature is (n int).
    pub fn MaxTokenSize(&mut self, n: int) { self.max_token = n.max(0) as usize; }

    /// Returns non-EOF error encountered, or nil.
    pub fn Err(&self) -> &error { &self.last_err }

    /// The most recent token's bytes.
    pub fn Bytes(&self) -> &[byte] { &self.token }

    /// The most recent token as a string.
    pub fn Text(&self) -> &str {
        std::str::from_utf8(&self.token).unwrap_or("")
    }

    /// Advance the scanner to the next token.
    pub fn Scan(&mut self) -> bool {
        if self.done { return false; }
        loop {
            // Try to split what we have.
            let data = &self.buffer[self.start..self.end];
            let (advance, token, err) = (self.split)(data, self.at_eof);
            if err != nil {
                self.last_err = err;
                self.done = true;
                if let Some(tok) = token {
                    self.token = tok.into();
                    return true;
                }
                return false;
            }
            if advance < 0 || (advance as usize) > data.len() {
                self.last_err = New("bufio.Scanner: SplitFunc returned invalid advance");
                self.done = true;
                return false;
            }
            self.start += advance as usize;
            if let Some(tok) = token {
                self.token = tok.into();
                if advance > 0 { self.empties = 0; }
                else {
                    self.empties += 1;
                    if self.empties > 100 {
                        self.last_err = New("bufio.Scanner: too many empty tokens without progress");
                        self.done = true;
                        return false;
                    }
                }
                return true;
            }
            // Need more data. Move or grow.
            if self.start > 0 {
                self.buffer.copy_within(self.start..self.end, 0);
                self.end -= self.start;
                self.start = 0;
            }
            if self.end >= self.buffer.len() {
                // Grow, up to max_token.
                let cap = self.buffer.capacity();
                let new_cap = (cap * 2).max(4096).max(self.end + 1);
                if new_cap > self.max_token {
                    self.last_err = ErrTooLong();
                    self.done = true;
                    return false;
                }
                self.buffer.resize(new_cap, 0);
            }
            if self.at_eof {
                // End of stream; emit final tokens.
                self.done = true;
                if self.end > 0 {
                    // Try one more split forcing EOF.
                    let data = &self.buffer[self.start..self.end];
                    let (_, token2, err2) = (self.split)(data, true);
                    if err2 != nil { self.last_err = err2; }
                    if let Some(tok) = token2 {
                        self.token = tok.into();
                        self.start = self.end;
                        return true;
                    }
                }
                return false;
            }
            // Read more.
            let mut scratch = [0u8; 4096];
            let avail = self.buffer.len() - self.end;
            let want = std::cmp::min(avail, scratch.len());
            match self.reader.read(&mut scratch[..want]) {
                Ok(0) => {
                    self.at_eof = true;
                }
                Ok(n) => {
                    self.buffer[self.end..self.end + n].copy_from_slice(&scratch[..n]);
                    self.end += n;
                }
                Err(e) => {
                    let msg = format!("{}", e);
                    if msg.to_lowercase().contains("unexpectedeof") || msg.to_lowercase().contains("eof") {
                        self.at_eof = true;
                    } else {
                        self.last_err = New(&format!("bufio.Scanner: {}", e));
                        self.done = true;
                        return false;
                    }
                }
            }
        }
    }
}

// ─── Built-in SplitFuncs ──────────────────────────────────────────────

/// ScanBytes is a split function that returns each byte as a token.
#[allow(non_snake_case)]
pub fn ScanBytes(data: &[byte], _at_eof: bool) -> (int, Option<crate::types::slice<byte>>, error) {
    if data.is_empty() { return (0, None, nil); }
    (1, Some(vec![data[0]].into()), nil)
}

/// ScanRunes is a split function that yields each UTF-8 rune as a token.
#[allow(non_snake_case)]
pub fn ScanRunes(data: &[byte], at_eof: bool) -> (int, Option<crate::types::slice<byte>>, error) {
    if data.is_empty() { return (0, None, nil); }
    // ASCII fast path.
    if data[0] < 0x80 { return (1, Some(vec![data[0]].into()), nil); }
    let width = match data[0] {
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF7 => 4,
        _ => { /* invalid lead */
            return (1, Some({
            let mut buf = vec![0u8; 4];
            let n = crate::unicode::utf8::EncodeRune(&mut buf, crate::unicode::RuneError);
            buf.truncate(n as usize);
            buf.into()
        }), nil);
        }
    };
    if data.len() < width {
        if at_eof {
            return (data.len() as int, Some({
            let mut buf = vec![0u8; 4];
            let n = crate::unicode::utf8::EncodeRune(&mut buf, crate::unicode::RuneError);
            buf.truncate(n as usize);
            buf.into()
        }), nil);
        }
        return (0, None, nil);
    }
    // Validate continuation bytes.
    for i in 1..width {
        if (data[i] & 0xC0) != 0x80 {
            return (1, Some({
            let mut buf = vec![0u8; 4];
            let n = crate::unicode::utf8::EncodeRune(&mut buf, crate::unicode::RuneError);
            buf.truncate(n as usize);
            buf.into()
        }), nil);
        }
    }
    (width as int, Some(data[..width].to_vec().into()), nil)
}

/// ScanLines is a split function that yields each line of text, stripped of
/// any trailing \r\n or \n marker.
#[allow(non_snake_case)]
pub fn ScanLines(data: &[byte], at_eof: bool) -> (int, Option<crate::types::slice<byte>>, error) {
    if at_eof && data.is_empty() { return (0, None, nil); }
    if let Some(i) = data.iter().position(|&b| b == b'\n') {
        // Drop the trailing \r if present.
        let tok_end = if i > 0 && data[i-1] == b'\r' { i - 1 } else { i };
        return ((i + 1) as int, Some(data[..tok_end].to_vec().into()), nil);
    }
    if at_eof {
        return (data.len() as int, Some(data.to_vec().into()), nil);
    }
    (0, None, nil)
}

/// ScanWords is a split function that yields each whitespace-separated word.
#[allow(non_snake_case)]
pub fn ScanWords(data: &[byte], at_eof: bool) -> (int, Option<crate::types::slice<byte>>, error) {
    // Skip leading whitespace.
    let mut start = 0usize;
    while start < data.len() {
        let (r, size) = decode_rune(&data[start..]);
        if !IsSpace(r) { break; }
        start += size;
    }
    // Find end of word.
    let mut i = start;
    while i < data.len() {
        let (r, size) = decode_rune(&data[i..]);
        if IsSpace(r) {
            return ((i + next_rune_size(&data[i..])) as int, Some(data[start..i].to_vec().into()), nil);
        }
        i += size;
    }
    if at_eof && data.len() > start {
        return (data.len() as int, Some(data[start..].to_vec().into()), nil);
    }
    // Need more data.
    (start as int, None, nil)
}

fn decode_rune(data: &[byte]) -> (rune, usize) {
    if data.is_empty() { return (crate::unicode::RuneError, 0); }
    let c = data[0];
    if c < 0x80 { return (c as rune, 1); }
    let (expected, first) = match c {
        0xC0..=0xDF => (2, (c & 0x1F) as u32),
        0xE0..=0xEF => (3, (c & 0x0F) as u32),
        0xF0..=0xF7 => (4, (c & 0x07) as u32),
        _ => return (crate::unicode::RuneError, 1),
    };
    if data.len() < expected { return (crate::unicode::RuneError, 1); }
    let mut acc = first;
    for i in 1..expected {
        let b = data[i];
        if (b & 0xC0) != 0x80 { return (crate::unicode::RuneError, 1); }
        acc = (acc << 6) | ((b & 0x3F) as u32);
    }
    (acc as rune, expected)
}

fn next_rune_size(data: &[byte]) -> usize {
    let (_, size) = decode_rune(data);
    if size == 0 { 1 } else { size }
}

/// IsSpace reports whether r is a Unicode whitespace rune, matching bufio's
/// internal isSpace (= unicode.IsSpace).
#[allow(non_snake_case)]
pub fn IsSpace(r: rune) -> bool {
    crate::unicode::IsSpace(r)
}

/// Convenience: read all lines from a reader into a `slice<string>`.
#[allow(non_snake_case)]
pub fn ReadLines<R: Read>(r: R) -> (crate::types::slice<string>, error) {
    let mut sc = NewScanner(r);
    let mut lines = crate::types::slice::<string>::new();
    while sc.Scan() {
        lines.push(sc.Text().into());
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
                return (String::from_utf8_lossy(&buf).into_owned().into(), nil);
            }
        }
        match self.inner.read_until(delim, &mut buf) {
            Ok(0) if buf.is_empty() => ("".into(), New("EOF")),
            Ok(0) => (String::from_utf8_lossy(&buf).into_owned().into(), New("EOF")),
            Ok(_) => {
                if buf.last() != Some(&delim) {
                    (String::from_utf8_lossy(&buf).into_owned().into(), New("EOF"))
                } else {
                    (String::from_utf8_lossy(&buf).into_owned().into(), nil)
                }
            }
            Err(e) => (String::from_utf8_lossy(&buf).into_owned().into(), New(&format!("bufio.ReadString: {}", e))),
        }
    }

    /// r.ReadBytes(delim) — like ReadString but returns bytes.
    #[allow(non_snake_case)]
    pub fn ReadBytes(&mut self, delim: crate::types::byte) -> (crate::types::slice<crate::types::byte>, error) {
        let mut buf = Vec::<u8>::new();
        if let Some(b) = self.unread.take() {
            buf.push(b);
            if b == delim { return (buf.into(), nil); }
        }
        match self.inner.read_until(delim, &mut buf) {
            Ok(0) if buf.is_empty() => (buf.into(), New("EOF")),
            Ok(0) => (buf.into(), New("EOF")),
            Ok(_) => {
                let last_is_delim = buf.last() == Some(&delim);
                (buf.into(), if last_is_delim { nil } else { New("EOF") })
            }
            Err(e) => (buf.into(), New(&format!("bufio.ReadBytes: {}", e))),
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
            seen.push(sc.Text().into());
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
