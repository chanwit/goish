// net: networking primitives namespace.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   "net/url"                           goish::net::url
//   "net/http" (v0.5)                   goish::net::http
//
// Go's `net` package itself contains Dial/Listen + IP address types;
// those live under v0.5's networking milestone (see tracking #23).

pub mod url;
