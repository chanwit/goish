// multipart: Go's mime/multipart — Reader/Writer + Part.
//
//   Go                                   goish
//   ──────────────────────────────────   ────────────────────────────────
//   w := multipart.NewWriter(&buf)       let mut w = multipart::Writer::new(&mut buf);
//   part, _ := w.CreateFormFile(..)       let (part, _) = w.CreateFormFile(..);
//   part.Write(data)                     part.Write(data);
//   w.WriteField("k","v")                w.WriteField("k","v");
//   w.Close()                             w.Close();
//
//   r := multipart.NewReader(body, bnd)  let mut r = multipart::Reader::new(body, bnd);
//   part, err := r.NextPart()             let (part, err) = r.NextPart();
//   slurp, _ := io.ReadAll(part)          let (slurp, _) = io::ReadAll(&mut part);

#![allow(dead_code)]

use crate::errors::{error, nil, New};
use crate::net::textproto::MIMEHeader;
use crate::types::string;
use std::io::{Read, Write};

// ── Writer ──────────────────────────────────────────────────────────

pub struct Writer<W: Write> {
    w: W,
    boundary: string,
    last_part_open: bool,
    wrote_first_boundary: bool,
}

/// Free function — Go-shape `multipart.NewWriter(w)`.
pub fn NewWriter<W: Write>(w: W) -> Writer<W> {
    Writer { w, boundary: random_boundary(), last_part_open: false, wrote_first_boundary: false }
}

/// Free function — Go-shape `multipart.NewReader(r, boundary)`.
pub fn NewReader<R: Read>(r: R, boundary: &str) -> Reader {
    Reader::NewReader(r, boundary)
}

impl<W: Write> Writer<W> {
    pub fn NewWriter(w: W) -> Writer<W> {
        Writer { w, boundary: random_boundary(), last_part_open: false, wrote_first_boundary: false }
    }

    pub fn Boundary(&self) -> string { self.boundary.clone() }

    pub fn SetBoundary(&mut self, boundary: &str) -> error {
        if !valid_boundary(boundary) {
            return New(&format!("mime: invalid boundary character"));
        }
        self.boundary = boundary.into();
        nil
    }

    pub fn FormDataContentType(&self) -> string {
        format!("multipart/form-data; boundary={}", self.boundary).into()
    }

    pub fn CreatePart(&mut self, header: MIMEHeader) -> (Part<'_, W>, error) {
        if self.last_part_open {
            // close previous part implicitly — but we'll just write the boundary.
        }
        let sep: &[u8] = if !self.wrote_first_boundary { b"" } else { b"\r\n" };
        if let Err(e) = self.w.write_all(sep) {
            return (Part::dummy(), New(&e.to_string()));
        }
        if let Err(e) = write!(self.w, "--{}\r\n", self.boundary) {
            return (Part::dummy(), New(&e.to_string()));
        }
        self.wrote_first_boundary = true;
        // Emit headers
        for k in header.Keys() {
            for v in header.Values(&k) {
                if let Err(e) = write!(self.w, "{}: {}\r\n", k, v) {
                    return (Part::dummy(), New(&e.to_string()));
                }
            }
        }
        if let Err(e) = self.w.write_all(b"\r\n") {
            return (Part::dummy(), New(&e.to_string()));
        }
        self.last_part_open = true;
        (Part::new(&mut self.w), nil)
    }

    pub fn CreateFormFile(&mut self, fieldname: &str, filename: &str) -> (Part<'_, W>, error) {
        let mut h = MIMEHeader::new();
        h.Set("Content-Disposition",
              &format!("form-data; name=\"{}\"; filename=\"{}\"",
                       escape_quotes(fieldname), escape_quotes(filename)));
        h.Set("Content-Type", "application/octet-stream");
        self.CreatePart(h)
    }

    pub fn CreateFormField(&mut self, fieldname: &str) -> (Part<'_, W>, error) {
        let mut h = MIMEHeader::new();
        h.Set("Content-Disposition",
              &format!("form-data; name=\"{}\"", escape_quotes(fieldname)));
        self.CreatePart(h)
    }

    pub fn WriteField(&mut self, fieldname: &str, value: &str) -> error {
        let (mut p, err) = self.CreateFormField(fieldname);
        if err != nil { return err; }
        let (_, e) = p.Write(value.as_bytes());
        e
    }

    pub fn Close(&mut self) -> error {
        if !self.wrote_first_boundary {
            // Empty writer: still emit the closing boundary for a valid body.
            if let Err(e) = write!(self.w, "--{}--\r\n", self.boundary) {
                return New(&e.to_string());
            }
            return nil;
        }
        if let Err(e) = write!(self.w, "\r\n--{}--\r\n", self.boundary) {
            return New(&e.to_string());
        }
        nil
    }
}

fn escape_quotes(s: &str) -> std::string::String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn valid_boundary(s: &str) -> bool {
    if s.is_empty() || s.len() > 70 { return false; }
    for b in s.bytes() {
        let ok = matches!(b,
            b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z'
            | b'\''..=b')' | b'+' | b'_' | b',' | b'-' | b'.' | b'/' | b':' | b'=' | b'?');
        let is_space = b == b' ';
        if !ok && !is_space { return false; }
    }
    // Last char must not be space
    s.as_bytes().last().map(|&c| c != b' ').unwrap_or(false)
}

fn random_boundary() -> string {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    format!("{:016x}{:016x}goishboundary", t ^ 0x9E3779B97F4A7C15_u64, n).into()
}

// ── Part (writer side) ──────────────────────────────────────────────

pub struct Part<'a, W: Write> {
    w: Option<&'a mut W>,
}

impl<'a, W: Write> Part<'a, W> {
    fn new(w: &'a mut W) -> Part<'a, W> { Part { w: Some(w) } }
    fn dummy() -> Part<'a, W> { Part { w: None } }
    pub fn Write(&mut self, data: &[u8]) -> (i64, error) {
        match &mut self.w {
            Some(w) => match w.write_all(data) {
                Ok(_) => (data.len() as i64, nil),
                Err(e) => (0, New(&e.to_string())),
            },
            None => (0, New("mime: write on closed part")),
        }
    }
}

// ── Reader ──────────────────────────────────────────────────────────

pub struct Reader {
    buf: Vec<u8>,
    pos: usize,
    boundary: string,
    eof: bool,
    first_part: bool,
}

impl Reader {
    pub fn NewReader<R: Read>(mut r: R, boundary: &str) -> Reader {
        let mut buf = Vec::new();
        let _ = r.read_to_end(&mut buf);
        Reader { buf, pos: 0, boundary: boundary.into(), eof: false, first_part: true }
    }

    pub fn NextPart(&mut self) -> (ReaderPart, error) {
        if self.eof {
            return (ReaderPart::default(), New("EOF"));
        }
        // Look for next boundary.
        let sep = if self.first_part { format!("--{}", self.boundary) }
                  else                { format!("\r\n--{}", self.boundary) };
        let sep_b = sep.as_bytes();
        let start = match find(&self.buf[self.pos..], sep_b) {
            Some(i) => self.pos + i + sep_b.len(),
            None => {
                self.eof = true;
                return (ReaderPart::default(), New("EOF"));
            }
        };
        self.first_part = false;
        // Two bytes after boundary determine: "\r\n" (continue) or "--" (terminator).
        if start + 2 > self.buf.len() {
            self.eof = true;
            return (ReaderPart::default(), New("EOF"));
        }
        if &self.buf[start..start + 2] == b"--" {
            self.eof = true;
            return (ReaderPart::default(), New("EOF"));
        }
        if &self.buf[start..start + 2] != b"\r\n" {
            self.eof = true;
            return (ReaderPart::default(), New("mime: malformed boundary (no CRLF)"));
        }
        // Parse headers until blank line.
        let mut p = start + 2;
        let mut header = MIMEHeader::new();
        loop {
            let line_end = match find(&self.buf[p..], b"\r\n") {
                Some(i) => p + i,
                None => return (ReaderPart::default(), New("mime: malformed headers")),
            };
            if line_end == p { p += 2; break; } // blank line
            let line = &self.buf[p..line_end];
            if let Some(colon) = line.iter().position(|&b| b == b':') {
                let k = std::str::from_utf8(&line[..colon]).unwrap_or("");
                let v = std::str::from_utf8(&line[colon + 1..]).unwrap_or("").trim();
                header.Add(k, v);
            }
            p = line_end + 2;
        }
        // Body ends at next "\r\n--<boundary>".
        let end_sep = format!("\r\n--{}", self.boundary);
        let end = match find(&self.buf[p..], end_sep.as_bytes()) {
            Some(i) => p + i,
            None => {
                self.eof = true;
                return (ReaderPart::default(), New("mime: unterminated part"));
            }
        };
        let body = self.buf[p..end].to_vec();
        self.pos = end;
        (ReaderPart { Header: header, body, read: 0 }, nil)
    }
}

fn is_form_data(cd: &str) -> bool {
    // Check `disposition-type` token, case-insensitive.
    let bytes = cd.trim_start().as_bytes();
    let needle = b"form-data";
    if bytes.len() < needle.len() { return false; }
    let prefix = &bytes[..needle.len()];
    if !prefix.iter().zip(needle.iter()).all(|(a, b)| a.eq_ignore_ascii_case(b)) {
        return false;
    }
    // Next char must be ';' or whitespace or end.
    match bytes.get(needle.len()) {
        None => true,
        Some(c) => *c == b';' || *c == b' ' || *c == b'\t',
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() { return Some(0); }
    if haystack.len() < needle.len() { return None; }
    for i in 0..=(haystack.len() - needle.len()) {
        if &haystack[i..i + needle.len()] == needle { return Some(i); }
    }
    None
}

#[derive(Default)]
pub struct ReaderPart {
    pub Header: MIMEHeader,
    body: Vec<u8>,
    read: usize,
}

impl ReaderPart {
    /// Construct a ReaderPart from just its headers — useful in tests
    /// that want to exercise FormName/FileName without a full round-trip.
    pub fn new_for_header(h: MIMEHeader) -> ReaderPart {
        ReaderPart { Header: h, body: Vec::new(), read: 0 }
    }

    pub fn FormName(&self) -> string {
        let cd = self.Header.Get("Content-Disposition");
        if !is_form_data(&cd) { return "".into(); }
        parse_param(&cd, "name").map(string::from).unwrap_or_default()
    }

    pub fn FileName(&self) -> string {
        let cd = self.Header.Get("Content-Disposition");
        parse_param(&cd, "filename").map(string::from).unwrap_or_default()
    }

    pub fn Body(&self) -> &[u8] { &self.body }
}

impl Read for ReaderPart {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = (self.body.len() - self.read).min(buf.len());
        if n == 0 { return Ok(0); }
        buf[..n].copy_from_slice(&self.body[self.read..self.read + n]);
        self.read += n;
        Ok(n)
    }
}

fn parse_param(cd: &str, name: &str) -> Option<string> {
    // Split on ';' and find a parameter whose key (trimmed, lowercased)
    // matches `name`. Handles quoted values and case-insensitive keys.
    // Skips the first segment (disposition type).
    let name_lower = name.to_ascii_lowercase();
    let segments: Vec<&str> = cd.split(';').skip(1).collect();
    for seg in segments {
        let s = seg.trim_matches(|c: char| c == ' ' || c == '\t');
        let eq = match s.find('=') {
            Some(i) => i,
            None => continue,
        };
        let k = s[..eq].trim().to_ascii_lowercase();
        if k != name_lower { continue; }
        let v = s[eq + 1..].trim();
        if let Some(stripped) = v.strip_prefix('"') {
            if let Some(end) = stripped.find('"') {
                return Some(stripped[..end].into());
            }
        }
        return Some(v.into());
    }
    None
}
