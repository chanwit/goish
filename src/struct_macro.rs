// Struct!: Go-style type declaration with a paired positional literal macro.
//
//   Go                                         goish
//   ────────────────────────────────────────   ──────────────────────────────────────
//   type PathTest struct {                     Struct!{ type PathTest struct {
//       path, result string                        path, result string
//   }                                          } }
//
//   var t = PathTest{"x", "y"}                 let t = PathTest!("x", "y");
//
// `Struct!` expands to both a `struct` declaration and a macro with the
// same name that builds the struct positionally. The generated macro
// automatically calls `.into()` on `string` fields and leaves other types
// alone, so string literal args work without `.into()` at the call site.
//
// Supported field-group forms inside the braces:
//   path, result string          // group of same-type fields
//   count int                    // single typed field
//   a, b string; count int       // multiple groups separated by `;`
//   elem []string; path string   // slice and scalar mixed

#[macro_export]
#[doc(hidden)]
macro_rules! __goish_field_convert {
    ($v:expr, string) => { $crate::__goish_into_string($v) };
    ($v:expr, [] string) => { $v };
    ($v:expr, [] $_t:tt) => { $v };
    ($v:expr, $_ty:tt) => { $v };
}

/// Internal helper: coerces `&str`/`String`/anything-Into-String into `String`.
#[doc(hidden)]
pub fn __goish_into_string<T: Into<String>>(v: T) -> String { v.into() }

#[macro_export]
macro_rules! Struct {
    // Entry point
    (type $name:ident struct { $($body:tt)* }) => {
        $crate::__goish_struct_parse!(@start [$name] [] [] $($body)*);
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __goish_struct_parse {
    // Terminal — emit struct and constructor macro
    (@start [$name:ident] [$($fields:tt)*] [$($order:tt)*]) => {
        $crate::__goish_struct_emit!([$name] [$($fields)*] [$($order)*]);
    };

    // Multi-name group: `a, b, c TYPE ;` or `a, b, c TYPE` at end
    (@start [$name:ident] [$($fd:tt)*] [$($ord:tt)*] $f:ident , $($rest:tt)*) => {
        $crate::__goish_struct_parse!(@collect [$name] [$($fd)*] [$($ord)*] [$f] $($rest)*);
    };

    // Qualified-path type: `name A::B::...::C ;` or trailing.
    // Matched BEFORE the single-tt arm so `semver::Version` is consumed as
    // a unit; the emitted type is paren-wrapped to keep it a single tt for
    // downstream __goish_type! / __goish_cast! dispatch.
    (@start [$name:ident] [$($fd:tt)*] [$($ord:tt)*] $f:ident $h:ident $(:: $s:ident)+ ; $($rest:tt)*) => {
        $crate::__goish_struct_parse!(@start [$name]
            [$($fd)* ($f : ($h $(:: $s)+) ,)]
            [$($ord)* ($f ($h $(:: $s)+))]
            $($rest)*);
    };
    (@start [$name:ident] [$($fd:tt)*] [$($ord:tt)*] $f:ident $h:ident $(:: $s:ident)+) => {
        $crate::__goish_struct_parse!(@start [$name]
            [$($fd)* ($f : ($h $(:: $s)+) ,)]
            [$($ord)* ($f ($h $(:: $s)+))]);
    };

    // Slice type: `name []T ;` or trailing.
    // `[]` is two tokens (bracket group + inner), so this arm has to run
    // before the single-tt arm. The emitted type is paren-wrapped
    // `( slice<T> )` so it remains a single tt for ctor-macro dispatch.
    (@start [$name:ident] [$($fd:tt)*] [$($ord:tt)*] $f:ident [ ] $inner:tt ; $($rest:tt)*) => {
        $crate::__goish_struct_parse!(@start [$name]
            [$($fd)* ($f : ( $crate::types::slice<$crate::__goish_type!($inner)> ) ,)]
            [$($ord)* ($f ( $crate::types::slice<$crate::__goish_type!($inner)> ))]
            $($rest)*);
    };
    (@start [$name:ident] [$($fd:tt)*] [$($ord:tt)*] $f:ident [ ] $inner:tt) => {
        $crate::__goish_struct_parse!(@start [$name]
            [$($fd)* ($f : ( $crate::types::slice<$crate::__goish_type!($inner)> ) ,)]
            [$($ord)* ($f ( $crate::types::slice<$crate::__goish_type!($inner)> ))]);
    };

    // Single field: `name TYPE ;` or `name TYPE` at end (TYPE = single tt)
    (@start [$name:ident] [$($fd:tt)*] [$($ord:tt)*] $f:ident $ty:tt ; $($rest:tt)*) => {
        $crate::__goish_struct_parse!(@start [$name]
            [$($fd)* ($f : $ty ,)]
            [$($ord)* ($f $ty)]
            $($rest)*);
    };
    (@start [$name:ident] [$($fd:tt)*] [$($ord:tt)*] $f:ident $ty:tt) => {
        $crate::__goish_struct_parse!(@start [$name]
            [$($fd)* ($f : $ty ,)]
            [$($ord)* ($f $ty)]);
    };

    // Gather more names in a multi-name group
    (@collect [$name:ident] [$($fd:tt)*] [$($ord:tt)*] [$($names:ident)+] $next:ident , $($rest:tt)*) => {
        $crate::__goish_struct_parse!(@collect [$name] [$($fd)*] [$($ord)*] [$($names)+ $next] $($rest)*);
    };
    // Multi-name group ending with qualified-path type.
    (@collect [$name:ident] [$($fd:tt)*] [$($ord:tt)*] [$($names:ident)+] $last:ident $h:ident $(:: $s:ident)+ ; $($rest:tt)*) => {
        $crate::__goish_struct_parse!(@start [$name]
            [$($fd)* $( ($names : ($h $(:: $s)+) ,) )+ ($last : ($h $(:: $s)+) ,)]
            [$($ord)* $( ($names ($h $(:: $s)+)) )+ ($last ($h $(:: $s)+))]
            $($rest)*);
    };
    (@collect [$name:ident] [$($fd:tt)*] [$($ord:tt)*] [$($names:ident)+] $last:ident $h:ident $(:: $s:ident)+) => {
        $crate::__goish_struct_parse!(@start [$name]
            [$($fd)* $( ($names : ($h $(:: $s)+) ,) )+ ($last : ($h $(:: $s)+) ,)]
            [$($ord)* $( ($names ($h $(:: $s)+)) )+ ($last ($h $(:: $s)+))]);
    };

    // Multi-name group ending with slice type: `a, b []T ;`
    (@collect [$name:ident] [$($fd:tt)*] [$($ord:tt)*] [$($names:ident)+] $last:ident [ ] $inner:tt ; $($rest:tt)*) => {
        $crate::__goish_struct_parse!(@start [$name]
            [$($fd)* $( ($names : ( $crate::types::slice<$crate::__goish_type!($inner)> ) ,) )+
                      ($last : ( $crate::types::slice<$crate::__goish_type!($inner)> ) ,)]
            [$($ord)* $( ($names ( $crate::types::slice<$crate::__goish_type!($inner)> )) )+
                      ($last ( $crate::types::slice<$crate::__goish_type!($inner)> ))]
            $($rest)*);
    };
    (@collect [$name:ident] [$($fd:tt)*] [$($ord:tt)*] [$($names:ident)+] $last:ident [ ] $inner:tt) => {
        $crate::__goish_struct_parse!(@start [$name]
            [$($fd)* $( ($names : ( $crate::types::slice<$crate::__goish_type!($inner)> ) ,) )+
                      ($last : ( $crate::types::slice<$crate::__goish_type!($inner)> ) ,)]
            [$($ord)* $( ($names ( $crate::types::slice<$crate::__goish_type!($inner)> )) )+
                      ($last ( $crate::types::slice<$crate::__goish_type!($inner)> ))]);
    };
    // Last ident in group + single-tt type + optional ; + more
    (@collect [$name:ident] [$($fd:tt)*] [$($ord:tt)*] [$($names:ident)+] $last:ident $ty:tt ; $($rest:tt)*) => {
        $crate::__goish_struct_parse!(@start [$name]
            [$($fd)* $( ($names : $ty ,) )+ ($last : $ty ,)]
            [$($ord)* $( ($names $ty) )+ ($last $ty)]
            $($rest)*);
    };
    (@collect [$name:ident] [$($fd:tt)*] [$($ord:tt)*] [$($names:ident)+] $last:ident $ty:tt) => {
        $crate::__goish_struct_parse!(@start [$name]
            [$($fd)* $( ($names : $ty ,) )+ ($last : $ty ,)]
            [$($ord)* $( ($names $ty) )+ ($last $ty)]);
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __goish_struct_emit {
    ([$name:ident] [$( ($fn:ident : $ft:tt ,) )*] [$( ($on:ident $ot:tt) )*]) => {
        #[derive(Clone, Debug, Default)]
        #[allow(non_snake_case)]
        pub struct $name {
            $( pub $fn: $crate::__goish_type!($ft), )*
        }

        $crate::__goish_struct_ctor!($name; $( ($on $ot) )*);
    };
}

// Map Go-style type tokens to Rust types
#[macro_export]
#[doc(hidden)]
macro_rules! __goish_type {
    (string) => { $crate::types::string };
    (int) => { $crate::types::int };
    (int64) => { $crate::types::int64 };
    (int32) => { $crate::types::int32 };
    (byte) => { $crate::types::byte };
    (rune) => { $crate::types::rune };
    (bool) => { bool };
    (float64) => { $crate::types::float64 };
    (float32) => { $crate::types::float32 };
    // []T slice
    ([ ] string) => { $crate::types::slice<$crate::types::string> };
    ([ ] int) => { $crate::types::slice<$crate::types::int> };
    ([ ] int64) => { $crate::types::slice<$crate::types::int64> };
    ([ ] byte) => { $crate::types::slice<$crate::types::byte> };
    ([ ] bool) => { $crate::types::slice<bool> };
    ([ ] float64) => { $crate::types::slice<$crate::types::float64> };
    // Fallback: pass through as-is (user-named type)
    ($t:ty) => { $t };
}

// Generate the positional-constructor macro. One arm per field count (up to 12).
#[macro_export]
#[doc(hidden)]
macro_rules! __goish_struct_ctor {
    // 1 field
    ($name:ident; ($on1:ident $ot1:tt)) => {
        #[macro_export]
        #[allow(non_snake_case)]
        macro_rules! $name {
            ($a1:expr) => {
                $name { $on1: $crate::__goish_cast!($a1, $ot1) }
            };
        }
    };
    // 2 fields
    ($name:ident; ($on1:ident $ot1:tt) ($on2:ident $ot2:tt)) => {
        #[macro_export]
        #[allow(non_snake_case)]
        macro_rules! $name {
            ($a1:expr, $a2:expr) => {
                $name {
                    $on1: $crate::__goish_cast!($a1, $ot1),
                    $on2: $crate::__goish_cast!($a2, $ot2),
                }
            };
        }
    };
    // 3 fields
    ($name:ident; ($on1:ident $ot1:tt) ($on2:ident $ot2:tt) ($on3:ident $ot3:tt)) => {
        #[macro_export]
        #[allow(non_snake_case)]
        macro_rules! $name {
            ($a1:expr, $a2:expr, $a3:expr) => {
                $name {
                    $on1: $crate::__goish_cast!($a1, $ot1),
                    $on2: $crate::__goish_cast!($a2, $ot2),
                    $on3: $crate::__goish_cast!($a3, $ot3),
                }
            };
        }
    };
    // 4 fields
    ($name:ident; ($on1:ident $ot1:tt) ($on2:ident $ot2:tt) ($on3:ident $ot3:tt) ($on4:ident $ot4:tt)) => {
        #[macro_export]
        #[allow(non_snake_case)]
        macro_rules! $name {
            ($a1:expr, $a2:expr, $a3:expr, $a4:expr) => {
                $name {
                    $on1: $crate::__goish_cast!($a1, $ot1),
                    $on2: $crate::__goish_cast!($a2, $ot2),
                    $on3: $crate::__goish_cast!($a3, $ot3),
                    $on4: $crate::__goish_cast!($a4, $ot4),
                }
            };
        }
    };
    // 5 fields
    ($name:ident; ($on1:ident $ot1:tt) ($on2:ident $ot2:tt) ($on3:ident $ot3:tt) ($on4:ident $ot4:tt) ($on5:ident $ot5:tt)) => {
        #[macro_export]
        #[allow(non_snake_case)]
        macro_rules! $name {
            ($a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr) => {
                $name {
                    $on1: $crate::__goish_cast!($a1, $ot1),
                    $on2: $crate::__goish_cast!($a2, $ot2),
                    $on3: $crate::__goish_cast!($a3, $ot3),
                    $on4: $crate::__goish_cast!($a4, $ot4),
                    $on5: $crate::__goish_cast!($a5, $ot5),
                }
            };
        }
    };
    // 6 fields
    ($name:ident; ($on1:ident $ot1:tt) ($on2:ident $ot2:tt) ($on3:ident $ot3:tt) ($on4:ident $ot4:tt) ($on5:ident $ot5:tt) ($on6:ident $ot6:tt)) => {
        #[macro_export]
        #[allow(non_snake_case)]
        macro_rules! $name {
            ($a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr, $a6:expr) => {
                $name {
                    $on1: $crate::__goish_cast!($a1, $ot1),
                    $on2: $crate::__goish_cast!($a2, $ot2),
                    $on3: $crate::__goish_cast!($a3, $ot3),
                    $on4: $crate::__goish_cast!($a4, $ot4),
                    $on5: $crate::__goish_cast!($a5, $ot5),
                    $on6: $crate::__goish_cast!($a6, $ot6),
                }
            };
        }
    };
}

// __goish_cast — per-field conversion at the positional call site.
#[macro_export]
#[doc(hidden)]
macro_rules! __goish_cast {
    ($v:expr, string) => { ($v).into() };
    ($v:expr, $_ty:tt) => { $v };
}

#[cfg(test)]
mod tests {
    Struct!{ type PathTest struct { path, result string } }

    #[test]
    fn path_test_positional_construction() {
        let t = PathTest!("abc", "def");
        assert_eq!(t.path, "abc");
        assert_eq!(t.result, "def");
    }

    Struct!{ type IsAbsTest struct { path string; isAbs bool } }

    #[test]
    fn is_abs_test_positional_construction() {
        let t = IsAbsTest!("/foo", true);
        assert_eq!(t.path, "/foo");
        assert_eq!(t.isAbs, true);
    }

    Struct!{ type Triple struct { a, b, c string } }

    #[test]
    fn triple_construction() {
        let t = Triple!("x", "y", "z");
        assert_eq!(t.a, "x");
        assert_eq!(t.b, "y");
        assert_eq!(t.c, "z");
    }

    Struct!{ type Mixed struct { name string; count int; ok bool } }

    #[test]
    fn mixed_types() {
        let t = Mixed!("alpha", 42i64, true);
        assert_eq!(t.name, "alpha");
        assert_eq!(t.count, 42);
        assert_eq!(t.ok, true);
    }

    // Nested module for the qualified-path test.
    mod semver {
        #[allow(non_snake_case)]
        #[derive(Clone, Debug, Default, PartialEq)]
        pub struct Version { pub Major: i64, pub Minor: i64, pub Patch: i64 }
    }
    Struct!{ type tcase struct {
        name string;
        ver1 semver::Version;
        ver2 semver::Version;
        expected int
    } }

    Struct!{ type Bag struct { name string; items []string } }

    #[test]
    fn slice_field_types() {
        let b = Bag!(
            "basket",
            vec![crate::gostring::GoString::from("a"), "b".into()].into()
        );
        assert_eq!(b.name, "basket");
        assert_eq!(b.items.len(), 2);
        assert_eq!(b.items[0i64], "a");
    }

    Struct!{ type Parts struct { head, tail []int } }

    #[test]
    fn multi_name_slice_fields() {
        let empty: crate::types::slice<i64> = crate::types::slice::new();
        let p = Parts!(empty.clone(), empty);
        assert_eq!(p.head.len(), 0);
        assert_eq!(p.tail.len(), 0);
    }

    #[test]
    fn qualified_path_field_types() {
        let t = tcase!(
            "1.2.0 vs 1.3.0",
            semver::Version { Major: 1, Minor: 2, Patch: 0 },
            semver::Version { Major: 1, Minor: 3, Patch: 0 },
            -1i64
        );
        assert_eq!(t.name, "1.2.0 vs 1.3.0");
        assert_eq!(t.ver1.Minor, 2);
        assert_eq!(t.ver2.Minor, 3);
        assert_eq!(t.expected, -1);
    }
}
