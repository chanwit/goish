//! html: Go's html package — EscapeString / UnescapeString.
//!
//!   Go                                  goish
//!   ─────────────────────────────────   ──────────────────────────────────
//!   html.EscapeString(s)                html::EscapeString(s)
//!   html.UnescapeString(s)              html::UnescapeString(s)
//!
//! Scope: the five "core" HTML entities (&amp; &lt; &gt; &quot; &#39;)
//! plus numeric (`&#NN;` / `&#xHH;`). The full HTML5 named-entity table
//! (~2000 entries) is not ported — it belongs to `html/template` which
//! is deferred to the long-tail milestone.

/// `html.EscapeString(s)` — replaces `<`, `>`, `&`, `"`, `'` with their
/// named entity equivalents.
#[allow(non_snake_case)]
pub fn EscapeString(s: impl AsRef<str>) -> String {
    let s = s.as_ref();
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&#34;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// `html.UnescapeString(s)` — reverses EscapeString, plus recognises
/// `&amp;`, `&lt;`, `&gt;`, `&quot;`, `&apos;`, `&#NN;`, `&#xHH;`. Any
/// unrecognised `&...;` is left intact (Go: substrings that don't parse
/// as entities pass through).
#[allow(non_snake_case)]
pub fn UnescapeString(s: impl AsRef<str>) -> String {
    let s = s.as_ref();
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'&' {
            // UTF-8 safe: copy byte directly since valid UTF-8 bytes
            // outside the ASCII range are passed through unchanged.
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }
        // Locate the terminating ';' (up to 10 bytes ahead is generous
        // for the entities we care about).
        let semi = bytes[i..].iter().take(10).position(|&b| b == b';');
        match semi {
            Some(len) => {
                let entity = &s[i..i + len + 1];  // includes & and ;
                if let Some(c) = decode_entity(entity) {
                    out.push(c);
                    i += len + 1;
                    continue;
                }
                // Unrecognised — pass through.
                out.push('&');
                i += 1;
            }
            None => {
                out.push('&');
                i += 1;
            }
        }
    }
    out
}

fn decode_entity(ent: &str) -> Option<char> {
    // ent starts with '&' and ends with ';'.
    let inner = &ent[1..ent.len() - 1];
    match inner {
        "amp"  => Some('&'),
        "lt"   => Some('<'),
        "gt"   => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        _ if inner.starts_with('#') => {
            let digits = &inner[1..];
            let (val, base) = if let Some(hex) = digits.strip_prefix('x').or_else(|| digits.strip_prefix('X')) {
                (hex, 16)
            } else {
                (digits, 10)
            };
            u32::from_str_radix(val, base).ok().and_then(char::from_u32)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_specials() {
        assert_eq!(EscapeString("a<b & c>d"), "a&lt;b &amp; c&gt;d");
        assert_eq!(EscapeString("\"'"), "&#34;&#39;");
    }

    #[test]
    fn unescape_named() {
        assert_eq!(UnescapeString("&lt;hi&gt;"), "<hi>");
        assert_eq!(UnescapeString("&amp;"), "&");
        assert_eq!(UnescapeString("&quot;"), "\"");
        assert_eq!(UnescapeString("&apos;"), "'");
    }

    #[test]
    fn unescape_numeric() {
        assert_eq!(UnescapeString("&#65;"), "A");
        assert_eq!(UnescapeString("&#x41;"), "A");
        assert_eq!(UnescapeString("&#x1F600;"), "😀");
    }

    #[test]
    fn unescape_unknown_passthrough() {
        assert_eq!(UnescapeString("&unknown;"), "&unknown;");
        assert_eq!(UnescapeString("a & b"), "a & b");
    }

    #[test]
    fn round_trip_core() {
        let s = "<script>alert(\"x & y\")</script>";
        assert_eq!(UnescapeString(EscapeString(s)), s);
    }
}
