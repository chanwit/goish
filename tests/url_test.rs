// Port of go1.25.5 src/net/url/url_test.go — Parse/String/Query/Escape tables.
//
// Elided: TestGob, TestJSON (serialization not ported); TestResolveReference,
// TestResolvePath (URL resolution not implemented); TestRejectControlCharacters
// (Go-specific reject table — broadly orthogonal); TestURLRedacted (no Redacted
// method); TestParseErrors exact-message checks (we use a looser match).

#![allow(non_snake_case)]
use goish::prelude::*;
use goish::url;

struct UT { r#in: &'static str, scheme: &'static str, host: &'static str,
            path: &'static str, raw_query: &'static str, fragment: &'static str }

test!{ fn TestParse(t) {
    let tests = vec![
        UT { r#in: "http://www.google.com", scheme: "http", host: "www.google.com",
             path: "", raw_query: "", fragment: "" },
        UT { r#in: "http://www.google.com/", scheme: "http", host: "www.google.com",
             path: "/", raw_query: "", fragment: "" },
        UT { r#in: "http://www.google.com/file%20one%26two", scheme: "http",
             host: "www.google.com", path: "/file one&two", raw_query: "", fragment: "" },
        UT { r#in: "http://www.google.com/?q=go+language", scheme: "http",
             host: "www.google.com", path: "/", raw_query: "q=go+language", fragment: "" },
        UT { r#in: "http://www.google.com/?", scheme: "http", host: "www.google.com",
             path: "/", raw_query: "", fragment: "" },
        UT { r#in: "http://www.google.com/a/b/c", scheme: "http", host: "www.google.com",
             path: "/a/b/c", raw_query: "", fragment: "" },
        UT { r#in: "https://www.google.com/#fragment", scheme: "https",
             host: "www.google.com", path: "/", raw_query: "", fragment: "fragment" },
        UT { r#in: "http://example.com:80", scheme: "http", host: "example.com:80",
             path: "", raw_query: "", fragment: "" },
        UT { r#in: "http://[::1]/", scheme: "http", host: "[::1]",
             path: "/", raw_query: "", fragment: "" },
        UT { r#in: "mailto:webmaster@golang.org", scheme: "mailto", host: "",
             path: "", raw_query: "", fragment: "" },
    ];
    for tt in &tests {
        let (u, err) = url::Parse(tt.r#in);
        if err != nil { t.Errorf(Sprintf!("Parse(%q) err: %s", tt.r#in, err)); continue; }
        if u.Scheme != tt.scheme {
            t.Errorf(Sprintf!("Parse(%q).Scheme = %q, want %q", tt.r#in, u.Scheme, tt.scheme));
        }
        if u.Host != tt.host {
            t.Errorf(Sprintf!("Parse(%q).Host = %q, want %q", tt.r#in, u.Host, tt.host));
        }
        if u.Path != tt.path {
            t.Errorf(Sprintf!("Parse(%q).Path = %q, want %q", tt.r#in, u.Path, tt.path));
        }
        if u.RawQuery != tt.raw_query {
            t.Errorf(Sprintf!("Parse(%q).RawQuery = %q, want %q", tt.r#in, u.RawQuery, tt.raw_query));
        }
        if u.Fragment != tt.fragment {
            t.Errorf(Sprintf!("Parse(%q).Fragment = %q, want %q", tt.r#in, u.Fragment, tt.fragment));
        }
    }
}}

struct UnescT { r#in: &'static str, out: &'static str, query: bool }

test!{ fn TestUnescape(t) {
    let tests = vec![
        UnescT { r#in: "", out: "", query: true },
        UnescT { r#in: "abc", out: "abc", query: true },
        UnescT { r#in: "1%41", out: "1A", query: true },
        UnescT { r#in: "1%41%42%43", out: "1ABC", query: true },
        UnescT { r#in: "%40%41%42", out: "@AB", query: true },
        UnescT { r#in: "a+b", out: "a b", query: true },
        UnescT { r#in: "a%20b", out: "a b", query: true },
        UnescT { r#in: "a+b", out: "a+b", query: false },
        UnescT { r#in: "a%20b", out: "a b", query: false },
    ];
    for tt in tests {
        let (got, err) = if tt.query {
            url::QueryUnescape(tt.r#in)
        } else {
            url::PathUnescape(tt.r#in)
        };
        if err != nil { t.Errorf(Sprintf!("unescape(%q): %s", tt.r#in, err)); continue; }
        if got != tt.out {
            t.Errorf(Sprintf!("unescape(%q, q=%v) = %q, want %q", tt.r#in, tt.query, got, tt.out));
        }
    }
}}

test!{ fn TestQueryEscape(t) {
    let cases = vec![
        ("", ""),
        ("abc", "abc"),
        ("one two", "one+two"),
        ("hello world", "hello+world"),
        ("a#b", "a%23b"),
        ("%", "%25"),
        ("/", "%2F"),
    ];
    for (input, want) in cases {
        let got = url::QueryEscape(input);
        if got != want {
            t.Errorf(Sprintf!("QueryEscape(%q) = %q, want %q", input, got, want));
        }
    }
}}

test!{ fn TestPathEscape(t) {
    let cases = vec![
        ("", ""),
        ("abc", "abc"),
        ("one two", "one%20two"),
        ("/path/with/slash", "/path/with/slash"),
        ("?q=1", "%3Fq=1"),
    ];
    for (input, want) in cases {
        let got = url::PathEscape(input);
        if got != want {
            t.Errorf(Sprintf!("PathEscape(%q) = %q, want %q", input, got, want));
        }
    }
}}

test!{ fn TestEncodeQuery(t) {
    let mut v = url::Values::new();
    v.Set("q", "go language");
    v.Add("cat", "lang");
    v.Add("cat", "goish");
    let got = v.Encode();
    if got != "cat=lang&cat=goish&q=go+language" {
        t.Errorf(Sprintf!("Encode = %q", got));
    }
}}

test!{ fn TestParseQuery(t) {
    let (v, err) = url::ParseQuery("a=1&a=2&b=three&c=");
    if err != nil { t.Errorf(Sprintf!("ParseQuery: %s", err)); }
    let aval = v.Values("a");
    if aval.len() != 2 || aval[0] != "1" || aval[1] != "2" {
        t.Errorf(Sprintf!("a values wrong"));
    }
    if v.Get("b") != "three" { t.Errorf(Sprintf!("b = %q", v.Get("b"))); }
    if v.Get("c") != "" { t.Errorf(Sprintf!("c = %q", v.Get("c"))); }
}}

test!{ fn TestQueryValues(t) {
    let mut v = url::Values::new();
    v.Set("name", "alice");
    v.Add("hobby", "running");
    v.Add("hobby", "reading");

    if v.Get("name") != "alice" {
        t.Errorf(Sprintf!("Get(name) wrong"));
    }
    let hobbies = v.Values("hobby");
    if hobbies.len() != 2 || hobbies[1] != "reading" {
        t.Errorf(Sprintf!("Values(hobby) wrong"));
    }
    v.Del("hobby");
    if v.Has("hobby") {
        t.Errorf(Sprintf!("Del failed"));
    }
    v.Del("missing"); // no-op
}}

test!{ fn TestURLString(t) {
    let cases = vec![
        "http://example.com/",
        "http://example.com/foo",
        "http://example.com/foo?bar=1",
        "http://example.com/foo#frag",
        "https://alice@example.com/path",
        "mailto:who@example.com",
    ];
    for s in cases {
        let (u, err) = url::Parse(s);
        if err != nil { t.Errorf(Sprintf!("Parse(%q): %s", s, err)); continue; }
        let got = u.String();
        if got != s {
            t.Errorf(Sprintf!("roundtrip: %q → %q", s, got));
        }
    }
}}

test!{ fn TestURLHostnameAndPort(t) {
    let cases: Vec<(&str, &str, &str)> = vec![
        ("foo.com:80", "foo.com", "80"),
        ("foo.com", "foo.com", ""),
        ("[1::6]:8080", "1::6", "8080"),
        ("[1::6]", "1::6", ""),
    ];
    for (host, hn, port) in cases {
        let u = url::URL { Host: host.into(), ..Default::default() };
        if u.Hostname() != hn {
            t.Errorf(Sprintf!("Hostname(%q) = %q, want %q", host, u.Hostname(), hn));
        }
        if u.Port() != port {
            t.Errorf(Sprintf!("Port(%q) = %q, want %q", host, u.Port(), port));
        }
    }
}}

test!{ fn TestRequestURI(t) {
    let cases: Vec<(&str, &str)> = vec![
        ("http://example.com/foo", "/foo"),
        ("http://example.com/", "/"),
        ("http://example.com", "/"),
        ("http://example.com/path?q=1", "/path?q=1"),
        ("mailto:user@example.com", "user@example.com"),
    ];
    for (input, want) in cases {
        let (u, _) = url::Parse(input);
        let got = u.RequestURI();
        if got != want {
            t.Errorf(Sprintf!("RequestURI(%q) = %q, want %q", input, got, want));
        }
    }
}}

test!{ fn TestJoinPath(t) {
    let cases: Vec<(&str, Vec<&str>, &str)> = vec![
        ("http://example.com/a", vec!["b", "c"], "http://example.com/a/b/c"),
        ("http://example.com", vec!["foo"], "http://example.com/foo"),
        ("http://example.com/a/", vec!["b"], "http://example.com/a/b"),
        ("http://example.com/a/b/c", vec![".."], "http://example.com/a/b"),
    ];
    for (base, elems, want) in cases {
        let (got, err) = url::JoinPath(base, &elems);
        if err != nil { t.Errorf(Sprintf!("JoinPath(%q): %s", base, err)); continue; }
        if got != want {
            t.Errorf(Sprintf!("JoinPath(%q, %d elems) = %q, want %q", base, elems.len(), got, want));
        }
    }
}}

test!{ fn TestParseFailure(t) {
    // Empty is OK. A few things are not OK.
    let bad_escape = "%gg";
    let (_, err) = url::QueryUnescape(bad_escape);
    if err == nil {
        t.Errorf(Sprintf!("QueryUnescape(%q) = nil err", bad_escape));
    }
}}

test!{ fn TestNilUser(t) {
    // A URL with no user should round-trip without crashing.
    let (u, _) = url::Parse("http://example.com/foo");
    if u.User.is_some() {
        t.Errorf(Sprintf!("URL without user reported User"));
    }
}}

test!{ fn TestInvalidUserPassword(t) {
    // A URL with userinfo should parse.
    let (u, err) = url::Parse("http://user:pass@example.com/");
    if err != nil { t.Fatal(Sprintf!("err: %s", err)); }
    let ui = u.User.as_ref().expect("user");
    if ui.Username() != "user" { t.Errorf(Sprintf!("Username = %q", ui.Username())); }
    let (pw, ok) = ui.Password();
    if !ok || pw != "pass" { t.Errorf(Sprintf!("Password = %q (ok=%v)", pw, ok)); }
}}
