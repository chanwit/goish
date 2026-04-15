// webserver: a tiny HTTP server/client round-trip in Go-shaped Rust.
//
// Run:
//   cargo run --example webserver
//
// Expected output:
//   server: listening on 127.0.0.1:8080
//   client: GET /hello          -> 200  "hello, world -- you asked for /hello"
//   client: GET /echo?msg=howdy -> 200  "echo: howdy"
//   client: GET /missing        -> 404
//   shutdown after demo round-trips

use goish::prelude::*;

fn main() {
    // Register handlers on the default mux -- same API shape as Go's
    // `http.HandleFunc`. The closure takes `ResponseWriter` + `*Request`
    // (represented here as `&mut` references).
    http::HandleFunc("/hello", |w, r| {
        Fprintf!(w, "hello, world -- you asked for %s", r.URL.Path);
    });

    http::HandleFunc("/echo", |w, r| {
        let msg = r.FormValue("msg");
        if msg.is_empty() {
            w.WriteHeader(http::StatusBadRequest);
            let _ = w.Write(b"missing ?msg=");
            return;
        }
        Fprintf!(w, "echo: %s", msg);
    });

    // Serve in a goroutine so main can drive the client.
    let addr = "127.0.0.1:8080";
    let server = go!{
        // Exact Go form: http.ListenAndServe(":8080", nil) uses the
        // default mux we just registered handlers on.
        let err = http::ListenAndServe(addr, nil);
        if err != nil { Println!("server error:", err); }
    };
    Println!("server: listening on", addr);

    // Give the listener a beat.
    time::Sleep(time::Millisecond * 100i64);

    // Drive three round-trips from main.
    for (path, label) in [
        ("/hello",              "/hello        "),
        ("/echo?msg=howdy",     "/echo?msg=...."),
        ("/missing",            "/missing      "),
    ] {
        let url = Sprintf!("http://%s%s", addr, path);
        let (mut resp, err) = http::Get(&url);
        if err != nil {
            Println!("client error:", err);
            continue;
        }
        let body = resp.Body.String();
        Printf!("client: GET %s -> %3d  %q\n", label, resp.StatusCode, body);
    }

    Println!("shutdown after demo round-trips");
    // Server never returns on its own; drop the handle so the process exits.
    drop(server);
}
