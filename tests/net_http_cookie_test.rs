// Port of go1.25.5 src/net/http/cookie_test.go — Cookie.String() round-
// trips from writeSetCookiesTests, plus ParseCookie + ParseSetCookie.

#![allow(non_snake_case)]
use goish::prelude::*;
use goish::net::http::{self, Cookie, SameSite, SameSiteLaxMode, SameSiteNoneMode, SameSiteStrictMode};

// ── TestWriteSetCookies (core subset — no Expires time formatting) ──

test!{ fn TestWriteSetCookies(t) {
    struct Case { cookie: Cookie, raw: &'static str }
    let cases = [
        Case { cookie: Cookie { Name: "cookie-1".into(), Value: "v$1".into(), ..Cookie::default() },
               raw: "cookie-1=v$1" },
        Case { cookie: Cookie { Name: "cookie-2".into(), Value: "two".into(), MaxAge: 3600, ..Cookie::default() },
               raw: "cookie-2=two; Max-Age=3600" },
        Case { cookie: Cookie { Name: "cookie-3".into(), Value: "three".into(), Domain: ".example.com".into(), ..Cookie::default() },
               raw: "cookie-3=three; Domain=example.com" },
        Case { cookie: Cookie { Name: "cookie-4".into(), Value: "four".into(), Path: "/restricted/".into(), ..Cookie::default() },
               raw: "cookie-4=four; Path=/restricted/" },
        Case { cookie: Cookie { Name: "cookie-5".into(), Value: "five".into(), Domain: "wrong;bad.abc".into(), ..Cookie::default() },
               raw: "cookie-5=five" },
        Case { cookie: Cookie { Name: "cookie-6".into(), Value: "six".into(), Domain: "bad-.abc".into(), ..Cookie::default() },
               raw: "cookie-6=six" },
        Case { cookie: Cookie { Name: "cookie-7".into(), Value: "seven".into(), Domain: "127.0.0.1".into(), ..Cookie::default() },
               raw: "cookie-7=seven; Domain=127.0.0.1" },
        Case { cookie: Cookie { Name: "cookie-8".into(), Value: "eight".into(), Domain: "::1".into(), ..Cookie::default() },
               raw: "cookie-8=eight" },
        Case { cookie: Cookie { Name: "cookie-12".into(), Value: "samesite-default".into(), SameSite: SameSite::Default, ..Cookie::default() },
               raw: "cookie-12=samesite-default" },
        Case { cookie: Cookie { Name: "cookie-13".into(), Value: "samesite-lax".into(), SameSite: SameSiteLaxMode, ..Cookie::default() },
               raw: "cookie-13=samesite-lax; SameSite=Lax" },
        Case { cookie: Cookie { Name: "cookie-14".into(), Value: "samesite-strict".into(), SameSite: SameSiteStrictMode, ..Cookie::default() },
               raw: "cookie-14=samesite-strict; SameSite=Strict" },
        Case { cookie: Cookie { Name: "cookie-15".into(), Value: "samesite-none".into(), SameSite: SameSiteNoneMode, ..Cookie::default() },
               raw: "cookie-15=samesite-none; SameSite=None" },
        // Partitioned + Secure
        Case { cookie: Cookie {
                 Name: "cookie-16".into(), Value: "partitioned".into(),
                 SameSite: SameSiteNoneMode, Secure: true, Path: "/".into(),
                 Partitioned: true, ..Cookie::default()
               },
               raw: "cookie-16=partitioned; Path=/; Secure; SameSite=None; Partitioned" },
        // Quoted values (issue #46443)
        Case { cookie: Cookie { Name: "cookie".into(), Value: "quoted".into(), Quoted: true, ..Cookie::default() },
               raw: r#"cookie="quoted""# },
        Case { cookie: Cookie { Name: "cookie".into(), Value: "quoted with spaces".into(), Quoted: true, ..Cookie::default() },
               raw: r#"cookie="quoted with spaces""# },
        Case { cookie: Cookie { Name: "cookie".into(), Value: "quoted,with,commas".into(), Quoted: true, ..Cookie::default() },
               raw: r#"cookie="quoted,with,commas""# },
        // Special values wrapped in quotes
        Case { cookie: Cookie { Name: "special-1".into(), Value: "a z".into(), ..Cookie::default() },
               raw: r#"special-1="a z""# },
        Case { cookie: Cookie { Name: "special-5".into(), Value: "a,z".into(), ..Cookie::default() },
               raw: r#"special-5="a,z""# },
        Case { cookie: Cookie { Name: "empty-value".into(), Value: "".into(), ..Cookie::default() },
               raw: "empty-value=" },
        // Invalid names produce empty string
        Case { cookie: Cookie { Name: "".into(), ..Cookie::default() }, raw: "" },
        Case { cookie: Cookie { Name: "\t".into(), ..Cookie::default() }, raw: "" },
        Case { cookie: Cookie { Name: "a\nb".into(), Value: "v".into(), ..Cookie::default() }, raw: "" },
        Case { cookie: Cookie { Name: "a\rb".into(), Value: "v".into(), ..Cookie::default() }, raw: "" },
    ];
    for (i, c) in cases.iter().enumerate() {
        let got = c.cookie.String();
        if got != c.raw {
            t.Errorf(Sprintf!("Test %d:\nwant: %s\n got: %s", i as i64, c.raw, got));
        }
    }
}}

// ── TestParseCookie (Cookie request header parsing) ─────────────────

test!{ fn TestParseCookie(t) {
    struct Case {
        line: &'static str,
        names: Vec<&'static str>,
        values: Vec<&'static str>,
    }
    let cases = vec![
        Case { line: "Cookie-1=v$1",                           names: vec!["Cookie-1"],              values: vec!["v$1"] },
        Case { line: "Cookie-1=v$1; c2=v2",                    names: vec!["Cookie-1", "c2"],         values: vec!["v$1", "v2"] },
        Case { line: r#"quoted="hello world""#,                names: vec!["quoted"],                 values: vec!["hello world"] },
    ];
    for c in &cases {
        let (cookies, err) = http::ParseCookie(c.line);
        if err != nil {
            t.Errorf(Sprintf!("ParseCookie(%s): error: %s", c.line, err));
            continue;
        }
        if cookies.len() != c.names.len() {
            t.Errorf(Sprintf!("ParseCookie(%s): len = %d, want %d", c.line, cookies.len() as i64, c.names.len() as i64));
            continue;
        }
        for (i, ck) in cookies.iter().enumerate() {
            if ck.Name != c.names[i] {
                t.Errorf(Sprintf!("[%d].Name = %s, want %s", i as i64, ck.Name, c.names[i]));
            }
            if ck.Value != c.values[i] {
                t.Errorf(Sprintf!("[%d].Value = %s, want %s", i as i64, ck.Value, c.values[i]));
            }
        }
    }
    // Invalid
    let bad = ["", "no-equals"];
    for b in &bad {
        let (_cs, err) = http::ParseCookie(b);
        if err == nil {
            t.Errorf(Sprintf!("ParseCookie(%s) = no error", b));
        }
    }
}}

// ── TestParseSetCookie (Set-Cookie response header parsing) ─────────

test!{ fn TestParseSetCookie(t) {
    let (c, err) = http::ParseSetCookie("NID=99=YsDT5i3E-CXax-; path=/; domain=.google.ch; HttpOnly");
    if err != nil { t.Fatal(&Sprintf!("ParseSetCookie: %s", err)); }
    if c.Name != "NID" { t.Errorf(Sprintf!("Name = %s, want NID", c.Name)); }
    if c.Value != "99=YsDT5i3E-CXax-" {
        t.Errorf(Sprintf!("Value = %s, want 99=YsDT5i3E-CXax-", c.Value));
    }
    if c.Path != "/" { t.Errorf(Sprintf!("Path = %s, want /", c.Path)); }
    if c.Domain != ".google.ch" { t.Errorf(Sprintf!("Domain = %s, want .google.ch", c.Domain)); }
    if !c.HttpOnly { t.Errorf(Sprintf!("HttpOnly = false, want true")); }

    let (c2, err2) = http::ParseSetCookie("foo=bar; Max-Age=60; Secure; SameSite=Lax");
    if err2 != nil { t.Fatal(&Sprintf!("ParseSetCookie 2: %s", err2)); }
    if c2.MaxAge != 60 { t.Errorf(Sprintf!("MaxAge = %d, want 60", c2.MaxAge)); }
    if !c2.Secure { t.Errorf(Sprintf!("Secure = false")); }
    if c2.SameSite != SameSiteLaxMode { t.Errorf(Sprintf!("SameSite not Lax")); }
}}
