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

pub type string = std::string::String;

// Go: []T  →  goish: slice<T>
pub type slice<T> = Vec<T>;

// Go: map[K]V  →  goish: map<K, V>
pub type map<K, V> = std::collections::HashMap<K, V>;

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
    // Go-shaped:  slice!([]T{a, b, c})
    ([] $t:ty { $($x:expr),* $(,)? }) => {
        {
            let v: $crate::types::slice<$t> = vec![ $( <$t as ::std::convert::From<_>>::from($x) ),* ];
            v
        }
    };
    // Typed semicolon form
    ($t:ty ; $($x:expr),* $(,)?) => {
        {
            let v: $crate::types::slice<$t> = vec![ $( <$t as ::std::convert::From<_>>::from($x) ),* ];
            v
        }
    };
    // Untyped (vec! alias)
    ($($x:expr),* $(,)?) => {
        vec![ $($x),* ]
    };
}

/// `range!(x, |i, v| body)` — Go's `for i, v := range x { body }` pattern.
///
///   range!(slice_expr, |i, v| { ... })   // i: usize, v: &T
///   range!(map_expr,   |k, v| { ... })   // k, v: &K, &V
///   range!(&str_expr,  |i, r| { ... })   // i: usize, r: char (rune)
///
/// Uses `.iter().enumerate()` for slices and arrays, `.iter()` for maps, and
/// `.chars().enumerate()` for string slices — whichever the expression's
/// inherent method resolution picks first.
#[macro_export]
macro_rules! range {
    // range!(slice, |i, v| body)  → index + value
    ($iter:expr, |$i:pat_param, $v:pat_param| $body:block) => {
        for ($i, $v) in $crate::range::RangeIter::range($iter) {
            $body
        }
    };
    // range!(iter, |v| body)  → value only
    ($iter:expr, |$v:pat_param| $body:block) => {
        for $v in ($iter).into_iter() {
            $body
        }
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
            $( __s.push(($x).into()); )+
            __s
        }
    };
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
        $crate::chan::Chan::<$t>::new($cap)
    };
    // make(map[K]V)
    (map [$k:ty] $v:ty) => {
        {
            let m: $crate::types::map<$k, $v> = ::std::collections::HashMap::new();
            m
        }
    };
    // make([]T, 0, cap) — empty slice with capacity; no Default needed
    ([] $t:ty, 0, $cap:expr) => {
        {
            let v: $crate::types::slice<$t> = Vec::with_capacity($cap);
            v
        }
    };
    // make([]T, len, cap)
    ([] $t:ty, $len:expr, $cap:expr) => {
        {
            let mut v: $crate::types::slice<$t> = Vec::with_capacity($cap);
            v.resize_with($len, <$t as ::std::default::Default>::default);
            v
        }
    };
    // make([]T, 0) — empty slice; no Default needed
    ([] $t:ty, 0) => {
        {
            let v: $crate::types::slice<$t> = Vec::new();
            v
        }
    };
    // make([]T, n)
    ([] $t:ty, $n:expr) => {
        {
            let v: $crate::types::slice<$t> = vec![<$t as ::std::default::Default>::default(); $n];
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
            let mut m: $crate::types::map<$k, $v> = ::std::collections::HashMap::new();
            $( m.insert(
                <$k as ::std::convert::From<_>>::from($key),
                <$v as ::std::convert::From<_>>::from($val),
            ); )*
            m
        }
    };
    // Inferred form
    ($($key:expr => $val:expr),* $(,)?) => {
        {
            let mut m = ::std::collections::HashMap::new();
            $( m.insert($key, $val); )*
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
        assert_eq!(m.get("host"), Some(&"db".to_string()));
    }

    #[test]
    fn map_inferred_no_conversion() {
        let m = crate::map!{1i64 => "a".to_string(), 2i64 => "b".to_string()};
        assert_eq!(m.get(&1), Some(&"a".to_string()));
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
        let s: string = "hello".to_string();
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
        assert_eq!(s, vec!["a".to_string(), "b".to_string(), "c".to_string()]);

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
        let key = "a".to_string();
        crate::delete!(m, &key);
        assert_eq!(m.len(), 0);
    }

    #[test]
    fn delete_int_key_via_ref() {
        let mut m: map<int, string> = crate::map!{1i64 => "a".to_string(), 2i64 => "b".to_string()};
        crate::delete!(m, &1);
        assert_eq!(m.len(), 1);
        assert!(m.contains_key(&2));
    }

    #[test]
    fn delete_missing_key_is_noop() {
        let mut m: map<string, int> = crate::map!([string]int{"a" => 1});
        crate::delete!(m, "missing");
        assert_eq!(m.len(), 1);
    }
}
