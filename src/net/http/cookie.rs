// cookie: Go's net/http Cookie, ParseCookie, ParseSetCookie, String().
// Follows RFC 6265 serialisation. Subset — leaves partitioning + SameSite
// full round-trip for a follow-up if needed.

#![allow(dead_code)]

use crate::errors::{error, nil, New};
use crate::types::string;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SameSite {
    Default,
    Lax,
    Strict,
    None,
}

impl Default for SameSite {
    fn default() -> Self { SameSite::Default }
}

pub const SameSiteDefaultMode: SameSite = SameSite::Default;
pub const SameSiteLaxMode:     SameSite = SameSite::Lax;
pub const SameSiteStrictMode:  SameSite = SameSite::Strict;
pub const SameSiteNoneMode:    SameSite = SameSite::None;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cookie {
    pub Name: string,
    pub Value: string,
    pub Quoted: bool,
    pub Path: string,
    pub Domain: string,
    pub Expires: string,     // RFC1123 serialization; empty = unset
    pub RawExpires: string,
    pub MaxAge: i64,
    pub Secure: bool,
    pub HttpOnly: bool,
    pub SameSite: SameSite,
    pub Partitioned: bool,
    pub Raw: string,
    pub Unparsed: Vec<string>,
}

/// Go-shape Cookie literal.
///
/// ```ignore
/// let c = Cookie!{Name: "foo", Value: "bar"};
/// let c = Cookie!{Name: "foo", Value: "bar", HttpOnly: true};
/// ```
///
/// Mirrors Go's `&http.Cookie{Name: "foo", Value: "bar"}` — accepts
/// string literals without `.into()` / `.into()` noise.
#[macro_export]
macro_rules! Cookie {
    ( $($field:ident : $value:expr),* $(,)? ) => {{
        let mut c = $crate::net::http::Cookie::default();
        $( $crate::__cookie_set!(c, $field, $value); )*
        c
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __cookie_set {
    ($c:ident, Name,        $v:expr) => { $c.Name        = $v.into(); };
    ($c:ident, Value,       $v:expr) => { $c.Value       = $v.into(); };
    ($c:ident, Path,        $v:expr) => { $c.Path        = $v.into(); };
    ($c:ident, Domain,      $v:expr) => { $c.Domain      = $v.into(); };
    ($c:ident, Expires,     $v:expr) => { $c.Expires     = $v.into(); };
    ($c:ident, RawExpires,  $v:expr) => { $c.RawExpires  = $v.into(); };
    ($c:ident, Raw,         $v:expr) => { $c.Raw         = $v.into(); };
    ($c:ident, Quoted,      $v:expr) => { $c.Quoted      = $v; };
    ($c:ident, MaxAge,      $v:expr) => { $c.MaxAge      = $v; };
    ($c:ident, Secure,      $v:expr) => { $c.Secure      = $v; };
    ($c:ident, HttpOnly,    $v:expr) => { $c.HttpOnly    = $v; };
    ($c:ident, SameSite,    $v:expr) => { $c.SameSite    = $v; };
    ($c:ident, Partitioned, $v:expr) => { $c.Partitioned = $v; };
}

impl Cookie {
    pub fn String(&self) -> string {
        if !is_token(&self.Name) { return "".into(); }
        let mut b = std::string::String::new();
        b.push_str(&self.Name);
        b.push('=');
        b.push_str(&sanitize_cookie_value(&self.Value, self.Quoted));
        if !self.Path.is_empty() {
            b.push_str("; Path=");
            b.push_str(&sanitize_cookie_path(&self.Path));
        }
        if !self.Domain.is_empty() && valid_cookie_domain(&self.Domain) {
            let d = if self.Domain.starts_with('.') { &self.Domain[1..] } else { &self.Domain[..] };
            b.push_str("; Domain=");
            b.push_str(d);
        }
        if !self.Expires.is_empty() {
            b.push_str("; Expires=");
            b.push_str(&self.Expires);
        }
        if self.MaxAge > 0 {
            b.push_str(&format!("; Max-Age={}", self.MaxAge));
        } else if self.MaxAge < 0 {
            b.push_str("; Max-Age=0");
        }
        if self.HttpOnly { b.push_str("; HttpOnly"); }
        if self.Secure   { b.push_str("; Secure"); }
        match self.SameSite {
            SameSite::Lax    => b.push_str("; SameSite=Lax"),
            SameSite::Strict => b.push_str("; SameSite=Strict"),
            SameSite::None   => b.push_str("; SameSite=None"),
            SameSite::Default => {}
        }
        if self.Partitioned { b.push_str("; Partitioned"); }
        b.into()
    }
}

// ── ParseCookie (Cookie request header) ─────────────────────────────

pub fn ParseCookie(line: &str) -> (crate::types::slice<Cookie>, error) {
    let parts: Vec<&str> = trim_string(line).split(';').collect();
    if parts.len() == 1 && parts[0].is_empty() {
        return (crate::types::slice::new(), New("http: blank cookie"));
    }
    let mut out: Vec<Cookie> = Vec::with_capacity(parts.len());
    for s in &parts {
        let s = trim_string(s);
        let (name, value) = match s.find('=') {
            Some(i) => (&s[..i], &s[i + 1..]),
            None => return (crate::types::slice::new(), New("http: '=' not found in cookie")),
        };
        if !is_token(name) {
            return (crate::types::slice::new(), New("http: invalid cookie name"));
        }
        let (val, quoted, ok) = parse_cookie_value(value, true);
        if !ok {
            return (crate::types::slice::new(), New("http: invalid cookie value"));
        }
        out.push(Cookie { Name: name.into(), Value: val, Quoted: quoted, ..Cookie::default() });
    }
    (out.into(), nil)
}

// ── ParseSetCookie (Set-Cookie response header) ────────────────────

pub fn ParseSetCookie(line: &str) -> (Cookie, error) {
    let parts: Vec<&str> = trim_string(line).split(';').collect();
    if parts.len() == 1 && parts[0].is_empty() {
        return (Cookie::default(), New("http: blank cookie"));
    }
    let first = trim_string(parts[0]);
    let eq = match first.find('=') {
        Some(i) => i,
        None => return (Cookie::default(), New("http: '=' not found in cookie")),
    };
    let name: string = trim_string(&first[..eq]).into();
    let value = &first[eq + 1..];
    if !is_token(&name) {
        return (Cookie::default(), New("http: invalid cookie name"));
    }
    let (val, quoted, ok) = parse_cookie_value(value, true);
    if !ok {
        return (Cookie::default(), New("http: invalid cookie value"));
    }
    let mut c = Cookie {
        Name: name,
        Value: val,
        Quoted: quoted,
        Raw: line.into(),
        ..Cookie::default()
    };

    for i in 1..parts.len() {
        let part = trim_string(parts[i]);
        if part.is_empty() { continue; }
        let (attr, val) = match part.find('=') {
            Some(j) => (&part[..j], &part[j + 1..]),
            None => (&part[..], ""),
        };
        let lower_attr = attr.to_ascii_lowercase();
        let (val, _, ok) = parse_cookie_value(val, false);
        if !ok {
            c.Unparsed.push(part.into());
            continue;
        }
        match lower_attr.as_str() {
            "samesite" => {
                match val.to_ascii_lowercase().as_str() {
                    "lax"    => c.SameSite = SameSite::Lax,
                    "strict" => c.SameSite = SameSite::Strict,
                    "none"   => c.SameSite = SameSite::None,
                    _        => c.SameSite = SameSite::Default,
                }
            }
            "secure"   => c.Secure   = true,
            "httponly" => c.HttpOnly = true,
            "domain"   => c.Domain   = val,
            "max-age"  => {
                match val.parse::<i64>() {
                    Ok(n) if n > 0 => c.MaxAge = n,
                    Ok(_)          => c.MaxAge = -1,
                    Err(_)         => c.Unparsed.push(part.into()),
                }
            }
            "expires"  => { c.RawExpires = val.clone(); c.Expires = val; }
            "path"     => c.Path = val,
            "partitioned" => c.Partitioned = true,
            _          => c.Unparsed.push(part.into()),
        }
    }
    (c, nil)
}

// ── helpers ─────────────────────────────────────────────────────────

fn is_token(s: &str) -> bool {
    if s.is_empty() { return false; }
    for b in s.bytes() {
        let ok = matches!(b,
            b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*' | b'+'
            | b'-' | b'.' | b'^' | b'_' | b'`' | b'|' | b'~'
            | b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z');
        if !ok { return false; }
    }
    true
}

fn trim_string(s: &str) -> &str {
    s.trim_matches(|c: char| c == ' ' || c == '\t')
}

/// (value, quoted, ok)
fn parse_cookie_value(raw: &str, allow_double_quote: bool) -> (string, bool, bool) {
    let raw = trim_string(raw);
    let mut quoted = false;
    let inner = if allow_double_quote && raw.len() >= 2
        && raw.starts_with('"') && raw.ends_with('"') {
        quoted = true;
        &raw[1..raw.len() - 1]
    } else {
        raw
    };
    for b in inner.bytes() {
        if quoted {
            // Inside quotes, spaces and commas are permitted.
            if !(valid_cookie_value_byte(b) || b == b' ' || b == b',') {
                return ("".into(), false, false);
            }
        } else if !valid_cookie_value_byte(b) {
            return ("".into(), false, false);
        }
    }
    (inner.into(), quoted, true)
}

fn valid_cookie_value_byte(b: u8) -> bool {
    // RFC 6265 cookie-octet: 0x21, 0x23-2B, 0x2D-3A, 0x3C-5B, 0x5D-7E.
    // Go's sanitiser accepts a little wider to quote-wrap; for parse we
    // accept all printable ASCII except control / DEL / comma / semicolon.
    0x20 < b && b < 0x7f && b != b'"' && b != b';' && b != b'\\'
}

fn valid_cookie_domain(v: &str) -> bool {
    if v.is_empty() { return false; }
    if v.len() > 255 { return false; }
    for c in v.chars() {
        if !(c.is_ascii_alphanumeric() || c == '.' || c == '-') {
            return false;
        }
    }
    // Pure IPv6 (contains ':') is excluded; pure IPv4 passes.
    if v.contains(':') { return false; }
    // Each DNS label: non-empty, cannot start or end with '-'.
    let d = if v.starts_with('.') { &v[1..] } else { v };
    for label in d.split('.') {
        if label.is_empty() { return false; }
        let bs = label.as_bytes();
        if bs[0] == b'-' || bs[bs.len() - 1] == b'-' { return false; }
    }
    // Reject pure-digit TLDs (but keep IPv4 — all-numeric labels + 4 parts).
    // Go accepts "127.0.0.1" as valid Domain; so we don't reject numeric.
    true
}

fn sanitize_cookie_value(v: &str, quoted: bool) -> std::string::String {
    let mut out = std::string::String::with_capacity(v.len());
    for b in v.bytes() {
        if valid_cookie_value_byte(b) || b == b' ' || b == b',' { out.push(b as char); }
    }
    if quoted || out.contains(' ') || out.contains(',') {
        format!("\"{}\"", out)
    } else {
        out
    }
}

fn sanitize_cookie_path(v: &str) -> std::string::String {
    let mut out = std::string::String::with_capacity(v.len());
    for b in v.bytes() {
        if (0x20 < b && b < 0x7f) && b != b';' { out.push(b as char); }
    }
    out
}

