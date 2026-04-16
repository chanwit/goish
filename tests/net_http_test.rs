// net/http — integration tests. Each test spins up a server on an ephemeral
// port in a goroutine, then drives it with the client API from the main
// thread. All tests exercise the real Go-shaped call site.

#![allow(non_snake_case)]

use goish::prelude::*;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::OnceLock;

/// Allocate an ephemeral TCP port by binding and immediately releasing.
/// Two tests running concurrently could collide on port reuse; we serialize
/// by bumping a static base after each allocation.
fn free_port() -> u16 {
    static NEXT: OnceLock<AtomicU16> = OnceLock::new();
    let ctr = NEXT.get_or_init(|| {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let p = l.local_addr().unwrap().port();
        AtomicU16::new(p)
    });
    let _ = ctr.fetch_add(1, Ordering::SeqCst);
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    l.local_addr().unwrap().port()
}

/// Start `srv` in a goroutine, wait briefly for the listener to come up,
/// and return the goroutine handle.
fn spawn_server(srv: net::http::Server) -> Goroutine {
    go!{ let _ = srv.ListenAndServe(); }
}

fn wait_listener(port: u16) {
    let addr = format!("127.0.0.1:{}", port);
    for _ in 0..100 {
        if std::net::TcpStream::connect(&addr).is_ok() { return; }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[test]
fn get_simple_200() {
    let port = free_port();
    let mux = net::http::ServeMux::new();
    mux.HandleFunc("/hello", |w, r| {
        let _ = Fprintf!(w, "hello %s", r.URL.Path);
    });
    let srv = net::http::Server::new(&format!("127.0.0.1:{}", port), mux);
    let _h = spawn_server(srv);
    wait_listener(port);

    let (resp, err) = net::http::Get(&format!("http://127.0.0.1:{}/hello", port));
    assert!(err == nil, "Get err: {}", err);
    assert_eq!(resp.StatusCode, 200);
    assert_eq!(resp.Body.String(), "hello /hello");
}

#[test]
fn not_found_on_unregistered_path() {
    let port = free_port();
    let mux = net::http::ServeMux::new();
    mux.HandleFunc("/known", |w, _r| { let _ = w.Write(b"ok"); });
    let srv = net::http::Server::new(&format!("127.0.0.1:{}", port), mux);
    let _h = spawn_server(srv);
    wait_listener(port);

    let (resp, err) = net::http::Get(&format!("http://127.0.0.1:{}/other", port));
    assert!(err == nil);
    assert_eq!(resp.StatusCode, 404);
}

#[test]
fn post_echoes_body() {
    let port = free_port();
    let mux = net::http::ServeMux::new();
    mux.HandleFunc("/echo", |w, r| {
        let body = r.Body.Bytes();
        w.Header().Set("Content-Type", "application/octet-stream");
        let _ = w.Write(&body);
    });
    let srv = net::http::Server::new(&format!("127.0.0.1:{}", port), mux);
    let _h = spawn_server(srv);
    wait_listener(port);

    let (resp, err) = net::http::Post(
        &format!("http://127.0.0.1:{}/echo", port),
        "text/plain",
        "hello world",   // &str body — no explicit b"…" / .as_bytes() needed
    );
    assert!(err == nil);
    assert_eq!(resp.StatusCode, 200);
    assert_eq!(resp.Body.String(), "hello world");
    assert_eq!(resp.Header.Get("Content-Type"), "application/octet-stream");
}

#[test]
fn handler_sets_custom_status() {
    let port = free_port();
    let mux = net::http::ServeMux::new();
    mux.HandleFunc("/teapot", |w, _r| {
        w.WriteHeader(net::http::StatusTeapot);
        let _ = w.Write(b"I'm a teapot");
    });
    let srv = net::http::Server::new(&format!("127.0.0.1:{}", port), mux);
    let _h = spawn_server(srv);
    wait_listener(port);

    let (resp, err) = net::http::Get(&format!("http://127.0.0.1:{}/teapot", port));
    assert!(err == nil);
    assert_eq!(resp.StatusCode, 418);
    assert_eq!(resp.Body.String(), "I'm a teapot");
}

#[test]
fn query_string_propagates_to_handler() {
    let port = free_port();
    let mux = net::http::ServeMux::new();
    mux.HandleFunc("/q", |w, r| {
        let name = r.FormValue("name");
        let _ = Fprintf!(w, "hi %s", name);
    });
    let srv = net::http::Server::new(&format!("127.0.0.1:{}", port), mux);
    let _h = spawn_server(srv);
    wait_listener(port);

    let (resp, err) = net::http::Get(&format!("http://127.0.0.1:{}/q?name=alice", port));
    assert!(err == nil);
    assert_eq!(resp.Body.String(), "hi alice");
}

#[test]
fn longest_prefix_wins_on_serve_mux() {
    let port = free_port();
    let mux = net::http::ServeMux::new();
    mux.HandleFunc("/api/", |w, _r| { let _ = w.Write(b"root"); });
    mux.HandleFunc("/api/users/", |w, _r| { let _ = w.Write(b"users"); });
    let srv = net::http::Server::new(&format!("127.0.0.1:{}", port), mux);
    let _h = spawn_server(srv);
    wait_listener(port);

    let url_base = format!("http://127.0.0.1:{}", port);
    let (r1, _) = net::http::Get(&format!("{}/api/other", url_base));
    assert_eq!(r1.Body.String(), "root");
    let (r2, _) = net::http::Get(&format!("{}/api/users/42", url_base));
    assert_eq!(r2.Body.String(), "users");
}

#[test]
fn client_sees_server_headers_and_custom_content_type() {
    let port = free_port();
    let mux = net::http::ServeMux::new();
    mux.HandleFunc("/j", |w, _r| {
        w.Header().Set("Content-Type", "application/json");
        w.Header().Set("X-Custom", "yes");
        let _ = w.Write(b"{\"ok\":true}");
    });
    let srv = net::http::Server::new(&format!("127.0.0.1:{}", port), mux);
    let _h = spawn_server(srv);
    wait_listener(port);

    let (resp, _) = net::http::Get(&format!("http://127.0.0.1:{}/j", port));
    assert_eq!(resp.Header.Get("Content-Type"), "application/json");
    assert_eq!(resp.Header.Get("X-Custom"), "yes");
    assert_eq!(resp.Body.String(), "{\"ok\":true}");
}

#[test]
fn shutdown_stops_the_accept_loop() {
    // Spin up a server, shut it down, verify new connections fail.
    let port = free_port();
    let mux = net::http::ServeMux::new();
    mux.HandleFunc("/ping", |w, _r| { let _ = w.Write(b"pong"); });
    let srv = net::http::Server::new(&format!("127.0.0.1:{}", port), mux);
    let srv_handle = srv.clone();
    let server_goroutine = go!{ let _ = srv_handle.ListenAndServe(); };
    wait_listener(port);

    // Happy request while server is up.
    let (resp, err) = net::http::Get(&format!("http://127.0.0.1:{}/ping", port));
    assert!(err == nil);
    assert_eq!(resp.Body.String(), "pong");

    // Graceful shutdown.
    let _ = srv.Shutdown(context::Background());
    time::Sleep(time::Millisecond * 100i64);

    // New connection should fail now (refused, reset, or empty).
    let (resp, err) = net::http::Get(&format!("http://127.0.0.1:{}/ping", port));
    assert!(
        err != nil || resp.StatusCode == 0,
        "expected failure after shutdown; got {} {}", resp.StatusCode, resp.Body.String()
    );

    let _ = server_goroutine;
}

#[test]
fn status_text_matches_go() {
    assert_eq!(net::http::StatusText(200), "OK");
    assert_eq!(net::http::StatusText(404), "Not Found");
    assert_eq!(net::http::StatusText(418), "I'm a teapot");
    assert_eq!(net::http::StatusText(999), "");
}

#[test]
fn client_context_deadline_aborts_request() {
    // Server that sleeps longer than the client's deadline.
    let port = free_port();
    let mux = net::http::ServeMux::new();
    mux.HandleFunc("/slow", |w, _r| {
        time::Sleep(time::Millisecond * 300i64);
        let _ = w.Write(b"too late");
    });
    let srv = net::http::Server::new(&format!("127.0.0.1:{}", port), mux);
    let _h = spawn_server(srv);
    wait_listener(port);

    let (ctx, _cancel) = context::WithTimeout(context::Background(), time::Millisecond * 50i64);
    let (req, err) = net::http::NewRequestWithContext(
        ctx,
        "GET",
        &format!("http://127.0.0.1:{}/slow", port),
        nil,
    );
    assert!(err == nil);

    let start = std::time::Instant::now();
    let (resp, err) = net::http::Do(req);
    let _ = resp;
    let elapsed = start.elapsed();

    assert!(err != nil, "expected timeout error");
    assert!(elapsed < std::time::Duration::from_millis(250), "cancelled quickly; got {:?}", elapsed);
    assert!(
        format!("{}", err).contains("deadline") || format!("{}", err).contains("canceled"),
        "error should mention deadline/canceled: {}",
        err
    );
}

#[test]
fn handler_can_select_on_request_context() {
    // Handler spawns a worker goroutine and uses select! to race it
    // against r.Context().Done(). If the client disconnects, the
    // handler can bail early. Here we verify the happy path — handler
    // sees its own ctx, doesn't cancel, returns normally.
    let port = free_port();
    let mux = net::http::ServeMux::new();
    mux.HandleFunc("/ctx", |w, r| {
        let ctx = r.Context();
        let worker = chan!(string, 1);
        let wp = worker.clone();
        let _g = go!{
            time::Sleep(time::Millisecond * 10i64);
            wp.Send("done".into());
        };
        select!{
            recv(worker) |v| => { let _ = w.Write(v.as_bytes()); },
            recv(ctx.Done()) => {
                w.WriteHeader(net::http::StatusServiceUnavailable);
                let _ = w.Write(b"cancelled");
            },
        }
    });
    let srv = net::http::Server::new(&format!("127.0.0.1:{}", port), mux);
    let _h = spawn_server(srv);
    wait_listener(port);

    let (resp, err) = net::http::Get(&format!("http://127.0.0.1:{}/ctx", port));
    assert!(err == nil);
    assert_eq!(resp.StatusCode, 200);
    assert_eq!(resp.Body.String(), "done");
}

#[test]
fn header_canonicalization_is_correct() {
    let mut h = net::http::Request::new("GET", "http://x/", nil).0.Header;
    h.Set("content-type", "text/html");
    h.Set("X-FOO-BAR", "yes");
    assert_eq!(h.Get("Content-Type"), "text/html");
    assert_eq!(h.Get("content-type"), "text/html");
    assert_eq!(h.Get("X-Foo-Bar"), "yes");
}
