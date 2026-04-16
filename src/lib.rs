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
//   errors.New("msg")                 errors::New("msg")

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
// Our doc comments use Go-syntax cheat-sheets with `[brackets]` and `<T>`
// that rustdoc would otherwise interpret as intra-doc-links / HTML tags.
#![allow(rustdoc::broken_intra_doc_links)]
#![allow(rustdoc::invalid_html_tags)]

// Make `::goish::…` resolve to `crate::…` inside our own lib, so paths the
// `select!` proc macro emits (e.g. `::goish::__flume::Selector`) work both
// in downstream crates and in goish's own integration tests.
extern crate self as goish;

// ── Top-level Go packages ──────────────────────────────────────────────

pub mod bufio;
pub mod bytes;
pub mod cmp;
pub mod container;        // container/{list,heap,ring}
pub mod context;
pub mod crypto;           // crypto/{md5,sha1,sha256}
pub mod encoding;         // encoding/{base64,binary,csv,hex,json}
pub mod errors;
pub mod flag;
pub mod fmt;
pub mod hash;             // hash/{crc32,fnv}
pub mod html;
pub mod io;
pub mod iter;
pub mod log;
pub mod maps;
pub mod math;             // math + math/rand
pub mod mime;
pub mod net;              // net/url (net/http in v0.5)
pub mod os;               // os + os/exec
pub mod path;             // path + path/filepath
pub mod regexp;
pub mod runtime;
pub mod slices;
pub mod sort;
pub mod strconv;
pub mod strings;
pub mod sync;             // sync + sync/atomic
pub mod testing;
pub mod text;             // text/{tabwriter,scanner,template}
pub mod time;
pub mod unicode;          // unicode + unicode/utf8

// ── Built-ins (Go keyword / language-level things) ─────────────────────

#[doc(hidden)]
pub mod __macros {
    pub use goish_macros::rewrite_go_body;
}

/// Re-export of the `flume` crate so `select!` (proc-macro) can reference
/// `$crate::__flume::Selector` without forcing users to add flume to
/// their own `Cargo.toml`. Not a public API.
#[doc(hidden)]
pub use flume as __flume;

/// `select!{ ... }` — Go's `select` statement (proc-macro form).
///
/// See `src/chan/mod.rs` for the semantic contract; the macro is
/// defined in the `goish-macros` proc-macro crate.
pub use goish_macros::select;

/// Re-export of the `inventory` crate so the `test!` macro in user crates
/// can submit registrations without the user depending on `inventory`
/// directly. Not a public API.
#[doc(hidden)]
pub mod __goish_inventory {
    pub use inventory::submit;
}

pub mod chan;
pub mod consts;
pub mod defer;
pub mod gostring;
pub mod goroutine;
pub mod range;
pub mod types;
#[doc(hidden)]
pub mod struct_macro;
pub use struct_macro::__goish_into_string;

// Backward-compat flat re-exports — keeps v0.3 import paths working. New
// code should prefer the Go-import-path form (`encoding::base64`, `math::rand`).
pub use encoding::base64;
pub use encoding::binary;
pub use encoding::csv;
pub use encoding::hex;
pub use encoding::json;
pub use math::rand;
pub use net::http;
pub use net::url;
pub use os::exec;
pub use path::filepath;
pub use unicode::utf8;

// Make Go primitive type names visible at the crate root so macros that
// say `$crate::int` resolve correctly.
pub use crate::types::*;

pub mod prelude {
    pub use crate::bufio;
    pub use crate::bytes;
    pub use crate::chan::Chan;
    pub use crate::cmp;
    pub use crate::container;
    pub use crate::context;
    pub use crate::crypto;
    pub use crate::encoding;
    pub use crate::errors::{self, error, nil};
    pub use crate::flag;
    pub use crate::fmt;
    pub use crate::hash;
    pub use crate::html;
    pub use crate::io;
    pub use crate::io::{Reader as _GoishIoReader, Writer as _GoishIoWriter,
                        Closer as _GoishIoCloser, Seeker as _GoishIoSeeker,
                        ReaderAt as _GoishIoReaderAt, WriterAt as _GoishIoWriterAt};
    pub use crate::iter;
    pub use crate::log;
    pub use crate::maps;
    pub use crate::math;
    pub use crate::mime;
    pub use crate::net;
    pub use crate::os;
    pub use crate::path;
    pub use crate::regexp;
    pub use crate::runtime;
    pub use crate::slices;
    pub use crate::sort;
    pub use crate::strconv;
    pub use crate::strings;
    pub use crate::sync;
    pub use crate::testing;
    pub use crate::text;
    pub use crate::time;
    pub use crate::unicode;

    // v0.3-compat flat names — call site keeps the short form users expect.
    pub use crate::base64;
    pub use crate::binary;
    pub use crate::csv;
    pub use crate::exec;
    pub use crate::filepath;
    pub use crate::hex;
    pub use crate::http;
    pub use crate::json;
    pub use crate::rand;
    pub use crate::url;
    pub use crate::utf8;

    pub use crate::types::*;
    pub use crate::goroutine::Goroutine;
    pub use crate::{
        Cookie, Errorf, Fprintf, MailAddress, Printf, Println, Sprintf,
        append, benchmark, cat, chan, close, const_block, defer, delete, go, len, make, map,
        range, recover, select, slice, stringer, Struct, test, test_h, test_main,
    };
}
