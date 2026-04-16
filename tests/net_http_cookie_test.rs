// Port of go1.25.5 src/net/http/cookie_test.go — Cookie.String() round-
// trips from writeSetCookiesTests, plus ParseCookie + ParseSetCookie.

#![allow(non_snake_case)]
use goish::prelude::*;
use goish::net::http::{self, SameSiteLaxMode, SameSiteNoneMode, SameSiteStrictMode};
use goish::net::http::Cookie as CookieType;

// ── TestWriteSetCookies (core subset — no Expires time formatting) ──

test!{ fn TestWriteSetCookies(t) {
    struct Case { cookie: CookieType, raw: &'static str }
    let cases = [
        Case { cookie: Cookie!{Name: "cookie-1", Value: "v$1"},
               raw: "cookie-1=v$1" },
        Case { cookie: Cookie!{Name: "cookie-2", Value: "two", MaxAge: 3600},
               raw: "cookie-2=two; Max-Age=3600" },
        Case { cookie: Cookie!{Name: "cookie-3", Value: "three", Domain: ".example.com"},
               raw: "cookie-3=three; Domain=example.com" },
        Case { cookie: Cookie!{Name: "cookie-4", Value: "four", Path: "/restricted/"},
               raw: "cookie-4=four; Path=/restricted/" },
        Case { cookie: Cookie!{Name: "cookie-5", Value: "five", Domain: "wrong;bad.abc"},
               raw: "cookie-5=five" },
        Case { cookie: Cookie!{Name: "cookie-6", Value: "six", Domain: "bad-.abc"},
               raw: "cookie-6=six" },
        Case { cookie: Cookie!{Name: "cookie-7", Value: "seven", Domain: "127.0.0.1"},
               raw: "cookie-7=seven; Domain=127.0.0.1" },
        Case { cookie: Cookie!{Name: "cookie-8", Value: "eight", Domain: "::1"},
               raw: "cookie-8=eight" },
        Case { cookie: Cookie!{Name: "cookie-13", Value: "samesite-lax", SameSite: SameSiteLaxMode},
               raw: "cookie-13=samesite-lax; SameSite=Lax" },
        Case { cookie: Cookie!{Name: "cookie-14", Value: "samesite-strict", SameSite: SameSiteStrictMode},
               raw: "cookie-14=samesite-strict; SameSite=Strict" },
        Case { cookie: Cookie!{Name: "cookie-15", Value: "samesite-none", SameSite: SameSiteNoneMode},
               raw: "cookie-15=samesite-none; SameSite=None" },
        Case { cookie: Cookie!{
                 Name: "cookie-16", Value: "partitioned",
                 SameSite: SameSiteNoneMode, Secure: true, Path: "/",
                 Partitioned: true
               },
               raw: "cookie-16=partitioned; Path=/; Secure; SameSite=None; Partitioned" },
        // Quoted values (issue #46443)
        Case { cookie: Cookie!{Name: "cookie", Value: "quoted", Quoted: true},
               raw: r#"cookie="quoted""# },
        Case { cookie: Cookie!{Name: "cookie", Value: "quoted with spaces", Quoted: true},
               raw: r#"cookie="quoted with spaces""# },
        Case { cookie: Cookie!{Name: "cookie", Value: "quoted,with,commas", Quoted: true},
               raw: r#"cookie="quoted,with,commas""# },
        // Special values wrapped in quotes
        Case { cookie: Cookie!{Name: "special-1", Value: "a z"},
               raw: r#"special-1="a z""# },
        Case { cookie: Cookie!{Name: "special-5", Value: "a,z"},
               raw: r#"special-5="a,z""# },
        Case { cookie: Cookie!{Name: "empty-value", Value: ""},
               raw: "empty-value=" },
        // Invalid names produce empty string
        Case { cookie: Cookie!{Name: ""}, raw: "" },
        Case { cookie: Cookie!{Name: "\t"}, raw: "" },
        Case { cookie: Cookie!{Name: "a\nb", Value: "v"}, raw: "" },
        Case { cookie: Cookie!{Name: "a\rb", Value: "v"}, raw: "" },
    ];
    r#for!{ i, c := range (cases[..]) {
        let got = c.cookie.String();
        if got != c.raw {
            t.Errorf(Sprintf!("Test %d:\nwant: %s\n got: %s", i as i64, c.raw, got));
        }
    }}
}}

// ── TestParseCookie (Cookie request header parsing) ─────────────────

test!{ fn TestParseCookie(t) {
    struct Case {
        line: &'static str,
        names: &'static [&'static str],
        values: &'static [&'static str],
    }
    let cases = [
        Case { line: "Cookie-1=v$1",                 names: &["Cookie-1"][..],         values: &["v$1"][..] },
        Case { line: "Cookie-1=v$1; c2=v2",          names: &["Cookie-1", "c2"][..],    values: &["v$1", "v2"][..] },
        Case { line: r#"quoted="hello world""#,      names: &["quoted"][..],            values: &["hello world"][..] },
    ];
    for c in &cases {
        let (cookies, err) = http::ParseCookie(c.line);
        if err != nil {
            t.Errorf(Sprintf!("ParseCookie(%s): error: %s", c.line, err));
            continue;
        }
        if len!(cookies) != len!(c.names) {
            t.Errorf(Sprintf!("ParseCookie(%s): len = %d, want %d",
                c.line, len!(cookies) as i64, len!(c.names) as i64));
            continue;
        }
        r#for!{ i, ck := range (cookies[..]) {
            if ck.Name != c.names[i] {
                t.Errorf(Sprintf!("[%d].Name = %s, want %s", i as i64, ck.Name, c.names[i]));
            }
            if ck.Value != c.values[i] {
                t.Errorf(Sprintf!("[%d].Value = %s, want %s", i as i64, ck.Value, c.values[i]));
            }
        }}
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

// ── TestParseSetCookie — tabular Go `readSetCookiesTests` port ──────

test!{ fn TestParseSetCookieTable(t) {
    struct Case {
        input: &'static str,
        name: &'static str,
        value: &'static str,
        path: &'static str,
        domain: &'static str,
        http_only: bool,
        secure: bool,
        same_site: goish::net::http::SameSite,
    }
    use goish::net::http::SameSite::*;
    let cases = [
        Case { input: "Cookie-1=v$1",
               name: "Cookie-1", value: "v$1", path: "", domain: "",
               http_only: false, secure: false, same_site: Default },
        Case { input: "ASP.NET_SessionId=foo; path=/; HttpOnly",
               name: "ASP.NET_SessionId", value: "foo", path: "/", domain: "",
               http_only: true, secure: false, same_site: Default },
        Case { input: "samesitedefault=foo; SameSite",
               name: "samesitedefault", value: "foo", path: "", domain: "",
               http_only: false, secure: false, same_site: Default },
        Case { input: "samesitelax=foo; SameSite=Lax",
               name: "samesitelax", value: "foo", path: "", domain: "",
               http_only: false, secure: false, same_site: Lax },
        Case { input: "samesitestrict=foo; SameSite=Strict",
               name: "samesitestrict", value: "foo", path: "", domain: "",
               http_only: false, secure: false, same_site: Strict },
        Case { input: "samesitenone=foo; SameSite=None",
               name: "samesitenone", value: "foo", path: "", domain: "",
               http_only: false, secure: false, same_site: None },
        Case { input: r#"special-2=" z""#,
               name: "special-2", value: " z", path: "", domain: "",
               http_only: false, secure: false, same_site: Default },
    ];
    r#for!{ i, c := range (cases[..]) {
        let (got, err) = http::ParseSetCookie(c.input);
        if err != nil { t.Fatal(&Sprintf!("case %d: ParseSetCookie: %s", i as i64, err)); }
        if got.Name != c.name {
            t.Errorf(Sprintf!("case %d: Name = %s, want %s", i as i64, got.Name, c.name));
        }
        if got.Value != c.value {
            t.Errorf(Sprintf!("case %d: Value = %s, want %s", i as i64, got.Value, c.value));
        }
        if got.Path != c.path {
            t.Errorf(Sprintf!("case %d: Path = %s, want %s", i as i64, got.Path, c.path));
        }
        if got.Domain != c.domain {
            t.Errorf(Sprintf!("case %d: Domain = %s, want %s", i as i64, got.Domain, c.domain));
        }
        if got.HttpOnly != c.http_only {
            t.Errorf(Sprintf!("case %d: HttpOnly mismatch", i as i64));
        }
        if got.Secure != c.secure {
            t.Errorf(Sprintf!("case %d: Secure mismatch", i as i64));
        }
        if got.SameSite != c.same_site {
            t.Errorf(Sprintf!("case %d: SameSite mismatch", i as i64));
        }
    }}
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
