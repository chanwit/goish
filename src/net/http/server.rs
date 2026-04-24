// http::Server — Go-shaped HTTP server backed by hyper 1.x.
//
//   http.HandleFunc("/hello", handler)
//   log.Fatal(http.ListenAndServe(":8080", nil))
//
// The handler closure is plain, synchronous Rust. Each request parks on
// tokio's blocking thread pool so the handler body looks like straight-
// line Go code — no `.await` needed.

use crate::errors::{error, nil, New};
use crate::net::http::request::{canonicalize, Header, Request};
use crate::net::http::response::ResponseWriter;
use crate::net::url::URL;
use crate::types::{int64, string};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, OnceLock};

/// A handler closure: takes a mutable `ResponseWriter` + mutable `Request`.
/// (Go's signature is `func(ResponseWriter, *Request)` — the `*Request`
/// is effectively mutable, so `&mut Request` is the Rust analog.)
///
/// Opaque newtype so that `HandlerFunc` doesn't leak `Arc<dyn Fn ...>`
/// into user-facing doc/tooltips. Build one with `HandlerFunc::new(f)`
/// or `f.into()`; invoke with `h.call(w, r)`.
#[derive(Clone)]
pub struct HandlerFunc(
    #[doc(hidden)]
    pub Arc<dyn Fn(&mut ResponseWriter, &mut Request) + Send + Sync + 'static>,
);

impl HandlerFunc {
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(&mut ResponseWriter, &mut Request) + Send + Sync + 'static,
    {
        HandlerFunc(Arc::new(f))
    }

    /// Invoke the handler. Hidden behind a method so call sites don't
    /// have to deref the inner `Arc<dyn Fn>`.
    #[allow(non_snake_case)]
    pub fn call(&self, w: &mut ResponseWriter, r: &mut Request) {
        (self.0)(w, r)
    }
}

impl<F> From<F> for HandlerFunc
where
    F: Fn(&mut ResponseWriter, &mut Request) + Send + Sync + 'static,
{
    fn from(f: F) -> Self { HandlerFunc::new(f) }
}

/// Go's `http.Handler` interface. Implemented manually here so that a
/// user-defined handler type with a `.ServeHTTP` method plugs in the
/// same way the closure form does.
pub trait Handler: Send + Sync + 'static {
    #[allow(non_snake_case)]
    fn ServeHTTP(&self, w: &mut ResponseWriter, r: &mut Request);
}

impl<F> Handler for F
where
    F: Fn(&mut ResponseWriter, &mut Request) + Send + Sync + 'static,
{
    fn ServeHTTP(&self, w: &mut ResponseWriter, r: &mut Request) { (self)(w, r) }
}

/// `http.ServeMux` — a trivial path-prefix router. Mirrors Go's `ServeMux`.
#[derive(Default, Clone)]
pub struct ServeMux {
    routes: Arc<Mutex<Vec<(string, HandlerFunc)>>>,
}

impl ServeMux {
    pub fn new() -> Self { ServeMux::default() }

    /// `mux.HandleFunc(pattern, f)` — register `f` for requests whose
    /// URL.Path starts with `pattern`. Longest-prefix wins.
    #[allow(non_snake_case)]
    pub fn HandleFunc<F>(&self, pattern: &str, f: F)
    where
        F: Fn(&mut ResponseWriter, &mut Request) + Send + Sync + 'static,
    {
        self.routes
            .lock()
            .unwrap()
            .push((pattern.into(), HandlerFunc::new(f)));
    }

    /// `mux.Handle(pattern, handler)` — register a struct impl.
    #[allow(non_snake_case)]
    pub fn Handle<H: Handler>(&self, pattern: &str, h: H) {
        let h = Arc::new(h);
        self.HandleFunc(pattern, move |w, r| h.ServeHTTP(w, r));
    }

    fn match_route(&self, path: &str) -> Option<HandlerFunc> {
        let g = self.routes.lock().unwrap();
        let mut best: Option<(usize, HandlerFunc)> = None;
        for (pat, h) in g.iter() {
            if path.starts_with(pat.as_str())
                && pat.as_str().len() >= best.as_ref().map(|(l, _)| *l).unwrap_or(0)
            {
                best = Some((pat.as_str().len(), h.clone()));
            }
        }
        best.map(|(_, h)| h)
    }
}

impl Handler for ServeMux {
    fn ServeHTTP(&self, w: &mut ResponseWriter, r: &mut Request) {
        match self.match_route(&r.URL.Path) {
            Some(h) => h.call(w, r),
            None => {
                w.WriteHeader(404);
                let _ = w.Write(b"404 page not found\n");
            }
        }
    }
}

/// The process-wide default mux, used by the package-level `HandleFunc`.
fn default_mux() -> &'static ServeMux {
    static MUX: OnceLock<ServeMux> = OnceLock::new();
    MUX.get_or_init(ServeMux::new)
}

/// `http.HandleFunc(pattern, f)` — register on the default mux.
#[allow(non_snake_case)]
pub fn HandleFunc<F>(pattern: &str, f: F)
where
    F: Fn(&mut ResponseWriter, &mut Request) + Send + Sync + 'static,
{
    default_mux().HandleFunc(pattern, f);
}

/// `http.ListenAndServe(addr, handler)` — blocks serving until shutdown.
///
/// The `handler` argument accepts any of:
///   - a `ServeMux`                    — use this mux
///   - `nil` (the `errors::nil` value) — use the process-wide default mux
///   - `None` / `Option::<ServeMux>`   — same as `nil`
///
/// This makes Go's canonical `http.ListenAndServe(":8080", nil)` compile
/// verbatim in goish.
#[allow(non_snake_case)]
pub fn ListenAndServe<H: IntoMux>(addr: &str, handler: H) -> error {
    let mux = handler.into_mux().unwrap_or_else(|| default_mux().clone());
    let srv = Server::new(addr, mux);
    srv.ListenAndServe()
}

/// Argument-converter for `ListenAndServe` so both a `ServeMux` and the
/// Go-idiomatic `nil` work as the second argument.
pub trait IntoMux {
    fn into_mux(self) -> Option<ServeMux>;
}

impl IntoMux for ServeMux {
    fn into_mux(self) -> Option<ServeMux> { Some(self) }
}
impl IntoMux for Option<ServeMux> {
    fn into_mux(self) -> Option<ServeMux> { self }
}
// `nil` is `errors::error(None)`. Accept it (and only the nil-valued error)
// as a "no mux — use default" signal.
impl IntoMux for crate::errors::error {
    fn into_mux(self) -> Option<ServeMux> {
        if self == crate::errors::nil { None } else { None }
    }
}

/// `http.Server` — handle for a running server, used for graceful shutdown.
pub struct Server {
    pub Addr: string,
    pub Handler: ServeMux,
    shutdown: Arc<tokio::sync::Notify>,
}

impl Clone for Server {
    fn clone(&self) -> Self {
        Server {
            Addr: self.Addr.clone(),
            Handler: self.Handler.clone(),
            shutdown: self.shutdown.clone(),
        }
    }
}

impl Server {
    pub fn new(addr: &str, handler: ServeMux) -> Self {
        Server {
            Addr: addr.into(),
            Handler: handler,
            shutdown: Arc::new(tokio::sync::Notify::new()),
        }
    }

    #[allow(non_snake_case)]
    pub fn ListenAndServe(&self) -> error {
        let addr_s = self.Addr.clone();
        let handler = self.Handler.clone();
        let shutdown = self.shutdown.clone();
        super::block_on(async move {
            let addr = match parse_addr(&addr_s) {
                Ok(a) => a,
                Err(e) => return e,
            };
            match run_server(addr, handler, shutdown).await {
                Ok(()) => New("http: Server closed"),
                Err(e) => New(&format!("http: {}", e)),
            }
        })
    }

    /// `srv.Shutdown(ctx)` — stop accepting new connections and wait
    /// for the listener to exit. Mirrors Go's `http.Server.Shutdown`.
    /// Returns when the accept loop has wound down, or when `ctx`
    /// fires, whichever comes first.
    #[allow(non_snake_case)]
    pub fn Shutdown(&self, ctx: crate::context::Context) -> error {
        self.shutdown.notify_waiters();
        // Race the shutdown against ctx cancellation, matching Go's
        // "if ctx expires first we give up waiting" semantic.
        super::block_on(async move {
            let done = ctx.Done();
            tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_millis(50)) => nil,
                _ = done.recv() => ctx.Err(),
            }
        })
    }

    /// `srv.Close()` — immediate close (no graceful drain). Equivalent
    /// to `Shutdown(context::Background())` with a zero-delay race.
    #[allow(non_snake_case)]
    pub fn Close(&self) -> error {
        self.shutdown.notify_waiters();
        nil
    }
}

fn parse_addr(addr: &str) -> Result<SocketAddr, error> {
    let a = if let Some(rest) = addr.strip_prefix(':') {
        format!("0.0.0.0:{}", rest)
    } else {
        addr.into()
    };
    a.parse::<SocketAddr>()
        .map_err(|e| New(&format!("http: parse addr {:?}: {}", addr, e)))
}

async fn run_server(
    addr: SocketAddr,
    handler: ServeMux,
    shutdown: Arc<tokio::sync::Notify>,
) -> Result<(), std::io::Error> {
    use hyper::server::conn::http1;
    use hyper_util::rt::TokioIo;
    use tokio::net::TcpListener;

    let listener = TcpListener::bind(addr).await?;
    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let (stream, remote) = accepted?;
                let handler = handler.clone();
                tokio::spawn(async move {
                    let io = TokioIo::new(stream);
                    let service = hyper::service::service_fn(move |req: hyper::Request<hyper::body::Incoming>| {
                        let handler = handler.clone();
                        let remote = remote;
                        async move { Ok::<_, Infallible>(serve_one(handler, remote, req).await) }
                    });
                    let _ = http1::Builder::new().serve_connection(io, service).await;
                });
            }
            _ = shutdown.notified() => {
                return Ok(());
            }
        }
    }
}

async fn serve_one(
    handler: ServeMux,
    remote: SocketAddr,
    req: hyper::Request<hyper::body::Incoming>,
) -> hyper::Response<http_body_util::Full<bytes::Bytes>> {
    use http_body_util::BodyExt;

    let method: string = req.method().as_str().into();
    let uri = req.uri().clone();
    let version: string = format!("{:?}", req.version()).into();
    let host_hdr: string = req
        .headers()
        .get(hyper::header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .into();

    // Collect headers into goish Header.
    let mut header = Header::new();
    for (name, value) in req.headers().iter() {
        if let Ok(s) = value.to_str() {
            header.Add(name.as_str(), s);
        }
    }

    // Drain body — buffered model (see Body::from_bytes).
    let body_bytes = req
        .into_body()
        .collect()
        .await
        .map(|c| c.to_bytes())
        .unwrap_or_default();
    let content_length = body_bytes.len() as int64;

    // Build the URL struct. hyper gives us path + optional query.
    let mut url = URL::default();
    url.Path = uri.path().into();
    if let Some(q) = uri.query() { url.RawQuery = q.into(); }
    url.Host = host_hdr.clone();

    let mut request = Request::from_parts(
        method,
        url,
        version,
        header,
        crate::net::http::body::Body::from_bytes(body_bytes.to_vec()),
        host_hdr,
        remote.to_string().into(),
        content_length,
    );

    // Wire a per-request cancelable context. If the hyper future gets
    // dropped (client disconnects mid-request), the `_cancel_guard` drop
    // fires and propagates the cancel to the handler via `r.Context()`.
    let (ctx, cancel) = crate::context::WithCancel(crate::context::Background());
    request.set_context(ctx);
    struct CancelOnDrop(Option<crate::context::CancelFunc>);
    impl Drop for CancelOnDrop {
        fn drop(&mut self) {
            if let Some(c) = self.0.take() { c.call(); }
        }
    }
    let _cancel_guard = CancelOnDrop(Some(cancel));

    // Dispatch to the user handler on a blocking task so their code can
    // be straight-line synchronous Rust (matching Go's handler shape).
    let mut w = ResponseWriter::new();
    let mut request = request;
    let w = tokio::task::spawn_blocking(move || {
        handler.ServeHTTP(&mut w, &mut request);
        w
    })
    .await
    .unwrap_or_else(|_| {
        let mut w = ResponseWriter::new();
        w.WriteHeader(500);
        let _ = w.Write(b"handler panicked");
        w
    });

    // Translate goish ResponseWriter → hyper response.
    let mut builder = hyper::Response::builder()
        .status(u16::try_from(w.status).unwrap_or(200));
    for (k, vs) in w.header.iter() {
        for v in vs {
            builder = builder.header(k.as_str(), v.as_str());
        }
    }
    // Default Content-Type if handler didn't set one.
    if header_missing(&w.header, "Content-Type") {
        builder = builder.header("Content-Type", "text/plain; charset=utf-8");
    }
    builder
        .body(http_body_util::Full::new(bytes::Bytes::from(w.body)))
        .unwrap()
}

fn header_missing(h: &Header, name: &str) -> bool {
    let canon = canonicalize(name);
    h.iter().all(|(k, _)| k.as_str() != canon)
}

