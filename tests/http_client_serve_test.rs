// Port of go1.25.5 src/net/http/{client,serve}_test.go — integration subset.
//
// Go's client_test.go spins up httptest.Server and exercises redirects,
// cookies, and transport internals that goish doesn't model. serve_test.go
// does the same but for server-side behaviors. Here we exercise the
// end-to-end stack (real local hyper server + real client) on the
// goish-shaped API: HandleFunc, ListenAndServe, Get, Post, Do, Status
// codes, Header propagation.

#![allow(non_snake_case)]
use goish::prelude::*;
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::Duration;

static PORT_COUNTER: AtomicU16 = AtomicU16::new(37000);

fn next_port() -> u16 {
    PORT_COUNTER.fetch_add(1, Ordering::SeqCst)
}

fn start_server<F>(addr: &str, register: F)
where F: FnOnce() + Send + 'static
{
    let addr_owned = addr.to_string();
    std::thread::spawn(move || {
        register();
        let _ = http::ListenAndServe(&addr_owned, nil);
    });
    std::thread::sleep(Duration::from_millis(200));
}

test!{ fn TestGetSimple(t) {
    let port = next_port();
    let addr = format!("127.0.0.1:{}", port);
    let path = format!("/hello-{}", port);
    let url_ = format!("http://{}{}", addr, path);
    let path_clone = path.clone();
    start_server(&addr, move || {
        http::HandleFunc(&path_clone, move |w, _r| {
            let _ = w.Write(b"hi there");
        });
    });

    let (mut resp, err) = http::Get(&url_);
    if err != nil { t.Fatal(Sprintf!("Get err: %s", err)); }
    if resp.StatusCode != 200 {
        t.Errorf(Sprintf!("StatusCode = %d, want 200", resp.StatusCode));
    }
    let body = resp.Body.String();
    if body != "hi there" {
        t.Errorf(Sprintf!("body = %q", body));
    }
    resp.Body.Close();
}}

test!{ fn TestGetStatusCodes(t) {
    let port = next_port();
    let addr = format!("127.0.0.1:{}", port);
    let url_ = format!("http://{}/teapot", addr);
    start_server(&addr, move || {
        http::HandleFunc("/teapot", move |w, _r| {
            w.WriteHeader(http::StatusTeapot);
            let _ = w.Write(b"nope");
        });
    });

    let (mut resp, err) = http::Get(&url_);
    if err != nil { t.Fatal(Sprintf!("Get err: %s", err)); }
    if resp.StatusCode != 418 {
        t.Errorf(Sprintf!("StatusCode = %d, want 418", resp.StatusCode));
    }
    resp.Body.Close();
}}

test!{ fn TestPostForm(t) {
    let port = next_port();
    let addr = format!("127.0.0.1:{}", port);
    let url_ = format!("http://{}/echo", addr);
    start_server(&addr, move || {
        http::HandleFunc("/echo", move |w, r| {
            let method = r.Method.clone();
            let body = r.Body.String();
            let _ = w.Write(format!("{} body={}", method, body).as_bytes());
        });
    });

    let (mut resp, err) = http::Post(&url_, "text/plain", b"payload-bytes".to_vec());
    if err != nil { t.Fatal(Sprintf!("Post err: %s", err)); }
    let body = resp.Body.String();
    if !strings::HasPrefix(&body, "POST body=") {
        t.Errorf(Sprintf!("post body = %q", body));
    }
    if !strings::Contains(&body, "payload-bytes") {
        t.Errorf(Sprintf!("post body missing payload: %q", body));
    }
    resp.Body.Close();
}}

test!{ fn TestResponseHeader(t) {
    let port = next_port();
    let addr = format!("127.0.0.1:{}", port);
    let url_ = format!("http://{}/hdr", addr);
    start_server(&addr, move || {
        http::HandleFunc("/hdr", move |w, _r| {
            w.Header().Set("X-Custom", "goish");
            let _ = w.Write(b"body");
        });
    });

    let (mut resp, err) = http::Get(&url_);
    if err != nil { t.Fatal(Sprintf!("Get err: %s", err)); }
    let x = resp.Header.Get("X-Custom");
    if x != "goish" {
        t.Errorf(Sprintf!("X-Custom = %q, want goish", x));
    }
    resp.Body.Close();
}}

test!{ fn TestClientGetMissing(t) {
    // A request to an unused port should error.
    let url_ = "http://127.0.0.1:1/definitely-not-running";
    let (_, err) = http::Get(url_);
    if err == nil {
        t.Errorf(Sprintf!("Get to dead port should have errored"));
    }
}}

test!{ fn TestDoCustomMethod(t) {
    let port = next_port();
    let addr = format!("127.0.0.1:{}", port);
    let url_ = format!("http://{}/m", addr);
    start_server(&addr, move || {
        http::HandleFunc("/m", move |w, r| {
            let _ = w.Write(r.Method.as_bytes());
        });
    });

    let (req, err) = http::NewRequest("DELETE", &url_, &[] as &[u8]);
    if err != nil { t.Fatal(Sprintf!("NewRequest: %s", err)); }
    let (mut resp, err) = http::Do(req);
    if err != nil { t.Fatal(Sprintf!("Do: %s", err)); }
    let body = resp.Body.String();
    if body != "DELETE" {
        t.Errorf(Sprintf!("body = %q, want DELETE", body));
    }
    resp.Body.Close();
}}

test!{ fn TestServeMuxPaths(t) {
    let port = next_port();
    let addr = format!("127.0.0.1:{}", port);
    start_server(&addr, move || {
        http::HandleFunc("/a", move |w, _r| { let _ = w.Write(b"A"); });
        http::HandleFunc("/b", move |w, _r| { let _ = w.Write(b"B"); });
    });

    let (mut ra, _) = http::Get(&format!("http://{}/a", addr));
    let (mut rb, _) = http::Get(&format!("http://{}/b", addr));
    let a = ra.Body.String();
    let b = rb.Body.String();
    if a != "A" || b != "B" {
        t.Errorf(Sprintf!("ServeMux routes: a=%q b=%q", a, b));
    }
    ra.Body.Close();
    rb.Body.Close();
}}
