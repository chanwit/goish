// mail: Go's net/mail — parse RFC 5322 email addresses and headers.
//
//   Go                                        goish
//   ───────────────────────────────────────   ────────────────────────────
//   a, err := mail.ParseAddress("a@b.com")    let (a, err) = mail::ParseAddress("a@b.com");
//   list, err := mail.ParseAddressList(s)     let (list, err) = mail::ParseAddressList(s);
//   a.String()                                a.String()
//
// Coverage: addr-spec, display-name + angle-addr, quoted-string display
// name, comma-separated lists, trailing/embedded comments. Does not
// implement date parsing or header full-parse yet.

#![allow(dead_code)]

use crate::errors::{error, nil, New};
use crate::types::string;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Address {
    pub Name: string,
    pub Address: string,
}

/// Go-shape `mail.Address{Name: "…", Address: "…"}` literal.
/// Accepts string literals without `.into()` noise.
#[macro_export]
macro_rules! MailAddress {
    ( $($field:ident : $value:expr),* $(,)? ) => {{
        let mut a = $crate::net::mail::Address::default();
        $( $crate::__mail_addr_set!(a, $field, $value); )*
        a
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __mail_addr_set {
    ($a:ident, Name,    $v:expr) => { $a.Name    = $v.into(); };
    ($a:ident, Address, $v:expr) => { $a.Address = $v.into(); };
}

impl Address {
    pub fn String(&self) -> string {
        // If there's a display name, quote if it has non-atom chars.
        let addr = format!("<{}>", self.Address);
        if self.Name.is_empty() { return addr.trim_matches(|c| c == '<' || c == '>').into(); }
        let needs_quote = self.Name.chars().any(|c| !is_atext(c) && c != ' ');
        let name: std::string::String = if needs_quote {
            let escaped = self.Name.replace('\\', "\\\\").replace('"', "\\\"");
            format!("\"{}\"", escaped)
        } else {
            self.Name.as_str().into()
        };
        format!("{} {}", name, addr).into()
    }
}

fn is_atext(c: char) -> bool {
    matches!(c,
        'a'..='z' | 'A'..='Z' | '0'..='9'
        | '!' | '#' | '$' | '%' | '&' | '\'' | '*' | '+'
        | '-' | '/' | '=' | '?' | '^' | '_' | '`' | '{'
        | '|' | '}' | '~' | '.')
}

pub fn ParseAddress(s: &str) -> (Address, error) {
    let mut p = Parser::new(s);
    match p.parse_address() {
        Ok(a) => {
            p.skip_cfws();
            if !p.eof() {
                return (Address::default(), New(&format!("mail: trailing garbage in {:?}", s)));
            }
            (a, nil)
        }
        Err(e) => (Address::default(), New(&format!("mail: {}", e))),
    }
}

pub fn ParseAddressList(s: &str) -> (Vec<Address>, error) {
    let mut p = Parser::new(s);
    let mut out = Vec::new();
    loop {
        match p.parse_address() {
            Ok(a) => out.push(a),
            Err(e) => return (Vec::new(), New(&format!("mail: {}", e))),
        }
        p.skip_cfws();
        if p.eof() { break; }
        if !p.consume(',') {
            return (Vec::new(), New(&format!("mail: expected comma, got {:?}", p.peek_str())));
        }
    }
    if out.is_empty() {
        return (Vec::new(), New("mail: empty address list"));
    }
    (out, nil)
}

struct Parser<'a> {
    s: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(s: &'a str) -> Parser<'a> { Parser { s, pos: 0 } }

    fn eof(&self) -> bool { self.pos >= self.s.len() }

    fn peek(&self) -> Option<char> { self.s[self.pos..].chars().next() }

    fn peek_str(&self) -> &str { &self.s[self.pos..] }

    fn consume(&mut self, c: char) -> bool {
        if self.peek() == Some(c) {
            self.pos += c.len_utf8();
            true
        } else {
            false
        }
    }

    fn skip_cfws(&mut self) {
        loop {
            while let Some(c) = self.peek() {
                if c == ' ' || c == '\t' || c == '\r' || c == '\n' {
                    self.pos += c.len_utf8();
                } else { break; }
            }
            if self.peek() == Some('(') {
                self.skip_comment();
            } else {
                break;
            }
        }
    }

    fn skip_comment(&mut self) {
        // assumes peek() == '('
        self.pos += 1;
        let mut depth = 1;
        while depth > 0 {
            match self.peek() {
                None => return,
                Some('\\') => {
                    self.pos += 1;
                    if let Some(c) = self.peek() { self.pos += c.len_utf8(); }
                }
                Some('(') => { self.pos += 1; depth += 1; }
                Some(')') => { self.pos += 1; depth -= 1; }
                Some(c) => { self.pos += c.len_utf8(); }
            }
        }
    }

    fn parse_address(&mut self) -> Result<Address, String> {
        self.skip_cfws();
        if self.lookahead_has_angle() {
            let mut name = std::string::String::new();
            loop {
                self.skip_cfws();
                if self.peek() == Some('<') { break; }
                let w = self.parse_word()?;
                if !name.is_empty() && !w.is_empty() { name.push(' '); }
                name.push_str(&w);
            }
            self.consume('<');
            let addr = self.parse_addr_spec()?;
            if !self.consume('>') {
                return Err(format!("expected > got {:?}", self.peek_str()));
            }
            return Ok(Address { Name: name.into(), Address: addr });
        }
        let addr = self.parse_addr_spec()?;
        Ok(Address { Name: "".into(), Address: addr })
    }

    /// Scan ahead for '<' outside quoted strings and comments, stopping at
    /// ',' or end-of-input.
    fn lookahead_has_angle(&self) -> bool {
        let bytes = self.s.as_bytes();
        let mut i = self.pos;
        let mut depth = 0usize;
        let mut in_quote = false;
        while i < bytes.len() {
            let b = bytes[i];
            if in_quote {
                if b == b'\\' { i += 2; continue; }
                if b == b'"'  { in_quote = false; i += 1; continue; }
                i += 1; continue;
            }
            match b {
                b'"' => { in_quote = true; i += 1; }
                b'(' => { depth += 1; i += 1; }
                b')' => { if depth > 0 { depth -= 1; } i += 1; }
                b'<' if depth == 0 => return true,
                b',' if depth == 0 => return false,
                _ => i += 1,
            }
        }
        false
    }

    fn parse_word(&mut self) -> Result<std::string::String, String> {
        self.skip_cfws();
        if self.peek() == Some('"') {
            return self.parse_quoted_string();
        }
        self.parse_atom(true)
    }

    fn parse_atom(&mut self, dot_allowed: bool) -> Result<std::string::String, String> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if is_atext(c) || (dot_allowed && c == '.') {
                self.pos += c.len_utf8();
            } else { break; }
        }
        if self.pos == start {
            return Err(format!("expected atom, got {:?}", self.peek_str()));
        }
        Ok(self.s[start..self.pos].into())
    }

    fn parse_quoted_string(&mut self) -> Result<std::string::String, String> {
        // assumes peek == '"'
        self.pos += 1;
        let mut out = std::string::String::new();
        loop {
            match self.peek() {
                None => return Err("unterminated quoted string".into()),
                Some('"') => { self.pos += 1; return Ok(out); }
                Some('\\') => {
                    self.pos += 1;
                    if let Some(c) = self.peek() {
                        out.push(c);
                        self.pos += c.len_utf8();
                    }
                }
                Some(c) => {
                    out.push(c);
                    self.pos += c.len_utf8();
                }
            }
        }
    }

    fn parse_addr_spec(&mut self) -> Result<string, String> {
        self.skip_cfws();
        let local = if self.peek() == Some('"') {
            self.parse_quoted_string()?
        } else {
            self.parse_atom(true)?
        };
        self.skip_cfws();
        if !self.consume('@') {
            return Err(format!("expected @ got {:?}", self.peek_str()));
        }
        self.skip_cfws();
        let domain = if self.peek() == Some('[') {
            self.pos += 1;
            let start = self.pos;
            while self.peek().is_some() && self.peek() != Some(']') {
                let c = self.peek().unwrap();
                self.pos += c.len_utf8();
            }
            if !self.consume(']') { return Err("unterminated domain literal".into()); }
            format!("[{}]", &self.s[start..self.pos - 1])
        } else {
            self.parse_atom(true)?
        };
        Ok(format!("{}@{}", local, domain).into())
    }
}
