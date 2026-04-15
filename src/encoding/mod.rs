// encoding: namespace for sub-encoders.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   "encoding/base64"                   goish::encoding::base64
//   "encoding/binary"                   goish::encoding::binary
//   "encoding/csv"                      goish::encoding::csv
//   "encoding/hex"                      goish::encoding::hex
//   "encoding/json"                     goish::encoding::json
//
// Go's `encoding` package itself only defines the `BinaryMarshaler` /
// `TextMarshaler` interfaces. Those are deferred; for now this module is
// purely a namespace.

pub mod base64;
pub mod binary;
pub mod csv;
pub mod hex;
pub mod json;
