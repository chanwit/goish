// Port of go1.25.5 src/net/http/response_test.go — a focused subset.
//
// Elided: TestReadResponse* — takes raw HTTP bytes via ReadResponse,
// not exposed in goish (responses come from hyper inside the Client).
// TestWriteResponseHeaderStatusCodes — wire-format serialization not
// a goish concern. Most cases here reduce to the StatusCode/StatusText
// bookkeeping and the client-response shape.

#![allow(non_snake_case)]
use goish::prelude::*;

test!{ fn TestStatusTextKnown(t) {
    let cases = vec![
        (200, "OK"),
        (201, "Created"),
        (204, "No Content"),
        (301, "Moved Permanently"),
        (302, "Found"),
        (304, "Not Modified"),
        (400, "Bad Request"),
        (401, "Unauthorized"),
        (404, "Not Found"),
        (418, "I'm a teapot"),
        (500, "Internal Server Error"),
        (503, "Service Unavailable"),
    ];
    for (code, want) in cases {
        let got = http::StatusText(code);
        if got != want {
            t.Errorf(Sprintf!("StatusText(%d) = %q, want %q", code, got, want));
        }
    }
}}

test!{ fn TestStatusTextUnknown(t) {
    // An unknown status code returns "".
    let got = http::StatusText(999);
    if got != "" {
        t.Errorf(Sprintf!("StatusText(999) = %q, want empty", got));
    }
}}

test!{ fn TestStatusConstants(t) {
    // Verify canonical values.
    if http::StatusOK != 200 { t.Errorf(Sprintf!("StatusOK = %d", http::StatusOK)); }
    if http::StatusNotFound != 404 { t.Errorf(Sprintf!("StatusNotFound = %d", http::StatusNotFound)); }
    if http::StatusInternalServerError != 500 {
        t.Errorf(Sprintf!("StatusInternalServerError = %d", http::StatusInternalServerError));
    }
}}

test!{ fn TestHeaderOnResponse(t) {
    // Response's Header is a goish::http::Header; verify basic behaviour.
    let mut h = http::Header::new();
    h.Set("Content-Type", "application/json");
    h.Add("X-Multi", "v1");
    h.Add("X-Multi", "v2");

    if h.Get("content-type") != "application/json" {
        t.Errorf(Sprintf!("Case-insensitive Get failed"));
    }
    let values = h.Values("x-multi");
    if values.len() != 2 {
        t.Errorf(Sprintf!("Values len = %d", values.len()));
    }
}}
