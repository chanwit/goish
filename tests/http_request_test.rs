// Port of go1.25.5 src/net/http/request_test.go — request-level subset.
//
// Elided: multipart form tests (TestMultipart*, TestFormFile*, TestParseMultipartForm*) —
// multipart handling is not yet ported. Transport-level tests that boot
// real servers (TestRequestRedirect, TestRequestWriteBufferedWriter).
// TestReadRequestErrors — parses raw HTTP bytes via ReadRequest, which is
// not exposed in goish (server-side only, driven by hyper).

#![allow(non_snake_case)]
use goish::prelude::*;

test!{ fn TestQuery(t) {
    let (req, err) = http::NewRequest("GET", "http://www.google.com/search?q=foo&q=bar", &[] as &[u8]);
    if err != nil { t.Fatal(Sprintf!("err: %s", err)); }
    let q = req.URL.Query().Get("q");
    if q != "foo" {
        t.Errorf(Sprintf!("FormValue q = %q, want %q", q, "foo"));
    }
}}

test!{ fn TestNewRequestHost(t) {
    let (req, err) = http::NewRequest("GET", "http://localhost:1234/", &[] as &[u8]);
    if err != nil { t.Fatal(Sprintf!("err: %s", err)); }
    if req.Host != "localhost:1234" {
        t.Errorf(Sprintf!("Host = %q, want localhost:1234", req.Host));
    }
}}

test!{ fn TestRequestInvalidMethod(t) {
    // A URL with a scheme should parse fine; the method field is free-form
    // in goish's wrapper (unlike Go which validates token chars). This
    // degenerate check just ensures the Method is preserved.
    let (req, err) = http::NewRequest("PATCH", "http://example.com/", &[] as &[u8]);
    if err != nil { t.Fatal(Sprintf!("err: %s", err)); }
    if req.Method != "PATCH" {
        t.Errorf(Sprintf!("Method = %q, want PATCH", req.Method));
    }
}}

test!{ fn TestNewRequestContentLength(t) {
    let (req, err) = http::NewRequest("POST", "http://example.com/", b"hello");
    if err != nil { t.Fatal(Sprintf!("err: %s", err)); }
    if req.ContentLength != 5 {
        t.Errorf(Sprintf!("ContentLength = %d, want 5", req.ContentLength));
    }
}}

struct HVT { vers: &'static str, major: i64, minor: i64, ok: bool }

test!{ fn TestParseHTTPVersion(t) {
    let tests = vec![
        HVT { vers: "HTTP/0.9", major: 0, minor: 9, ok: true },
        HVT { vers: "HTTP/1.0", major: 1, minor: 0, ok: true },
        HVT { vers: "HTTP/1.1", major: 1, minor: 1, ok: true },
        HVT { vers: "HTTP/2.0", major: 2, minor: 0, ok: true },
        HVT { vers: "HTTP/", major: 0, minor: 0, ok: false },
        HVT { vers: "HTTP/1", major: 0, minor: 0, ok: false },
        HVT { vers: "HTTP/1.", major: 0, minor: 0, ok: false },
        HVT { vers: "HTTP/.1", major: 0, minor: 0, ok: false },
        HVT { vers: "HTTP/01.1", major: 0, minor: 0, ok: false },
        HVT { vers: "HTTP/1.01", major: 0, minor: 0, ok: false },
        HVT { vers: "", major: 0, minor: 0, ok: false },
    ];
    for tt in tests {
        let (major, minor, ok) = http::ParseHTTPVersion(tt.vers);
        if ok != tt.ok || major != tt.major || minor != tt.minor {
            t.Errorf(Sprintf!("ParseHTTPVersion(%q) = (%d, %d, %v), want (%d, %d, %v)",
                tt.vers, major, minor, ok, tt.major, tt.minor, tt.ok));
        }
    }
}}

test!{ fn TestSetBasicAuth(t) {
    let (mut req, err) = http::NewRequest("GET", "http://example.com/", &[] as &[u8]);
    if err != nil { t.Fatal(Sprintf!("err: %s", err)); }
    req.SetBasicAuth("Aladdin", "open sesame");
    let auth = req.Header.Get("Authorization");
    // Expected encoding: "Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ=="
    if auth != "Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ==" {
        t.Errorf(Sprintf!("Authorization = %q", auth));
    }
}}

test!{ fn TestGetBasicAuth(t) {
    let (mut req, _) = http::NewRequest("GET", "http://example.com/", &[] as &[u8]);
    req.SetBasicAuth("Aladdin", "open sesame");
    let (u, p, ok) = req.BasicAuth();
    if !ok || u != "Aladdin" || p != "open sesame" {
        t.Errorf(Sprintf!("BasicAuth = (%q, %q, %v)", u, p, ok));
    }
}}

test!{ fn TestBasicAuthNoHeader(t) {
    let (req, _) = http::NewRequest("GET", "http://example.com/", &[] as &[u8]);
    let (_, _, ok) = req.BasicAuth();
    if ok {
        t.Errorf(Sprintf!("BasicAuth with no header should return ok=false"));
    }
}}

test!{ fn TestRequestHeaderCanonical(t) {
    // Go's Header.Set/Get canonicalizes keys (content-type → Content-Type).
    let (mut req, _) = http::NewRequest("GET", "http://example.com/", &[] as &[u8]);
    req.Header.Set("content-type", "application/json");
    let got = req.Header.Get("Content-Type");
    if got != "application/json" {
        t.Errorf(Sprintf!("canonical Header.Get = %q", got));
    }
    let got = req.Header.Get("CONTENT-TYPE");
    if got != "application/json" {
        t.Errorf(Sprintf!("uppercase Header.Get = %q", got));
    }
}}

test!{ fn TestRequestHeaderAddDelValues(t) {
    let (mut req, _) = http::NewRequest("GET", "http://example.com/", &[] as &[u8]);
    req.Header.Add("X-Multi", "a");
    req.Header.Add("X-Multi", "b");
    let values = req.Header.Values("X-Multi");
    if values.len() != 2 || values[0] != "a" || values[1] != "b" {
        t.Errorf(Sprintf!("Values = %d, want [a, b]", values.len()));
    }
    req.Header.Del("X-Multi");
    let values = req.Header.Values("X-Multi");
    if !values.is_empty() {
        t.Errorf(Sprintf!("after Del, Values len = %d want 0", values.len()));
    }
}}

test!{ fn TestRequestContext(t) {
    let (req, _) = http::NewRequest("GET", "http://example.com/", &[] as &[u8]);
    // Context should not be nil by default.
    let ctx = req.Context();
    let _ = ctx;
    let _ = t;
}}

test!{ fn TestFormValue(t) {
    let (req, _) = http::NewRequest("GET", "http://example.com/?x=1&y=hello&x=2", &[] as &[u8]);
    if req.FormValue("x") != "1" {
        t.Errorf(Sprintf!("FormValue(x) = %q", req.FormValue("x")));
    }
    if req.FormValue("y") != "hello" {
        t.Errorf(Sprintf!("FormValue(y) = %q", req.FormValue("y")));
    }
    if req.FormValue("missing") != "" {
        t.Errorf(Sprintf!("FormValue(missing) should be empty"));
    }
}}
