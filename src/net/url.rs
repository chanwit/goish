// url: Go's net/url package — parse and manipulate URLs.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   u, err := url.Parse(s)              let (u, err) = url::Parse(s);
//   u.Scheme                            u.Scheme
//   u.Host                              u.Host
//   u.Path                              u.Path
//   u.Query()                           u.Query()
//   v := url.Values{}                   let mut v = url::Values::new();
//   v.Set("k", "v")                     v.Set("k", "v");
//   v.Encode()                          v.Encode();
//   url.QueryEscape(s)                  url::QueryEscape(s)
//   url.PathEscape(s)                   url::PathEscape(s)
//
// Implements the common subset. For the full WHATWG URL algorithm, reach
// for a dedicated crate; Go's net/url and this port are both "best effort
// for typical URLs".

use crate::errors::{error, nil, New};
use crate::types::{int, map, slice, string};
use std::collections::HashMap;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct URL {
    pub Scheme: string,
    pub Opaque: string,
    pub User: Option<Userinfo>,
    pub Host: string,
    pub Path: string,
    pub RawPath: string,
    pub RawQuery: string,
    pub Fragment: string,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Userinfo {
    pub username: string,
    pub password: Option<string>,
}

impl Userinfo {
    pub fn Username(&self) -> string { self.username.clone() }
    pub fn Password(&self) -> (string, bool) {
        match &self.password {
            Some(p) => (p.clone(), true),
            None => ("".into(), false),
        }
    }
    pub fn String(&self) -> string {
        match &self.password {
            Some(p) => format!("{}:{}", QueryEscape(&self.username), QueryEscape(p)).into(),
            None => QueryEscape(&self.username),
        }
    }
}

impl URL {
    pub fn String(&self) -> string {
        let mut out = std::string::String::new();
        if !self.Scheme.is_empty() {
            out.push_str(&self.Scheme);
            out.push(':');
        }
        if !self.Opaque.is_empty() {
            out.push_str(&self.Opaque);
        } else {
            if !self.Host.is_empty() || self.User.is_some()
                || &*self.Scheme == "http" || &*self.Scheme == "https"
                || &*self.Scheme == "ws" || &*self.Scheme == "wss" || &*self.Scheme == "ftp"
            {
                out.push_str("//");
                if let Some(u) = &self.User {
                    out.push_str(&u.String());
                    out.push('@');
                }
                out.push_str(&self.Host);
            }
            out.push_str(&self.Path);
        }
        if !self.RawQuery.is_empty() {
            out.push('?');
            out.push_str(&self.RawQuery);
        }
        if !self.Fragment.is_empty() {
            out.push('#');
            out.push_str(&self.Fragment);
        }
        out.into()
    }

    pub fn Query(&self) -> Values {
        ParseQuery(&self.RawQuery).0
    }

    pub fn IsAbs(&self) -> bool { !self.Scheme.is_empty() }

    pub fn Hostname(&self) -> string {
        let h = &self.Host;
        if h.starts_with('[') {
            if let Some(end) = h.find(']') { return h[1..end].into(); }
        }
        match h.rsplit_once(':') {
            Some((host, _)) => host.into(),
            None => h.clone(),
        }
    }

    /// URL.RequestURI — the path?query portion, suitable for an HTTP request.
    pub fn RequestURI(&self) -> string {
        let mut out = std::string::String::new();
        if !self.Opaque.is_empty() {
            out.push_str(&self.Opaque);
        } else {
            if self.Path.is_empty() {
                out.push('/');
            } else {
                out.push_str(&self.Path);
            }
        }
        if !self.RawQuery.is_empty() {
            out.push('?');
            out.push_str(&self.RawQuery);
        }
        out.into()
    }

    /// URL.JoinPath — returns a new URL with the given path components appended.
    pub fn JoinPath(&self, elem: &[impl AsRef<str>]) -> URL {
        let mut joined = String::from(&*self.Path);
        for e in elem {
            let s = e.as_ref();
            if s.is_empty() { continue; }
            if !joined.ends_with('/') && !s.starts_with('/') { joined.push('/'); }
            joined.push_str(s);
        }
        // Normalise ./ and ../ like path.Clean would do for a pure path.
        let cleaned = path_clean(&joined);
        let mut u = self.clone();
        u.Path = cleaned.into();
        u.RawPath = "".into();
        u
    }

    pub fn Port(&self) -> string {
        let h = &self.Host;
        if h.starts_with('[') {
            if let Some(end) = h.find(']') {
                if h.as_str().len() > end + 1 && &h[end + 1..end + 2] == ":" {
                    return h[end + 2..].into();
                }
                return "".into();
            }
        }
        match h.rsplit_once(':') {
            Some((_, p)) => p.into(),
            None => "".into(),
        }
    }
}

#[allow(non_snake_case)]
pub fn Parse(raw: impl AsRef<str>) -> (URL, error) {
    let s = raw.as_ref();
    let mut u = URL::default();
    let mut rest = s;

    // Fragment.
    if let Some(i) = rest.find('#') {
        u.Fragment = rest[i + 1..].into();
        rest = &rest[..i];
    }

    // Query.
    if let Some(i) = rest.find('?') {
        u.RawQuery = rest[i + 1..].into();
        rest = &rest[..i];
    }

    // Scheme.
    if let Some(i) = find_scheme_end(rest) {
        u.Scheme = rest[..i].to_ascii_lowercase().into();
        rest = &rest[i + 1..];
    }

    // Authority (//host...)?
    if rest.starts_with("//") {
        rest = &rest[2..];
        let authority_end = rest.find('/').unwrap_or(rest.len());
        let authority = &rest[..authority_end];
        rest = &rest[authority_end..];
        let (userinfo_opt, host) = match authority.rfind('@') {
            Some(i) => (Some(&authority[..i]), &authority[i + 1..]),
            None => (None, authority),
        };
        if let Some(ui) = userinfo_opt {
            let (user, pwd) = match ui.find(':') {
                Some(i) => {
                    let (u, p) = ui.split_at(i);
                    (QueryUnescape(u).0, Some(QueryUnescape(&p[1..]).0))
                }
                None => (QueryUnescape(ui).0, None),
            };
            u.User = Some(Userinfo { username: user, password: pwd });
        }
        u.Host = host.into();
    } else if !u.Scheme.is_empty() && !rest.starts_with('/') {
        // Opaque URL: e.g. "mailto:foo@bar".
        u.Opaque = rest.into();
        return (u, nil);
    }

    let (decoded, err) = PathUnescape(rest);
    if err != nil {
        return (u, New(&format!("parse {:?}: invalid URL escape", s)));
    }
    // Go's net/url invariant: RawPath is set only when the escaped form
    // differs from the default encoding of Path. If PathEscape(decoded)
    // round-trips to the original bytes, there were no percent-escapes
    // to preserve and RawPath stays empty — this is what keeps
    // `reflect.DeepEqual`-style URL equality working on parsed URLs.
    let reencoded = PathEscape(&*decoded);
    u.RawPath = if &*reencoded == rest { "".into() } else { rest.into() };
    u.Path = decoded;
    (u, nil)
}

fn find_scheme_end(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_alphabetic() { return None; }
    for (i, &b) in bytes.iter().enumerate() {
        if b == b':' { return Some(i); }
        if i == 0 {
            if !b.is_ascii_alphabetic() { return None; }
        } else if !(b.is_ascii_alphanumeric() || b == b'+' || b == b'-' || b == b'.') {
            return None;
        }
    }
    None
}

// ── Values (query string) ─────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct Values {
    inner: map<string, slice<string>>,
}

impl Values {
    pub fn new() -> Self { Values { inner: map::new() } }

    pub fn Get(&self, key: impl AsRef<str>) -> string {
        self.inner.get(key.as_ref())
            .and_then(|v| v.first())
            .cloned()
            .unwrap_or_default()
    }

    pub fn Set(&mut self, key: impl AsRef<str>, value: impl AsRef<str>) {
        self.inner.insert(key.as_ref().into(), slice(vec![value.as_ref().into()]));
    }

    pub fn Add(&mut self, key: impl AsRef<str>, value: impl AsRef<str>) {
        self.inner.entry(key.as_ref().into()).or_default()
            .push(value.as_ref().into());
    }

    pub fn Del(&mut self, key: impl AsRef<str>) {
        self.inner.remove(key.as_ref());
    }

    pub fn Has(&self, key: impl AsRef<str>) -> bool {
        self.inner.contains_key(key.as_ref())
    }

    pub fn Encode(&self) -> string {
        let mut keys: Vec<&string> = self.inner.keys().collect();
        keys.sort();
        let mut out = std::string::String::new();
        for k in keys {
            let enc_k = QueryEscape(&**k);
            if let Some(vs) = self.inner.get(k) {
                for v in vs {
                    if !out.is_empty() { out.push('&'); }
                    out.push_str(&enc_k);
                    out.push('=');
                    out.push_str(&QueryEscape(&**v));
                }
            }
        }
        out.into()
    }

    pub fn Len(&self) -> int { self.inner.len() as int }

    pub fn Values(&self, key: impl AsRef<str>) -> slice<string> {
        self.inner.get(key.as_ref()).cloned().unwrap_or_default()
    }
}

#[allow(non_snake_case)]
pub fn ParseQuery(s: impl AsRef<str>) -> (Values, error) {
    let mut v = Values::new();
    let mut err: error = nil;
    for part in s.as_ref().split('&') {
        if part.is_empty() { continue; }
        let (k, val) = match part.find('=') {
            Some(i) => (&part[..i], &part[i + 1..]),
            None => (part, ""),
        };
        let (key, e1) = QueryUnescape(k);
        let (value, e2) = QueryUnescape(val);
        if e1 != nil { err = e1; }
        if e2 != nil { err = e2; }
        v.Add(key, value);
    }
    (v, err)
}

// ── Escape / unescape ────────────────────────────────────────────────

#[allow(non_snake_case)]
pub fn QueryEscape(s: impl AsRef<str>) -> string {
    escape(s.as_ref(), true)
}

#[allow(non_snake_case)]
pub fn PathEscape(s: impl AsRef<str>) -> string {
    escape(s.as_ref(), false)
}

fn escape(s: &str, is_query: bool) -> string {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        if should_not_escape(b, is_query) {
            out.push(b as char);
        } else if b == b' ' && is_query {
            out.push('+');
        } else {
            out.push('%');
            out.push(hex_digit(b >> 4));
            out.push(hex_digit(b & 0xf));
        }
    }
    out.into()
}

fn should_not_escape(b: u8, is_query: bool) -> bool {
    if b.is_ascii_alphanumeric() { return true; }
    match b {
        b'-' | b'.' | b'_' | b'~' => true,
        b'$' | b'&' | b'+' | b',' | b'/' | b':' | b';' | b'=' | b'?' | b'@' => !is_query && b != b'?',
        _ => false,
    }
}

fn hex_digit(n: u8) -> char {
    match n {
        0..=9 => (b'0' + n) as char,
        10..=15 => (b'A' + (n - 10)) as char,
        _ => '?',
    }
}

#[allow(non_snake_case)]
pub fn QueryUnescape(s: impl AsRef<str>) -> (string, error) {
    unescape(s.as_ref(), true)
}

#[allow(non_snake_case)]
pub fn PathUnescape(s: impl AsRef<str>) -> (string, error) {
    unescape(s.as_ref(), false)
}

fn unescape(s: &str, is_query: bool) -> (string, error) {
    let mut out = Vec::<u8>::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'%' {
            if i + 2 >= bytes.len() {
                return ("".into(), New("invalid URL escape"));
            }
            let hi = hex_val(bytes[i + 1]);
            let lo = hex_val(bytes[i + 2]);
            if hi < 0 || lo < 0 {
                return ("".into(), New("invalid URL escape"));
            }
            out.push(((hi as u8) << 4) | (lo as u8));
            i += 3;
        } else if b == b'+' && is_query {
            out.push(b' ');
            i += 1;
        } else {
            out.push(b);
            i += 1;
        }
    }
    (String::from_utf8_lossy(&out).into_owned().into(), nil)
}

fn path_clean(p: &str) -> String {
    if p.is_empty() { return ".".into(); }
    let absolute = p.starts_with('/');
    let mut stack: Vec<&str> = Vec::new();
    for part in p.split('/') {
        match part {
            "" | "." => continue,
            ".." => {
                if stack.last().map_or(false, |t| *t != "..") && !stack.is_empty() {
                    stack.pop();
                } else if !absolute {
                    stack.push("..");
                }
            }
            other => stack.push(other),
        }
    }
    let joined = stack.join("/");
    if absolute { format!("/{}", joined) }
    else if joined.is_empty() { ".".into() }
    else { joined }
}

/// url.JoinPath(base, elem...) — returns base with elem joined to its path.
#[allow(non_snake_case)]
pub fn JoinPath(base: impl AsRef<str>, elem: &[impl AsRef<str>]) -> (string, error) {
    let (u, err) = Parse(base);
    if err != nil { return ("".into(), err); }
    let out = u.JoinPath(elem);
    (out.String(), nil)
}

/// url.ParseRequestURI parses rawurl as absolute (either URL or path-only) for server Request.URI.
#[allow(non_snake_case)]
pub fn ParseRequestURI(raw: impl AsRef<str>) -> (URL, error) {
    Parse(raw)
}

fn hex_val(b: u8) -> i32 {
    match b {
        b'0'..=b'9' => (b - b'0') as i32,
        b'a'..=b'f' => (b - b'a' + 10) as i32,
        b'A'..=b'F' => (b - b'A' + 10) as i32,
        _ => -1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_http() {
        let (u, err) = Parse("http://example.com/foo/bar");
        assert_eq!(err, nil);
        assert_eq!(u.Scheme, "http");
        assert_eq!(u.Host, "example.com");
        assert_eq!(u.Path, "/foo/bar");
    }

    #[test]
    fn parse_with_userinfo_and_port() {
        let (u, err) = Parse("https://alice:s3cret@api.example.com:8443/v1/ping?x=1&y=2#section");
        assert_eq!(err, nil);
        assert_eq!(u.Scheme, "https");
        assert_eq!(u.Host, "api.example.com:8443");
        assert_eq!(u.Hostname(), "api.example.com");
        assert_eq!(u.Port(), "8443");
        assert_eq!(u.Path, "/v1/ping");
        assert_eq!(u.RawQuery, "x=1&y=2");
        assert_eq!(u.Fragment, "section");
        let ui = u.User.as_ref().unwrap();
        assert_eq!(ui.username, "alice");
        assert_eq!(ui.password, Some("s3cret".into()));
    }

    #[test]
    fn parse_ipv6_host() {
        let (u, err) = Parse("http://[::1]:8080/path");
        assert_eq!(err, nil);
        assert_eq!(u.Hostname(), "::1");
        assert_eq!(u.Port(), "8080");
    }

    #[test]
    fn parse_opaque_mailto() {
        let (u, err) = Parse("mailto:alice@example.com");
        assert_eq!(err, nil);
        assert_eq!(u.Scheme, "mailto");
        assert_eq!(u.Opaque, "alice@example.com");
    }

    #[test]
    fn parse_raw_path_empty_when_clean() {
        // Go invariant: RawPath is "" when Path re-encodes back to the
        // original bytes. Keeps reflect.DeepEqual-style URL equality
        // stable for the common no-escape case.
        let (u, err) = Parse("https://x.com/abc");
        assert_eq!(err, nil);
        assert_eq!(u.Path, "/abc");
        assert_eq!(u.RawPath, "");
    }

    #[test]
    fn parse_raw_path_kept_when_escapes() {
        // With escapes, RawPath preserves the original so round-trip
        // encoding can distinguish "/a%2Fb" from "/a/b".
        let (u, err) = Parse("https://x.com/a%2Fb");
        assert_eq!(err, nil);
        assert_eq!(u.Path, "/a/b");
        assert_eq!(u.RawPath, "/a%2Fb");
    }

    #[test]
    fn url_string_roundtrip() {
        let (u, _) = Parse("http://example.com/foo?bar=1#x");
        assert_eq!(u.String(), "http://example.com/foo?bar=1#x");
    }

    #[test]
    fn query_escape_and_unescape() {
        assert_eq!(QueryEscape("hello world/+?"), "hello+world%2F%2B%3F");
        let (v, err) = QueryUnescape("hello+world%2F");
        assert_eq!(err, nil);
        assert_eq!(v, "hello world/");
    }

    #[test]
    fn path_escape_preserves_slashes() {
        assert_eq!(PathEscape("a b/c"), "a%20b/c");
    }

    #[test]
    fn values_encode_sorts_keys() {
        let mut v = Values::new();
        v.Set("name", "alice smith");
        v.Add("tag", "a");
        v.Add("tag", "b");
        assert_eq!(v.Encode(), "name=alice+smith&tag=a&tag=b");
    }

    #[test]
    fn parse_query_round_trip() {
        let (v, err) = ParseQuery("a=1&a=2&b=three");
        assert_eq!(err, nil);
        assert_eq!(v.Values("a"), vec!["1", "2"]);
        assert_eq!(v.Get("b"), "three");
    }

    #[test]
    fn bad_escape_is_error() {
        let (_, err) = QueryUnescape("%ZZ");
        assert!(err != nil);
    }
}
