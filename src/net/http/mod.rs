// http: Go's net/http package — server + client, Go-shaped call sites.
//
//   Go                                      goish
//   ───────────────────────────────────────  ──────────────────────────────────
//   http.HandleFunc("/", h)                  http::HandleFunc("/", h);
//   http.ListenAndServe(":8080", nil)        http::ListenAndServe(":8080", nil);
//   resp, err := http.Get(url)               let (resp, err) = http::Get(url);
//   resp.Body.Close()                        resp.Body.Close();
//   resp.StatusCode                          resp.StatusCode
//   body, _ := io.ReadAll(resp.Body)         let (body, _) = io::ReadAll(&mut resp.Body);
//
// Backed by hyper 1.x + tokio. The user-facing API is sync (Go's HTTP is
// sync from the caller's perspective); the server drives hyper inside
// the goish tokio runtime, and each request handler is invoked on a
// blocking task so user code can look like straight-line Go.

mod body;
mod client;
mod request;
mod response;
mod server;
mod status;

pub use body::Body;
pub use client::{Client, DefaultClient, Do, Get, Head, NewRequest, NewRequestWithContext, Post, PostForm};
pub use request::Request;
pub use response::{Response, ResponseWriter};
pub use server::{HandleFunc, Handler, HandlerFunc, IntoMux, ListenAndServe, Server, ServeMux};
pub use status::*;

// Common method strings, matching Go's `http.MethodGet` etc.
pub const MethodGet: &str = "GET";
pub const MethodPost: &str = "POST";
pub const MethodPut: &str = "PUT";
pub const MethodDelete: &str = "DELETE";
pub const MethodHead: &str = "HEAD";
pub const MethodPatch: &str = "PATCH";
pub const MethodOptions: &str = "OPTIONS";

/// Shared tokio runtime used by every http server/client call. Reuses the
/// same runtime as `go!{}` so spawning a goroutine from inside a handler
/// keeps working.
pub(crate) fn rt() -> &'static tokio::runtime::Runtime {
    crate::goroutine::runtime()
}

/// Run an async block to completion from a sync caller. Uses `block_on`
/// if no tokio context exists, or `block_in_place` + `Handle::block_on`
/// if we're already inside one (e.g. a handler that calls `http::Get`).
pub(crate) fn block_on<F, T>(fut: F) -> T
where
    F: std::future::Future<Output = T>,
{
    match tokio::runtime::Handle::try_current() {
        Ok(h) => tokio::task::block_in_place(|| h.block_on(fut)),
        Err(_) => rt().block_on(fut),
    }
}
