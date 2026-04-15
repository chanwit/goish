// net: networking primitives namespace.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   "net/url"                           goish::net::url
//   "net/http"                          goish::net::http
//
// Go's `net` package itself contains Dial/Listen + IP address types;
// those remain a later milestone (see tracking #23).

pub mod http;
pub mod url;
