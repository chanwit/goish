// const! — Go's const block with iota auto-increment.
//
//   Go                                   goish
//   ──────────────────────────────────   ──────────────────────────────────
//   const (                              const! {
//       Sunday = iota                        Sunday = iota;
//       Monday                               Monday;
//       Tuesday                              Tuesday;
//   )                                    }
//
//   const (                              const! {
//       KB = 1 << (10 * (iota + 1))          KB = 1 << (10 * (iota + 1));
//       MB                                   MB;
//       GB                                   GB;
//   )                                    }
//
// Each bare name `Name;` repeats the *previous expression*, with `iota`
// in that expression evaluating to the current position index (0, 1, 2…).
// Each `Name = expr;` resets both the expression template and type.
//
// Types can be given with `Name: T = expr;` or `Name: T;`. Default: i64.

/// `const!{}` — Go-style constant block with `iota`.
///
/// `iota` is available as a local `const` inside each generated constant,
/// starting at 0 and incrementing by 1 for each subsequent entry in the
/// block. To repeat the previous expression, write just the name:
///
/// ```ignore
/// goish::const_block! {
///     Sunday: i64 = iota;
///     Monday;
///     Tuesday;
/// }
/// // Expands to:
/// //   pub const Sunday: i64 = { const iota: i64 = 0; iota };
/// //   pub const Monday: i64 = { const iota: i64 = 1; iota };
/// //   pub const Tuesday: i64 = { const iota: i64 = 2; iota };
/// ```
#[macro_export]
macro_rules! const_block {
    // Entry point: seed with index 0 and no prior expression.
    ( $($rest:tt)* ) => {
        $crate::__const_block_inner!(@idx 0usize; @prev (0i64); $($rest)*);
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __const_block_inner {
    // Base case: no more tokens.
    (@idx $idx:expr; @prev ($prev:expr); ) => {};

    // Typed + explicit expression:  Name: Ty = expr;
    (@idx $idx:expr; @prev ($prev:expr); $name:ident : $ty:ty = $val:expr; $($rest:tt)*) => {
        #[allow(dead_code, non_upper_case_globals)]
        pub const $name: $ty = {
            #[allow(non_upper_case_globals, unused)]
            const iota: $ty = $idx as $ty;
            $val
        };
        $crate::__const_block_inner!(@idx $idx + 1usize; @prev ($val); $($rest)*);
    };

    // Untyped + explicit expression:  Name = expr;
    (@idx $idx:expr; @prev ($prev:expr); $name:ident = $val:expr; $($rest:tt)*) => {
        #[allow(dead_code, non_upper_case_globals)]
        pub const $name: i64 = {
            #[allow(non_upper_case_globals, unused)]
            const iota: i64 = $idx as i64;
            $val
        };
        $crate::__const_block_inner!(@idx $idx + 1usize; @prev ($val); $($rest)*);
    };

    // Typed, repeat previous:  Name: Ty;
    // (Go's rule is that a bare name re-uses the *previous expression*; we
    // approximate this by literally re-emitting the stored $prev with the
    // current iota.)
    (@idx $idx:expr; @prev ($prev:expr); $name:ident : $ty:ty ; $($rest:tt)*) => {
        #[allow(dead_code, non_upper_case_globals)]
        pub const $name: $ty = {
            #[allow(non_upper_case_globals, unused)]
            const iota: $ty = $idx as $ty;
            $prev as $ty
        };
        $crate::__const_block_inner!(@idx $idx + 1usize; @prev ($prev); $($rest)*);
    };

    // Untyped, repeat previous:  Name;
    (@idx $idx:expr; @prev ($prev:expr); $name:ident ; $($rest:tt)*) => {
        #[allow(dead_code, non_upper_case_globals)]
        pub const $name: i64 = {
            #[allow(non_upper_case_globals, unused)]
            const iota: i64 = $idx as i64;
            $prev
        };
        $crate::__const_block_inner!(@idx $idx + 1usize; @prev ($prev); $($rest)*);
    };
}

#[cfg(test)]
mod tests {
    // Weekday enum pattern.
    crate::const_block! {
        Sunday = iota;
        Monday;
        Tuesday;
        Wednesday;
        Thursday;
        Friday;
        Saturday;
    }

    #[test]
    fn weekday_iota() {
        assert_eq!(Sunday, 0);
        assert_eq!(Monday, 1);
        assert_eq!(Tuesday, 2);
        assert_eq!(Saturday, 6);
    }

    // Storage units — iota shift pattern.
    crate::const_block! {
        _ignored = 1 << (10 * iota);
        KB;
        MB;
        GB;
        TB;
    }

    #[test]
    fn storage_units() {
        assert_eq!(KB, 1 << 10);
        assert_eq!(MB, 1 << 20);
        assert_eq!(GB, 1 << 30);
        assert_eq!(TB, 1i64 << 40);
    }

    // Typed + bit-flag pattern.
    crate::const_block! {
        ReadPerm:  u32 = 1 << iota;
        WritePerm: u32;
        ExecPerm:  u32;
    }

    #[test]
    fn permissions_typed() {
        assert_eq!(ReadPerm, 1u32);
        assert_eq!(WritePerm, 2u32);
        assert_eq!(ExecPerm, 4u32);
    }
}
