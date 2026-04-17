// net: networking primitives namespace.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   "net/url"                           goish::net::url
//   "net/http"                          goish::net::http
//
// Go's `net` package itself contains Dial/Listen + IP address types;
// those remain a later milestone (see tracking #23).

pub mod http;
pub mod mail;
pub mod netip;
pub mod smtp;
pub mod textproto;
pub mod url;

use crate::errors::{error, New, nil};
use crate::types::string;

/// `net.SplitHostPort(hostport)` — split "host:port", "host%zone:port",
/// "[host]:port", or "[host%zone]:port" into host / port.
///
/// Examples (matching Go's net/ipsock.go):
///   SplitHostPort("example.com:80")       → ("example.com", "80", nil)
///   SplitHostPort("[::1]:8080")           → ("::1",         "8080", nil)
///   SplitHostPort("[::1%eth0]:8080")      → ("::1%eth0",    "8080", nil)
///   SplitHostPort("bad")                  → ("", "", "missing port in address")
#[allow(non_snake_case)]
pub fn SplitHostPort(hostport: impl AsRef<str>) -> (string, string, error) {
    let hp = hostport.as_ref();
    fn err_missing(addr: &str) -> error {
        New(&format!("address {}: missing port in address", addr))
    }
    fn err_too_many(addr: &str) -> error {
        New(&format!("address {}: too many colons in address", addr))
    }
    // Bracketed host form: [host]:port or [host%zone]:port
    if hp.starts_with('[') {
        let Some(close) = hp.find(']') else {
            return ("".into(), "".into(),
                    New(&format!("address {}: missing ']' in address", hp)));
        };
        let host = &hp[1..close];
        let rest = &hp[close + 1..];
        if rest.is_empty() || !rest.starts_with(':') {
            return ("".into(), "".into(), err_missing(hp));
        }
        let port = &rest[1..];
        if port.contains(':') {
            return ("".into(), "".into(), err_too_many(hp));
        }
        return (host.into(), port.into(), nil);
    }
    // Unbracketed form: host:port — exactly one colon allowed.
    let bytes = hp.as_bytes();
    let Some(colon) = bytes.iter().rposition(|&b| b == b':') else {
        return ("".into(), "".into(), err_missing(hp));
    };
    let host = &hp[..colon];
    let port = &hp[colon + 1..];
    if host.contains(':') {
        return ("".into(), "".into(), err_too_many(hp));
    }
    (host.into(), port.into(), nil)
}

/// `net.JoinHostPort(host, port)` — inverse of SplitHostPort.
/// Wraps IPv6 hosts in brackets.
#[allow(non_snake_case)]
pub fn JoinHostPort(host: impl AsRef<str>, port: impl AsRef<str>) -> string {
    let h = host.as_ref();
    let p = port.as_ref();
    if h.contains(':') {
        format!("[{}]:{}", h, p).into()
    } else {
        format!("{}:{}", h, p).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_unbracketed() {
        let (h, p, err) = SplitHostPort("example.com:80");
        assert_eq!(err, crate::errors::nil);
        assert_eq!(h, "example.com");
        assert_eq!(p, "80");
    }

    #[test]
    fn split_bracketed_ipv6() {
        let (h, p, err) = SplitHostPort("[::1]:8080");
        assert_eq!(err, crate::errors::nil);
        assert_eq!(h, "::1");
        assert_eq!(p, "8080");
    }

    #[test]
    fn split_ipv6_with_zone() {
        let (h, p, err) = SplitHostPort("[fe80::1%eth0]:443");
        assert_eq!(err, crate::errors::nil);
        assert_eq!(h, "fe80::1%eth0");
        assert_eq!(p, "443");
    }

    #[test]
    fn split_missing_port_errors() {
        let (_, _, err) = SplitHostPort("example.com");
        assert!(err != crate::errors::nil);
    }

    #[test]
    fn split_too_many_colons_errors() {
        let (_, _, err) = SplitHostPort("a:b:c");
        assert!(err != crate::errors::nil);
    }

    #[test]
    fn join_roundtrip() {
        assert_eq!(JoinHostPort("example.com", "80"), "example.com:80");
        assert_eq!(JoinHostPort("::1", "8080"), "[::1]:8080");
    }
}
