// Port of go1.25.5/src/fmt/stringer_test.go — verifies the Stringer
// interface drives %v formatting. goish uses `fmt::stringer!` to attach
// a String() method to user types; the macro also provides Display so
// `{}` formatting works automatically.

#![allow(non_snake_case)]
use goish::prelude::*;

// Per-type newtypes mirroring Go's TI/TI8/TI16/... test fixtures.
struct TI   (i64);
struct TI8  (i8);
struct TI16 (i16);
struct TI32 (i32);
struct TI64 (i64);
struct TU   (u64);
struct TU8  (u8);
struct TU16 (u16);
struct TU32 (u32);
struct TU64 (u64);
struct TUI  (usize);
struct TF   (f64);
struct TF32 (f32);
struct TF64 (f64);
struct TB   (bool);
struct TS   (&'static str);

fmt::stringer!{ impl TI   { fn String(&self) -> string { Sprintf!("I: %d",   self.0) } } }
fmt::stringer!{ impl TI8  { fn String(&self) -> string { Sprintf!("I8: %d",  self.0) } } }
fmt::stringer!{ impl TI16 { fn String(&self) -> string { Sprintf!("I16: %d", self.0) } } }
fmt::stringer!{ impl TI32 { fn String(&self) -> string { Sprintf!("I32: %d", self.0) } } }
fmt::stringer!{ impl TI64 { fn String(&self) -> string { Sprintf!("I64: %d", self.0) } } }
fmt::stringer!{ impl TU   { fn String(&self) -> string { Sprintf!("U: %d",   self.0) } } }
fmt::stringer!{ impl TU8  { fn String(&self) -> string { Sprintf!("U8: %d",  self.0) } } }
fmt::stringer!{ impl TU16 { fn String(&self) -> string { Sprintf!("U16: %d", self.0) } } }
fmt::stringer!{ impl TU32 { fn String(&self) -> string { Sprintf!("U32: %d", self.0) } } }
fmt::stringer!{ impl TU64 { fn String(&self) -> string { Sprintf!("U64: %d", self.0) } } }
fmt::stringer!{ impl TUI  { fn String(&self) -> string { Sprintf!("UI: %d",  self.0) } } }
fmt::stringer!{ impl TF   { fn String(&self) -> string { Sprintf!("F: %f",   self.0) } } }
fmt::stringer!{ impl TF32 { fn String(&self) -> string { Sprintf!("F32: %f", self.0) } } }
fmt::stringer!{ impl TF64 { fn String(&self) -> string { Sprintf!("F64: %f", self.0) } } }
fmt::stringer!{ impl TB   { fn String(&self) -> string { Sprintf!("B: %t",   self.0) } } }
fmt::stringer!{ impl TS   { fn String(&self) -> string { Sprintf!("S: %q",   self.0) } } }

fn check(t: &testing::T, got: &str, want: &str) {
    if got != want {
        t.Error(Sprintf!("%s != %s", got, want));
    }
}

test!{ fn TestStringer(t) {
    let s = Sprintf!("%v %v %v %v %v", TI(0), TI8(1), TI16(2), TI32(3), TI64(4));
    check(t, &s, "I: 0 I8: 1 I16: 2 I32: 3 I64: 4");
    let s = Sprintf!("%v %v %v %v %v %v", TU(5), TU8(6), TU16(7), TU32(8), TU64(9), TUI(10));
    check(t, &s, "U: 5 U8: 6 U16: 7 U32: 8 U64: 9 UI: 10");
    let s = Sprintf!("%v %v %v", TF(1.0), TF32(2.0), TF64(3.0));
    check(t, &s, "F: 1.000000 F32: 2.000000 F64: 3.000000");
    let s = Sprintf!("%v %v", TB(true), TS("x"));
    check(t, &s, "B: true S: \"x\"");
}}
