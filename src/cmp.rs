//! cmp: Go's cmp package — ordered comparison helpers.
//!
//!   Go                                  goish
//!   ─────────────────────────────────   ──────────────────────────────────
//!   cmp.Compare(a, b)                   cmp::Compare(&a, &b)
//!   cmp.Less(a, b)                      cmp::Less(&a, &b)
//!   cmp.Or(a, b, c)                     cmp::Or(&[a, b, c])
//!
//! Go's `Ordered` constraint unifies ints, floats, strings; Rust's
//! `PartialOrd` covers the same territory (with extra NaN handling for
//! floats that we re-implement to match Go semantics).
//!
//! NaN rule (Go spec):
//!   - `Less`: NaN is LESS than any non-NaN.
//!   - `Compare`: NaN == NaN, NaN < any non-NaN.
//! Rust's `PartialOrd::partial_cmp` returns `None` on NaN, so we branch
//! explicitly.

/// `cmp.Less(x, y)` — reports whether x < y, with NaN ordered below all
/// other floats. Works for any `PartialOrd` type.
#[allow(non_snake_case)]
pub fn Less<T: PartialOrd>(x: &T, y: &T) -> bool {
    // x < y per PartialOrd; if PartialOrd returns None (e.g. NaN vs
    // anything), test whether x is the NaN by comparing with itself.
    if let Some(o) = x.partial_cmp(y) {
        return o == std::cmp::Ordering::Less;
    }
    // At least one of x, y is NaN (or incomparable). Go says NaN < non-NaN.
    // x is NaN iff x != x. If x is NaN and y is not, x < y. Otherwise false.
    x != x && y == y
}

/// `cmp.Compare(x, y)` — returns -1, 0, or +1 (as `i64`). NaN == NaN
/// and NaN is less than every non-NaN (Go spec).
#[allow(non_snake_case)]
pub fn Compare<T: PartialOrd>(x: &T, y: &T) -> crate::types::int {
    let x_nan = x != x;
    let y_nan = y != y;
    if x_nan {
        return if y_nan { 0 } else { -1 };
    }
    if y_nan {
        return 1;
    }
    match x.partial_cmp(y) {
        Some(std::cmp::Ordering::Less) => -1,
        Some(std::cmp::Ordering::Greater) => 1,
        _ => 0,
    }
}

/// `cmp.Or(vals...)` — returns the first non-zero value, or the zero value
/// if all are zero. Accepts any `PartialEq + Default` type.
///
/// Go's signature is variadic; in goish we take a slice. Call sites:
///   cmp.Or(&[a, b, c])
#[allow(non_snake_case)]
pub fn Or<T: PartialEq + Default + Clone>(vals: &[T]) -> T {
    let zero = T::default();
    for v in vals {
        if *v != zero {
            return v.clone();
        }
    }
    zero
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn less_int() {
        assert!(Less(&1, &2));
        assert!(!Less(&2, &1));
        assert!(!Less(&1, &1));
    }

    #[test]
    fn less_float_nan() {
        assert!(Less(&f64::NAN, &0.0));
        assert!(!Less(&0.0, &f64::NAN));
        assert!(!Less(&f64::NAN, &f64::NAN));
    }

    #[test]
    fn compare_int() {
        assert_eq!(Compare(&1, &2), -1);
        assert_eq!(Compare(&2, &1), 1);
        assert_eq!(Compare(&5, &5), 0);
    }

    #[test]
    fn compare_float_nan() {
        assert_eq!(Compare(&f64::NAN, &f64::NAN), 0);
        assert_eq!(Compare(&f64::NAN, &1.0), -1);
        assert_eq!(Compare(&1.0, &f64::NAN), 1);
    }

    #[test]
    fn or_picks_first_nonzero() {
        assert_eq!(Or(&[0i64, 0, 42, 100]), 42);
        assert_eq!(Or::<i64>(&[0, 0, 0]), 0);
        assert_eq!(Or::<String>(&[String::new(), "hi".into()]), "hi");
    }
}
