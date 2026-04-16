// textproto: Go's net/textproto — line-based protocol primitives
// (CRLF-terminated lines, MIME headers, dot-stuffing).

#![allow(dead_code)]

use crate::errors::{error, nil, New};
use crate::types::string;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};

// ── MIMEHeader ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct MIMEHeader {
    // canonical-key → ordered values
    entries: Vec<(string, Vec<string>)>,
}

impl MIMEHeader {
    pub fn new() -> MIMEHeader { MIMEHeader::default() }

    pub fn Add(&mut self, key: &str, value: &str) {
        let k = CanonicalMIMEHeaderKey(key);
        for (ek, ev) in self.entries.iter_mut() {
            if *ek == k {
                ev.push(value.to_string());
                return;
            }
        }
        self.entries.push((k, vec![value.to_string()]));
    }

    pub fn Set(&mut self, key: &str, value: &str) {
        let k = CanonicalMIMEHeaderKey(key);
        for (ek, ev) in self.entries.iter_mut() {
            if *ek == k {
                *ev = vec![value.to_string()];
                return;
            }
        }
        self.entries.push((k, vec![value.to_string()]));
    }

    pub fn Get(&self, key: &str) -> string {
        let k = CanonicalMIMEHeaderKey(key);
        for (ek, ev) in self.entries.iter() {
            if *ek == k {
                return ev.first().cloned().unwrap_or_default();
            }
        }
        String::new()
    }

    pub fn Values(&self, key: &str) -> Vec<string> {
        let k = CanonicalMIMEHeaderKey(key);
        for (ek, ev) in self.entries.iter() {
            if *ek == k { return ev.clone(); }
        }
        Vec::new()
    }

    pub fn Del(&mut self, key: &str) {
        let k = CanonicalMIMEHeaderKey(key);
        self.entries.retain(|(ek, _)| *ek != k);
    }

    pub fn Len(&self) -> i64 { self.entries.len() as i64 }

    pub fn Keys(&self) -> Vec<string> {
        self.entries.iter().map(|(k, _)| k.clone()).collect()
    }
}

// ── CanonicalMIMEHeaderKey ───────────────────────────────────────────
// Go: "content-type" → "Content-Type". Hyphens split words; first letter
// of each word uppercased, rest lowercased. Non-token characters pass
// through unchanged (Go falls back to raw key for invalid chars).

pub fn CanonicalMIMEHeaderKey(key: &str) -> string {
    if !is_valid_header_key(key) {
        return key.to_string();
    }
    let mut out = String::with_capacity(key.len());
    let mut upper = true;
    for c in key.chars() {
        if upper { out.push(c.to_ascii_uppercase()); }
        else     { out.push(c.to_ascii_lowercase()); }
        upper = c == '-';
    }
    out
}

fn is_valid_header_key(key: &str) -> bool {
    if key.is_empty() { return false; }
    for b in key.bytes() {
        let ok = matches!(b,
            b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*' | b'+'
            | b'-' | b'.' | b'^' | b'_' | b'`' | b'|' | b'~'
            | b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z');
        if !ok { return false; }
    }
    true
}

// ── Reader ───────────────────────────────────────────────────────────

pub struct Reader<R: Read> {
    r: BufReader<R>,
    dot_state: Option<DotState>,
}

enum DotState {
    Start,
    Mid,
    EndCR,
    Done,
}

/// Free function — Go-shape `textproto.NewReader(r)`. Preferred call
/// site; the `Reader::NewReader` method stays for backward compat.
pub fn NewReader<R: Read>(r: R) -> Reader<R> {
    Reader { r: BufReader::new(r), dot_state: None }
}

/// Free function — Go-shape `textproto.NewWriter(w)`.
pub fn NewWriter<W: Write>(w: W) -> Writer<W> {
    Writer { w }
}

impl<R: Read> Reader<R> {
    pub fn NewReader(r: R) -> Reader<R> {
        Reader { r: BufReader::new(r), dot_state: None }
    }

    /// ReadLine reads a single line stripping the trailing \r\n or \n.
    /// Returns empty string + EOF error at the end.
    pub fn ReadLine(&mut self) -> (string, error) {
        let mut buf = String::new();
        match self.r.read_line(&mut buf) {
            Ok(0) => (String::new(), New("EOF")),
            Ok(_) => {
                if buf.ends_with('\n') { buf.pop(); }
                if buf.ends_with('\r') { buf.pop(); }
                (buf, nil)
            }
            Err(e) => (String::new(), New(&e.to_string())),
        }
    }

    /// ReadContinuedLine joins folded lines (continuation lines start with
    /// whitespace). Empty line terminates.
    pub fn ReadContinuedLine(&mut self) -> (string, error) {
        let (first, err) = self.ReadLine();
        if err != nil { return (first, err); }
        if first.is_empty() { return (first, nil); }
        let mut out = first;
        loop {
            // peek next byte
            let buf = match self.r.fill_buf() {
                Ok(b) if b.is_empty() => break,
                Ok(b) => b,
                Err(e) => return (out, New(&e.to_string())),
            };
            if !(buf[0] == b' ' || buf[0] == b'\t') { break; }
            let (cont, err) = self.ReadLine();
            if err != nil { return (out, err); }
            // Trim trailing whitespace on accumulator, leading on cont, join with single space.
            while out.ends_with(' ') || out.ends_with('\t') { out.pop(); }
            out.push(' ');
            out.push_str(cont.trim_start_matches(|c: char| c == ' ' || c == '\t'));
        }
        (out, nil)
    }

    /// ReadMIMEHeader reads a block of MIME headers. Terminated by a blank
    /// line. Returns the parsed header.
    pub fn ReadMIMEHeader(&mut self) -> (MIMEHeader, error) {
        let mut h = MIMEHeader::new();
        loop {
            let (line, err) = self.ReadContinuedLine();
            if err != nil && line.is_empty() {
                if h.Len() == 0 { return (h, err); }
                return (h, err);
            }
            if line.is_empty() { return (h, nil); }
            let colon = match line.find(':') {
                Some(i) => i,
                None => return (h, New(&format!("malformed MIME header line: {}", line))),
            };
            let key = line[..colon].to_string();
            // Go accepts non-compliant keys (spaces before colon) and
            // preserves them verbatim. We only reject genuinely malformed
            // lines (empty key after leading whitespace check).
            if key.is_empty() {
                return (h, New(&format!("malformed MIME header line: {}", line)));
            }
            for b in key.bytes() {
                if b == b'\r' || b == b'\n' {
                    return (h, New(&format!("malformed MIME header line: {}", line)));
                }
            }
            let value = line[colon + 1..].trim_matches(|c: char| c == ' ' || c == '\t').to_string();
            h.Add(&key, &value);
        }
    }

    /// ReadCodeLine reads an SMTP/NNTP/FTP-style response line —
    /// "CODE message\r\n". The expectCode is a prefix match:
    ///   - 0     — accept any code
    ///   - 2     — accept any 2xx
    ///   - 25    — accept any 25x
    ///   - 250   — accept exactly 250
    /// Returns (code, message, err).
    pub fn ReadCodeLine(&mut self, expect: i64) -> (i64, string, error) {
        let (line, err) = self.ReadLine();
        if err != nil { return (0, String::new(), err); }
        if line.len() < 4 {
            return (0, line.clone(), New(&format!("short response: {}", line)));
        }
        let code: i64 = match line[..3].parse() {
            Ok(c) => c,
            Err(_) => return (0, line.clone(), New(&format!("invalid response code: {}", line))),
        };
        let sep = line.as_bytes()[3];
        if sep != b' ' && sep != b'-' {
            return (code, line[4..].to_string(),
                    New(&format!("invalid response separator: {}", line)));
        }
        let msg = line[4..].to_string();
        if expect != 0 {
            let ok = if expect >= 100 {
                code == expect
            } else if expect >= 10 {
                code / 10 == expect
            } else {
                code / 100 == expect
            };
            if !ok {
                return (code, msg.clone(), New(&format!("{} {}", code, msg)));
            }
        }
        (code, msg, nil)
    }

    /// ReadDotLines reads "dot-style" lines (SMTP/NNTP) terminated by a "."
    /// on a line by itself.
    pub fn ReadDotLines(&mut self) -> (Vec<string>, error) {
        let mut lines = Vec::new();
        loop {
            let (line, err) = self.ReadLine();
            if err != nil { return (lines, err); }
            if line == "." { return (lines, nil); }
            let unstuffed = if let Some(r) = line.strip_prefix('.') { r.to_string() } else { line };
            lines.push(unstuffed);
        }
    }

    /// ReadDotBytes is like ReadDotLines but returns one blob with \n
    /// separators and a trailing \n.
    pub fn ReadDotBytes(&mut self) -> (Vec<u8>, error) {
        let (lines, err) = self.ReadDotLines();
        if err != nil { return (Vec::new(), err); }
        let mut out = Vec::new();
        for l in lines {
            out.extend_from_slice(l.as_bytes());
            out.push(b'\n');
        }
        (out, nil)
    }
}

// ── Writer ───────────────────────────────────────────────────────────

pub struct Writer<W: Write> {
    w: W,
}

impl<W: Write> Writer<W> {
    pub fn NewWriter(w: W) -> Writer<W> { Writer { w } }

    pub fn PrintfLine(&mut self, format: &str, args: &[&dyn std::fmt::Display]) -> error {
        let mut s = format.to_string();
        for a in args {
            let needle = "%s";
            if let Some(i) = s.find(needle) {
                s.replace_range(i..i + needle.len(), &a.to_string());
            }
        }
        s.push_str("\r\n");
        match self.w.write_all(s.as_bytes()) {
            Ok(_) => nil,
            Err(e) => New(&e.to_string()),
        }
    }

    pub fn DotWriter(&mut self) -> DotWriter<'_, W> {
        DotWriter { w: &mut self.w, at_line_start: true, closed: false }
    }
}

pub struct DotWriter<'a, W: Write> {
    w: &'a mut W,
    at_line_start: bool,
    closed: bool,
}

impl<'a, W: Write> DotWriter<'a, W> {
    pub fn Write(&mut self, data: &[u8]) -> (i64, error) {
        let mut n = 0;
        for &b in data {
            if self.at_line_start && b == b'.' {
                if let Err(e) = self.w.write_all(b".") {
                    return (n, New(&e.to_string()));
                }
            }
            if let Err(e) = self.w.write_all(&[b]) {
                return (n, New(&e.to_string()));
            }
            n += 1;
            self.at_line_start = b == b'\n';
        }
        (n, nil)
    }

    pub fn Close(&mut self) -> error {
        if self.closed { return nil; }
        self.closed = true;
        let tail: &[u8] = if self.at_line_start { b".\r\n" } else { b"\r\n.\r\n" };
        match self.w.write_all(tail) {
            Ok(_) => nil,
            Err(e) => New(&e.to_string()),
        }
    }
}

impl<'a, W: Write> Drop for DotWriter<'a, W> {
    fn drop(&mut self) { let _ = self.Close(); }
}

// ── Conn (TCP-less convenience) ──────────────────────────────────────
// Go's textproto.Conn wraps Reader+Writer around a net.Conn. We skip the
// Conn wrapper for now — users can construct Reader/Writer directly.

// ── TrimString ───────────────────────────────────────────────────────

pub fn TrimString(s: &str) -> string {
    // Go: trim ASCII space + horizontal tab.
    s.trim_matches(|c: char| c == ' ' || c == '\t').to_string()
}

pub fn TrimBytes(b: &[u8]) -> Vec<u8> {
    let mut start = 0;
    let mut end = b.len();
    while start < end && (b[start] == b' ' || b[start] == b'\t') { start += 1; }
    while end > start && (b[end - 1] == b' ' || b[end - 1] == b'\t') { end -= 1; }
    b[start..end].to_vec()
}

// Quieten lints on feature trimmed types
#[allow(dead_code)]
fn _unused(_: HashMap<string, Vec<string>>) {}
