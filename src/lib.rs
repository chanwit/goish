// goish: write Rust using Go idioms.
//
//   use goish::prelude::*;
//
//   fn divide(a: int64, b: int64) -> (int64, error) {
//       if b == 0 {
//           return (0, errors::New("divide by zero"));
//       }
//       (a / b, nil)
//   }
//
//   fn main() {
//       Println!("hello", "world");
//
//       let (q, err) = divide(10, 0);
//       if err != nil {
//           Println!("error:", err);
//       } else {
//           Printf!("q = %d\n", q);
//       }
//   }
//
// Cheat-sheet:
//
//   Go                                goish
//   ───────────────────────────────   ──────────────────────────────────
//   int64 / float64 / byte / rune     int64 / float64 / byte / rune
//   error                             error           (a newtype, not Option)
//   nil                               nil             (zero-value error)
//   if err != nil { ... }             if err != nil { ... }
//   fmt.Println(a, b)                 fmt::Println!(a, b)
//   fmt.Printf("%d", n)               fmt::Printf!("%d", n)
//   fmt.Sprintf("%s", s)              fmt::Sprintf!("%s", s)
//   fmt.Fprintf(w, "%d", n)           fmt::Fprintf!(w, "%d", n)
//   fmt.Errorf("bad: %s", e)          fmt::Errorf!("bad: %s", e)
//   errors.New("msg")                 errors::New("msg")
//   errors.Wrap(err, "msg")           errors::Wrap(err, "msg")
//   errors.Is(err, ErrX)              errors::Is(&err, &ErrX)
//   errors.Unwrap(err)                errors::Unwrap(err)

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
// Our doc comments use Go-syntax cheat-sheets with `[brackets]` and `<T>`
// that rustdoc would otherwise interpret as intra-doc-links / HTML tags.
#![allow(rustdoc::broken_intra_doc_links)]
#![allow(rustdoc::invalid_html_tags)]

pub mod base64;
pub mod binary;
pub mod bufio;
pub mod bytes;
pub mod chan;
pub mod consts;
pub mod container;
pub mod context;
pub mod crypto;
pub mod csv;
pub mod defer;
pub mod errors;
pub mod exec;
pub mod filepath;
pub mod flag;
pub mod fmt;
pub mod goroutine;
pub mod hash;
pub mod hex;
pub mod io;
pub mod json;
pub mod log;
pub mod math;
pub mod mime;
pub mod os;
pub mod path;
pub mod rand;
pub mod range;
pub mod regexp;
pub mod runtime;
pub mod sort;
pub mod strconv;
pub mod strings;
pub mod sync;
pub mod time;
pub mod types;
pub mod unicode;
pub mod url;
pub mod utf8;

// Make Go primitive type names visible at the crate root so macros that
// say `$crate::int` resolve correctly.
pub use crate::types::*;

pub mod prelude {
    pub use crate::base64;
    pub use crate::binary;
    pub use crate::bufio;
    pub use crate::bytes;
    pub use crate::chan::Chan;
    pub use crate::container;
    pub use crate::context;
    pub use crate::crypto;
    pub use crate::csv;
    pub use crate::errors::{self, error, nil};
    pub use crate::exec;
    pub use crate::filepath;
    pub use crate::flag;
    pub use crate::fmt;
    pub use crate::hash;
    pub use crate::hex;
    pub use crate::io;
    pub use crate::json;
    pub use crate::log;
    pub use crate::math;
    pub use crate::mime;
    pub use crate::os;
    pub use crate::path;
    pub use crate::rand;
    pub use crate::regexp;
    pub use crate::runtime;
    pub use crate::sort;
    pub use crate::strconv;
    pub use crate::strings;
    pub use crate::sync;
    pub use crate::time;
    pub use crate::unicode;
    pub use crate::url;
    pub use crate::utf8;
    pub use crate::types::*;
    pub use crate::goroutine::Goroutine;
    pub use crate::{
        Errorf, Fprintf, Printf, Println, Sprintf,
        append, chan, const_block, defer, delete, go, len, make, map, range, slice, stringer,
    };
}
