// Port of go1.25.5 src/html/escape_test.go — EscapeString +
// UnescapeString. Skips the full HTML5 named-entity table (goish only
// implements the five "core" entities + numeric references, which is
// what users need 99% of the time — see module docs).

#![allow(non_snake_case)]
use goish::prelude::*;

test!{ fn TestEscape(t) {
    let cases: Vec<(&str, &str)> = vec![
        ("", ""),
        ("abc", "abc"),
        ("<", "&lt;"),
        (">", "&gt;"),
        ("&", "&amp;"),
        ("\"", "&#34;"),
        ("'", "&#39;"),
        ("<script>alert('x')</script>", "&lt;script&gt;alert(&#39;x&#39;)&lt;/script&gt;"),
    ];
    for (inp, want) in cases {
        let got = html::EscapeString(inp);
        if got != want {
            t.Errorf(Sprintf!("EscapeString(%q) = %q, want %q", inp, got, want));
        }
    }
}}

test!{ fn TestUnescapeNamed(t) {
    let cases: Vec<(&str, &str)> = vec![
        ("", ""),
        ("abc", "abc"),
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&amp;", "&"),
        ("&quot;", "\""),
        ("&apos;", "'"),
        ("a&lt;b&amp;c&gt;d", "a<b&c>d"),
    ];
    for (inp, want) in cases {
        let got = html::UnescapeString(inp);
        if got != want {
            t.Errorf(Sprintf!("UnescapeString(%q) = %q, want %q", inp, got, want));
        }
    }
}}

test!{ fn TestUnescapeNumeric(t) {
    if html::UnescapeString("&#65;&#x42;&#x43;") != "ABC" {
        t.Errorf(Sprintf!("UnescapeString numeric failed"));
    }
    if html::UnescapeString("&#x1F600;") != "😀" {
        t.Errorf(Sprintf!("UnescapeString emoji failed"));
    }
}}

test!{ fn TestUnknownPassthrough(t) {
    // Go: unrecognised `&foo;` sequences pass through literally.
    if html::UnescapeString("&foo;") != "&foo;" {
        t.Errorf(Sprintf!("&foo; should pass through"));
    }
    // Bare & without ;
    if html::UnescapeString("a & b") != "a & b" {
        t.Errorf(Sprintf!("bare & should pass through"));
    }
}}

test!{ fn TestRoundTrip(t) {
    let cases = [
        "plain text",
        "<tag>",
        "a & b < c > d",
        "\"quoted\" and 'apos'",
    ];
    for s in cases {
        let e = html::EscapeString(s);
        let back = html::UnescapeString(&e);
        if back != s {
            t.Errorf(Sprintf!("round-trip(%q) = %q", s, back));
        }
    }
}}
