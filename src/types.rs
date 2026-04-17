// types: Go's built-in primitive type names, mapped to Rust types.
//
//   Go            goish
//   ───────────   ──────────────
//   int           int          (i64 — Go's int is platform-sized; we pick 64)
//   int8          int8         (i8)
//   int16         int16        (i16)
//   int32         int32        (i32)
//   int64         int64        (i64)
//   uint          uint         (u64)
//   uint8         uint8        (u8)
//   uint16        uint16       (u16)
//   uint32        uint32       (u32)
//   uint64        uint64       (u64)
//   float32       float32      (f32)
//   float64       float64      (f64)
//   byte          byte         (u8)
//   rune          rune         (i32 — Go's rune is an alias for int32)
//   string        string       (alias for std::string::String)

pub type int = i64;
pub type int8 = i8;
pub type int16 = i16;
pub type int32 = i32;
pub type int64 = i64;

pub type uint = u64;
pub type uint8 = u8;
pub type uint16 = u16;
pub type uint32 = u32;
pub type uint64 = u64;

pub type float32 = f32;
pub type float64 = f64;

pub type byte = u8;
pub type rune = i32;

pub type string = crate::gostring::GoString;

// Go: []T  →  goish: slice<T>
//
// Newtype around Vec<T> so we can impl Index<i64> (Go's `int`) — the orphan
// rule blocks adding foreign traits to foreign Vec directly. All the Vec
// API stays reachable via Deref/DerefMut, and From<Vec<T>> / Into<Vec<T>>
// keep Vec-returning stdlib calls fluent.
pub use crate::_slice::slice;

// Go: map[K]V  →  goish: map<K, V>
pub use crate::_map::map;

/// Three forms — pick the one closest to your Go original:
///
///   slice!([]string{"a", "b"})    // Go-shaped:   []T{...}
///   slice![string; "a", "b"]      // typed:       T; values
///   slice![1, 2, 3]               // untyped:     just values (vec! alias)
///
/// The typed and Go-shaped forms call `.into()` on each element so
/// `&str` literals become `string`, `i32` literals widen to `int64`, etc.
#[macro_export]
macro_rules! slice {
    // Go-shaped, primitive numeric types — use `as` cast so bare literals
    // widen/truncate naturally (Go: `[]uint64{10, 500}` works with untyped
    // int literals; Rust wouldn't otherwise accept i32 → u64 via From).
    ([] int    { $($x:expr),* $(,)? }) => { $crate::_slice::slice(vec![ $( ($x) as $crate::types::int ),*    ]) };
    ([] int8   { $($x:expr),* $(,)? }) => { $crate::_slice::slice(vec![ $( ($x) as $crate::types::int8 ),*   ]) };
    ([] int16  { $($x:expr),* $(,)? }) => { $crate::_slice::slice(vec![ $( ($x) as $crate::types::int16 ),*  ]) };
    ([] int32  { $($x:expr),* $(,)? }) => { $crate::_slice::slice(vec![ $( ($x) as $crate::types::int32 ),*  ]) };
    ([] int64  { $($x:expr),* $(,)? }) => { $crate::_slice::slice(vec![ $( ($x) as $crate::types::int64 ),*  ]) };
    ([] uint   { $($x:expr),* $(,)? }) => { $crate::_slice::slice(vec![ $( ($x) as $crate::types::uint ),*   ]) };
    ([] uint8  { $($x:expr),* $(,)? }) => { $crate::_slice::slice(vec![ $( ($x) as $crate::types::uint8 ),*  ]) };
    ([] uint16 { $($x:expr),* $(,)? }) => { $crate::_slice::slice(vec![ $( ($x) as $crate::types::uint16 ),* ]) };
    ([] uint32 { $($x:expr),* $(,)? }) => { $crate::_slice::slice(vec![ $( ($x) as $crate::types::uint32 ),* ]) };
    ([] uint64 { $($x:expr),* $(,)? }) => { $crate::_slice::slice(vec![ $( ($x) as $crate::types::uint64 ),* ]) };
    ([] float32 { $($x:expr),* $(,)? }) => { $crate::_slice::slice(vec![ $( ($x) as $crate::types::float32 ),* ]) };
    ([] float64 { $($x:expr),* $(,)? }) => { $crate::_slice::slice(vec![ $( ($x) as $crate::types::float64 ),* ]) };
    ([] byte   { $($x:expr),* $(,)? }) => { $crate::_slice::slice(vec![ $( ($x) as $crate::types::byte ),*   ]) };
    ([] rune   { $($x:expr),* $(,)? }) => { $crate::_slice::slice(vec![ $( ($x) as $crate::types::rune ),*   ]) };

    // Go-shaped, generic:  slice!([]T{a, b, c}) — .into() for &str→string,
    // struct literal identity, and anything with From<_>.
    ([] $t:ty { $($x:expr),* $(,)? }) => {
        {
            let v: $crate::types::slice<$t> =
                vec![ $( <$t as ::std::convert::From<_>>::from($x) ),* ].into();
            v
        }
    };
    // Typed semicolon form
    ($t:ty ; $($x:expr),* $(,)?) => {
        {
            let v: $crate::types::slice<$t> =
                vec![ $( <$t as ::std::convert::From<_>>::from($x) ),* ].into();
            v
        }
    };
    // Untyped
    ($($x:expr),* $(,)?) => {
        {
            let v: $crate::types::slice<_> = vec![ $($x),* ].into();
            v
        }
    };
}

/// `range!(xs)` — Go's `for i, v := range xs` as a Rust iterator.
///
///   // Go:    for i, v := range xs { body }
///   // goish: for (i, v) in range!(xs) { body }
///
///   // Go:    for _, v := range xs { body }
///   // goish: for (_, v) in range!(xs) { body }
///
/// Works on slices, arrays, maps, and strings. Uses `.iter().enumerate()`
/// for slices/arrays, `.iter()` for maps, and `.chars().enumerate()` for
/// string slices.
#[macro_export]
macro_rules! range {
    ($iter:expr) => {
        $crate::range::RangeIter::range(&$iter)
    };
}

/// `delete!(m, k)` — Go's `delete(m, k)` builtin for maps.
///
///   delete!(m, "key")       // &str literal — works directly
///   delete!(m, &my_string)  // owned String — pass &  (HashMap.remove takes &Q)
///   delete!(m, &42)         // owned int    — same
///
/// Silent no-op if the key isn't present (matches Go).
#[macro_export]
macro_rules! delete {
    ($m:expr, $k:expr) => {
        { let _ = ($m).remove($k); }
    };
}

/// `IntNewtype!(ID = uint64)` — Go's `type ID uint64`.
///
/// Generates a tuple struct `pub struct ID(pub uint64)` with the derives
/// you'd expect (Clone/Copy/Debug/Default/PartialEq/Eq/Hash/PartialOrd/Ord)
/// plus `From<i32/i64/u32/u64/usize>` via `as` cast so `slice!([]ID{10, 20})`
/// accepts bare literals the way Go does, and `From<ID>` for the
/// underlying integer so `uint64::from(id)` works.
///
/// Does NOT generate `Display` / a `String()` method — wrap the result
/// with `stringer!` to layer those on:
///
/// ```ignore
/// IntNewtype!(ID = uint64);
/// stringer! {
///     impl ID {
///         fn String(&self) -> string { strconv::FormatUint(self.0, 16) }
///     }
/// }
/// ```
///
/// Only valid when the underlying type is a primitive integer — the
/// generated `as` casts require it.
#[macro_export]
macro_rules! IntNewtype {
    ($name:ident = $t:ty) => {
        #[allow(non_camel_case_types)]
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub $t);

        impl ::std::convert::From<i32>   for $name { fn from(x: i32)   -> Self { $name(x as $t) } }
        impl ::std::convert::From<i64>   for $name { fn from(x: i64)   -> Self { $name(x as $t) } }
        impl ::std::convert::From<u32>   for $name { fn from(x: u32)   -> Self { $name(x as $t) } }
        impl ::std::convert::From<u64>   for $name { fn from(x: u64)   -> Self { $name(x as $t) } }
        impl ::std::convert::From<usize> for $name { fn from(x: usize) -> Self { $name(x as $t) } }
        impl ::std::convert::From<$name> for $t    { fn from(x: $name) -> $t    { x.0 } }
    };
}

/// `SliceNewtype!(IDSlice = ID)` — Go's `type IDSlice []ID`.
///
/// Generates `pub struct IDSlice(pub slice<ID>)` with Deref/DerefMut to
/// slice<ID> (so all slice methods flow through), From<slice<ID>> /
/// From<Vec<ID>> / From<IDSlice> for slice<ID>, IntoIterator, and the
/// usual derives (Clone/Debug/Default; PartialEq/Eq when the element
/// supports it — add `#[derive(...)]` manually if you need more).
#[macro_export]
macro_rules! SliceNewtype {
    ($name:ident = $elem:ty) => {
        #[allow(non_camel_case_types)]
        #[derive(Clone, Debug, Default, PartialEq, Eq)]
        pub struct $name(pub $crate::types::slice<$elem>);

        impl ::std::ops::Deref for $name {
            type Target = $crate::types::slice<$elem>;
            fn deref(&self) -> &Self::Target { &self.0 }
        }
        impl ::std::ops::DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
        }

        impl ::std::convert::From<$crate::types::slice<$elem>> for $name {
            fn from(v: $crate::types::slice<$elem>) -> Self { $name(v) }
        }
        impl ::std::convert::From<::std::vec::Vec<$elem>> for $name {
            fn from(v: ::std::vec::Vec<$elem>) -> Self { $name(v.into()) }
        }
        impl ::std::convert::From<$name> for $crate::types::slice<$elem> {
            fn from(x: $name) -> $crate::types::slice<$elem> { x.0 }
        }

        impl ::std::iter::IntoIterator for $name {
            type Item = $elem;
            type IntoIter = ::std::vec::IntoIter<$elem>;
            fn into_iter(self) -> Self::IntoIter { self.0.into_vec().into_iter() }
        }
        impl<'a> ::std::iter::IntoIterator for &'a $name {
            type Item = &'a $elem;
            type IntoIter = ::std::slice::Iter<'a, $elem>;
            fn into_iter(self) -> Self::IntoIter { self.0.iter() }
        }
        impl<'a> ::std::iter::IntoIterator for &'a mut $name {
            type Item = &'a mut $elem;
            type IntoIter = ::std::slice::IterMut<'a, $elem>;
            fn into_iter(self) -> Self::IntoIter { self.0.iter_mut() }
        }
    };
}

/// `Type!(Name = <shape>)` — Go's `type Name <shape>` decl.
///
/// Dispatches on the shape of the RHS:
///
///   Type!(ID = uint64)         // int newtype  → IntNewtype!
///   Type!(IDSlice = []ID)      // slice newtype → SliceNewtype!
///   Type!(X = SomeStruct)      // generic newtype fallback
///
/// Compose with `stringer!` to layer on `String()` + `Display`. See
/// examples/id_newtype_demo.rs for the full Go → goish port pattern.
#[macro_export]
macro_rules! Type {
    // Slice: `type IDSlice []ID`
    ($name:ident = [ ] $elem:ty) => { $crate::SliceNewtype!($name = $elem); };

    // Integer primitive names — dispatch to IntNewtype.
    ($name:ident = int)    => { $crate::IntNewtype!($name = $crate::types::int); };
    ($name:ident = int8)   => { $crate::IntNewtype!($name = $crate::types::int8); };
    ($name:ident = int16)  => { $crate::IntNewtype!($name = $crate::types::int16); };
    ($name:ident = int32)  => { $crate::IntNewtype!($name = $crate::types::int32); };
    ($name:ident = int64)  => { $crate::IntNewtype!($name = $crate::types::int64); };
    ($name:ident = uint)   => { $crate::IntNewtype!($name = $crate::types::uint); };
    ($name:ident = uint8)  => { $crate::IntNewtype!($name = $crate::types::uint8); };
    ($name:ident = uint16) => { $crate::IntNewtype!($name = $crate::types::uint16); };
    ($name:ident = uint32) => { $crate::IntNewtype!($name = $crate::types::uint32); };
    ($name:ident = uint64) => { $crate::IntNewtype!($name = $crate::types::uint64); };
    ($name:ident = byte)   => { $crate::IntNewtype!($name = $crate::types::byte); };
    ($name:ident = rune)   => { $crate::IntNewtype!($name = $crate::types::rune); };

    // Generic fallback: `type X SomeStruct` — plain tuple struct newtype
    // with the minimum viable plumbing. Users derive more if they need it.
    ($name:ident = $t:ty) => {
        #[allow(non_camel_case_types)]
        #[derive(Clone, Debug, Default)]
        pub struct $name(pub $t);

        impl ::std::convert::From<$t> for $name {
            fn from(x: $t) -> Self { $name(x) }
        }
        impl ::std::convert::From<$name> for $t {
            fn from(x: $name) -> $t { x.0 }
        }
    };
}

/// `Enum!(Name)` — Go's `type Name string` for named-constant enums.
///
/// Generates a `&'static str` newtype that's `const`-constructible, `Copy`,
/// and pattern-matchable — the natural Rust equivalent of Go's string-enum
/// pattern:
///
/// ```ignore
/// // Go:
/// type Status string
/// const ( StatusOK Status = "ok"; StatusFail Status = "fail" )
///
/// // Goish:
/// Enum!(Status);
/// const StatusOK: Status = Status("ok");
/// const StatusFail: Status = Status("fail");
/// ```
///
/// Includes Display (prints the inner string) so `Sprintf!("%v", s)` works.
/// Internal helper — int-backed enum body. Used by `Enum!` arms.
#[macro_export]
#[doc(hidden)]
macro_rules! __goish_int_enum {
    ($name:ident, $t:ty) => {
        #[allow(non_camel_case_types)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
        pub struct $name(pub $t);

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

#[macro_export]
macro_rules! Enum {
    // String enum (default): Enum!(Status)
    //   Go: type Status string
    ($name:ident) => {
        #[allow(non_camel_case_types)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
        pub struct $name(pub &'static str);

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(self.0)
            }
        }
    };

    // Int-backed enums: Enum!(Priority = int)
    //   Go: type Priority int
    ($name:ident = int)     => { $crate::__goish_int_enum!($name, $crate::types::int); };
    ($name:ident = int8)    => { $crate::__goish_int_enum!($name, $crate::types::int8); };
    ($name:ident = int16)   => { $crate::__goish_int_enum!($name, $crate::types::int16); };
    ($name:ident = int32)   => { $crate::__goish_int_enum!($name, $crate::types::int32); };
    ($name:ident = int64)   => { $crate::__goish_int_enum!($name, $crate::types::int64); };
    ($name:ident = uint)    => { $crate::__goish_int_enum!($name, $crate::types::uint); };
    ($name:ident = uint8)   => { $crate::__goish_int_enum!($name, $crate::types::uint8); };
    ($name:ident = uint16)  => { $crate::__goish_int_enum!($name, $crate::types::uint16); };
    ($name:ident = uint32)  => { $crate::__goish_int_enum!($name, $crate::types::uint32); };
    ($name:ident = uint64)  => { $crate::__goish_int_enum!($name, $crate::types::uint64); };
    ($name:ident = byte)    => { $crate::__goish_int_enum!($name, $crate::types::byte); };
    ($name:ident = rune)    => { $crate::__goish_int_enum!($name, $crate::types::rune); };
}

/// `copy!(dst, src)` — Go's `copy(dst, src) int` builtin.
///
/// Copies `min(len(dst), len(src))` elements from src into dst, returning
/// the number copied as `int`. Works with any `&mut [T]` / `&[T]` pair
/// where `T: Copy` (bytes, ints, copy structs, etc.):
///
///   let mut buf = make!([]byte, 32);
///   let n = copy!(&mut buf, &src);      // byte copy, n = bytes written
///   let n = copy!(&mut buf[off..], data); // copy into offset
///
/// Note: Rust requires explicit `&mut` / `&` at the call site — there's
/// no equivalent to Go's implicit slice-header-by-value semantics.
#[macro_export]
macro_rules! copy {
    ($dst:expr, $src:expr) => {{
        let __dst: &mut [_] = $dst;
        let __src: &[_] = $src;
        let __n = __dst.len().min(__src.len());
        __dst[..__n].copy_from_slice(&__src[..__n]);
        __n as $crate::types::int
    }};
}

/// `len!(x)` — Go's polymorphic `len()` builtin.
///
/// Works on `string`, `&str`, `slice<T>`, `map<K,V>`, `Chan<T>`, and anything
/// else with a `.len() -> usize` method. Returns Go's `int`.
///
///   let n = len!(s);          // s: string
///   let n = len!(my_slice);   // slice<T>
///   let n = len!(my_map);     // map<K,V>
///   let n = len!(ch);         // Chan<T>
#[macro_export]
macro_rules! len {
    ($x:expr) => {
        ($x).len() as $crate::types::int
    };
}

/// `append!(s, x, y, z)` — Go's `append(s, ...)` for slices.
///
/// Consumes `s`, pushes each element (with `.into()` for widening), and
/// returns the modified slice — mirroring Go's `s = append(s, x, y, z)`.
///
///   let s = slice!([]int{1, 2, 3});
///   let s = append!(s, 4, 5, 6);          // s is now [1,2,3,4,5,6]
///   let names = slice!([]string{"a"});
///   let names = append!(names, "b", "c"); // &str literals widen to string
#[macro_export]
macro_rules! append {
    ($s:expr $(, $x:expr)+ $(,)?) => {
        {
            let mut __s = $s;
            // Fork the Arc if shared — matches Go's `append` always-succeeds
            // semantics. Requires T: Clone (true for all Go-portable types).
            __s.cow();
            $( __s.push(($x).into()); )+
            __s
        }
    };
}

/// Converts any int-shaped length to `usize`, panicking on negative or
/// overflow — matches Go's runtime check on `make([]T, n)` with n < 0.
/// Used by `make!` to accept `int` (i64) / `usize` / literals interchangeably.
#[doc(hidden)]
pub fn __goish_len<N>(n: N) -> usize
where
    N: TryInto<usize> + Copy + std::fmt::Display,
    <N as TryInto<usize>>::Error: std::fmt::Debug,
{
    n.try_into()
        .unwrap_or_else(|_| panic!("make: length {} out of range", n))
}

/// `make!(...)` — Go's `make()` builtin: allocate empty/sized container.
///
///   make!(chan int)              // unbuffered channel
///   make!(chan int, 10)          // buffered channel
///   make!(map[string]int)        // empty map
///   make!([]int, 5)              // slice of 5 zero values
///   make!([]int, 0, 10)          // slice len 0, cap 10
///
/// Use this when you want an empty container; use `slice!`, `map!`, or
/// `chan!` (with a literal body) when you want one populated up-front.
#[macro_export]
macro_rules! make {
    // make(chan T)
    (chan $t:ty) => {
        $crate::chan::Chan::<$t>::new(0)
    };
    // make(chan T, n)
    (chan $t:ty, $cap:expr) => {
        $crate::chan::Chan::<$t>::new($crate::types::__goish_len($cap))
    };
    // make(map[K]V)
    (map [$k:ty] $v:ty) => {
        {
            let m: $crate::types::map<$k, $v> = $crate::types::map::new();
            m
        }
    };
    // make([]T, 0, cap) — empty slice with capacity; no Default needed
    ([] $t:ty, 0, $cap:expr) => {
        {
            let v: $crate::types::slice<$t> =
                Vec::<$t>::with_capacity($crate::types::__goish_len($cap)).into();
            v
        }
    };
    // make([]T, len, cap)
    ([] $t:ty, $len:expr, $cap:expr) => {
        {
            let mut __v: Vec<$t> = Vec::with_capacity($crate::types::__goish_len($cap));
            __v.resize_with(
                $crate::types::__goish_len($len),
                <$t as ::std::default::Default>::default,
            );
            let v: $crate::types::slice<$t> = __v.into();
            v
        }
    };
    // make([]T, 0) — empty slice; no Default needed
    ([] $t:ty, 0) => {
        {
            let v: $crate::types::slice<$t> = Vec::<$t>::new().into();
            v
        }
    };
    // make([]T, n)
    ([] $t:ty, $n:expr) => {
        {
            let v: $crate::types::slice<$t> = vec![
                <$t as ::std::default::Default>::default();
                $crate::types::__goish_len($n)
            ].into();
            v
        }
    };
}

/// Two forms — pick the one closest to your Go original:
///
///   map!([string]int{"a" => 1, "b" => 2})    // Go-shaped:  [K]V{...}
///   map!{1i64 => "a", 2i64 => "b"}           // inferred:   no conversion
///
/// The Go-shaped form calls `.into()` on each key and value, so `&str`
/// literals turn into `string`, `i32` literals widen to `int64`, etc.
#[macro_export]
macro_rules! map {
    // Go-shaped:  map!([K]V{k => v, ...})
    ([$k:ty] $v:ty { $($key:expr => $val:expr),* $(,)? }) => {
        {
            let mut __hm = ::std::collections::HashMap::new();
            $( __hm.insert(
                <$k as ::std::convert::From<_>>::from($key),
                <$v as ::std::convert::From<_>>::from($val),
            ); )*
            let m: $crate::types::map<$k, $v> = __hm.into();
            m
        }
    };
    // Inferred form
    ($($key:expr => $val:expr),* $(,)?) => {
        {
            let mut __hm = ::std::collections::HashMap::new();
            $( __hm.insert($key, $val); )*
            let m: $crate::types::map<_, _> = __hm.into();
            m
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slice_untyped_is_vec() {
        let v = crate::slice![1, 2, 3];
        assert_eq!(v, vec![1, 2, 3]);
    }

    #[test]
    fn slice_typed_string_from_str_literals() {
        let v: slice<string> = crate::slice![string; "a", "b", "c"];
        assert_eq!(v, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    }

    #[test]
    fn slice_typed_int64_widens() {
        let v: slice<int64> = crate::slice![int64; 1i32, 2i32, 3i32];
        assert_eq!(v, vec![1i64, 2i64, 3i64]);
    }

    #[test]
    fn slice_go_shaped_string() {
        let v = crate::slice!([]string{"a", "b", "c"});
        assert_eq!(v, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    }

    #[test]
    fn slice_go_shaped_int() {
        let v = crate::slice!([]int{1i32, 2i32, 3i32});
        assert_eq!(v, vec![1i64, 2i64, 3i64]);
    }

    #[test]
    fn map_go_shaped_string_to_int() {
        let m: map<string, int> = crate::map!([string]int{"a" => 1i32, "b" => 2i32});
        assert_eq!(m.get("a"), Some(&1i64));
        assert_eq!(m.get("b"), Some(&2i64));
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn map_go_shaped_string_to_string() {
        let m = crate::map!([string]string{"host" => "db", "port" => "5432"});
        assert_eq!(m.get("host"), Some(&string::from("db")));
    }

    #[test]
    fn map_inferred_no_conversion() {
        let m = crate::map!{1i64 => string::from("a"), 2i64 => string::from("b")};
        assert_eq!(m.get(&1), Some(&string::from("a")));
    }

    #[test]
    fn make_empty_map() {
        let m = crate::make!(map[string]int);
        assert_eq!(m.len(), 0);
    }

    #[test]
    fn make_slice_with_len() {
        let v = crate::make!([]int, 5);
        assert_eq!(v, vec![0i64, 0, 0, 0, 0]);
        assert_eq!(v.len(), 5);
    }

    #[test]
    fn make_slice_with_len_and_cap() {
        let v = crate::make!([]int, 0, 10);
        assert_eq!(v.len(), 0);
        assert!(v.capacity() >= 10);
    }

    #[test]
    fn len_polymorphic() {
        let s: string = "hello".into();
        assert_eq!(crate::len!(s), 5);
        assert_eq!(crate::len!("world"), 5);

        let v: slice<int> = crate::slice!([]int{1, 2, 3, 4});
        assert_eq!(crate::len!(v), 4);

        let m: map<string, int> = crate::map!([string]int{"a" => 1, "b" => 2});
        assert_eq!(crate::len!(m), 2);

        let ch = crate::chan!(int, 4);
        ch.Send(10);
        ch.Send(20);
        assert_eq!(crate::len!(ch), 2);
    }

    #[test]
    fn append_variadic() {
        let s = crate::slice!([]int{1, 2, 3});
        let s = crate::append!(s, 4, 5);
        assert_eq!(s, vec![1i64, 2, 3, 4, 5]);
    }

    #[test]
    fn append_widens_via_into() {
        let s = crate::slice!([]string{"a"});
        let s = crate::append!(s, "b", "c"); // &str → string
        let want: Vec<string> = vec!["a".into(), "b".into(), "c".into()];
        assert_eq!(s, want);

        let s: slice<int> = crate::slice!([]int{1});
        let s = crate::append!(s, 2i32, 3i32); // i32 → i64
        assert_eq!(s, vec![1i64, 2, 3]);
    }

    #[test]
    fn delete_removes_string_key_literal() {
        let mut m: map<string, int> = crate::map!([string]int{"a" => 1, "b" => 2});
        crate::delete!(m, "a");
        assert_eq!(m.len(), 1);
        assert!(!m.contains_key("a"));
        assert!(m.contains_key("b"));
    }

    #[test]
    fn delete_with_owned_string_via_ref() {
        let mut m: map<string, int> = crate::map!([string]int{"a" => 1});
        crate::delete!(m, "a");
        assert_eq!(m.len(), 0);
    }

    #[test]
    fn delete_int_key_via_ref() {
        let mut m: map<int, string> = crate::map!{1i64 => "a".into(), 2i64 => "b".into()};
        crate::delete!(m, &1);
        assert_eq!(m.len(), 1);
        assert!(m.contains_key(&2));
    }

    #[test]
    fn make_slice_accepts_int_len() {
        let l: int = 5;
        let v = crate::make!([]int, l);
        assert_eq!(v.len(), 5);
        // Also works with usize directly.
        let u: usize = 3;
        let v = crate::make!([]int, u);
        assert_eq!(v.len(), 3);
    }

    #[test]
    #[should_panic(expected = "out of range")]
    fn make_slice_negative_len_panics() {
        let l: int = -1;
        let _ = crate::make!([]int, l);
    }

    #[test]
    fn make_slice_len_cap_accepts_int() {
        let l: int = 2;
        let c: int = 10;
        let v = crate::make!([]int, l, c);
        assert_eq!(v.len(), 2);
        assert!(v.capacity() >= 10);
    }

    #[test]
    fn enum_macro_int_const() {
        crate::Enum!(Priority = int);
        const Low: Priority = Priority(0);
        const High: Priority = Priority(10);
        let p: Priority = Low;
        assert_eq!(p, Low);
        assert!(Low < High);  // PartialOrd/Ord derived
        assert_eq!(format!("{}", High), "10");
    }

    #[test]
    fn enum_macro_uint8_const() {
        crate::Enum!(Level = uint8);
        const Debug: Level = Level(0);
        const Info: Level = Level(1);
        const Warn: Level = Level(2);
        match Info {
            Debug => panic!("wrong"),
            Info => {}
            _ => panic!("unknown"),
        }
    }

    #[test]
    fn enum_macro_string_const() {
        crate::Enum!(Color);
        const Red: Color = Color("red");
        const Blue: Color = Color("blue");
        // const-constructible + Copy + PartialEq
        let c: Color = Red;
        assert_eq!(c, Red);
        assert_ne!(c, Blue);
        // Display prints the inner string.
        assert_eq!(format!("{}", Red), "red");
        // Pattern matching works.
        match c {
            Red => {}
            Blue => panic!("wrong"),
            _ => panic!("unknown"),
        }
    }

    #[test]
    fn type_macro_int_and_slice() {
        use crate as goish;
        goish::Type!(UserId = uint64);
        goish::Type!(UserIds = []UserId);

        // Int form: slice! with bare literals widens through IntNewtype's
        // From<i32> impl that Type! forwarded to.
        let _raw: crate::types::slice<UserId> = crate::slice!([]UserId{1i32, 2i32, 3i32});
        // Slice form: From<Vec<T>> via SliceNewtype wraps the inner Vec.
        let ids: UserIds = vec![UserId(10), UserId(20), UserId(30)].into();
        assert_eq!(ids.0.len(), 3);
        // Deref lets us reach slice<UserId> methods directly.
        assert_eq!(ids.len(), 3);
        // IntoIterator by reference.
        let sum: u64 = (&ids).into_iter().map(|u| u.0).sum();
        assert_eq!(sum, 60);

        // v0.17.8: PartialEq/Eq derived — no manual impl needed.
        let a: UserIds = vec![UserId(1), UserId(2)].into();
        let b: UserIds = vec![UserId(1), UserId(2)].into();
        let c: UserIds = vec![UserId(1), UserId(3)].into();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn copy_macro_returns_count() {
        let mut dst = [0u8; 5];
        let src = b"hello world";
        // Go: n := copy(dst, src)  → 5 (dst is full)
        let n = crate::copy!(&mut dst, src);
        assert_eq!(n, 5);
        assert_eq!(&dst, b"hello");
    }

    #[test]
    fn copy_macro_min_len_wins() {
        let mut dst = [0u8; 10];
        let src = b"hi";
        let n = crate::copy!(&mut dst, src);
        assert_eq!(n, 2);
        assert_eq!(&dst[..2], b"hi");
    }

    #[test]
    fn copy_macro_with_offset_slice() {
        // Go: copy(buf[n:], more)
        let mut buf = [0u8; 10];
        let first = b"foo";
        let more = b"bar";
        let a = crate::copy!(&mut buf, first);
        let b = crate::copy!(&mut buf[a as usize..], more);
        assert_eq!(a, 3);
        assert_eq!(b, 3);
        assert_eq!(&buf[..6], b"foobar");
    }

    #[test]
    fn int_newtype_macro() {
        use crate as goish;
        goish::IntNewtype!(ID = u64);
        // From<i32> via `as` — slice! with bare literals works.
        let ids: crate::types::slice<ID> = crate::slice!([]ID{10i32, 20i32, 30i32});
        assert_eq!(ids[0i64].0, 10u64);
        assert_eq!(ids[2i64].0, 30u64);
        // From<ID> for underlying.
        let u: u64 = ID(42).into();
        assert_eq!(u, 42);
        // Derives are present.
        assert_eq!(ID(1), ID(1));
        assert!(ID(1) < ID(2));
    }

    #[test]
    fn delete_missing_key_is_noop() {
        let mut m: map<string, int> = crate::map!([string]int{"a" => 1});
        crate::delete!(m, "missing");
        assert_eq!(m.len(), 1);
    }
}
