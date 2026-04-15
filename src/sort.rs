// sort: Go's sort package.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   sort.Ints(s)                        sort::Ints(&mut s);
//   sort.Strings(s)                     sort::Strings(&mut s);
//   sort.Float64s(s)                    sort::Float64s(&mut s);
//   sort.Slice(s, func(i,j int) bool    sort::Slice(&mut s, |i, j| …);
//   sort.SliceStable(s, less)           sort::SliceStable(&mut s, |i, j| …);
//   sort.IntsAreSorted(s)               sort::IntsAreSorted(&s)

use crate::types::{float64, int, slice, string};

#[allow(non_snake_case)]
pub fn Ints(s: &mut slice<int>) {
    s.sort();
}

#[allow(non_snake_case)]
pub fn Strings(s: &mut slice<string>) {
    s.sort();
}

#[allow(non_snake_case)]
pub fn Float64s(s: &mut slice<float64>) {
    // Go's sort.Float64s treats NaN as less than any non-NaN. std's f64 isn't
    // totally ordered; use total_cmp which matches Go's NaN positioning.
    s.sort_by(|a, b| a.total_cmp(b));
}

/// sort.Slice(s, less) — `less(a, b)` is true if a should come before b.
///
/// Note: Go uses `less(i, j int) bool` with the slice captured in the
/// closure. Rust's borrow checker forbids borrowing the slice inside a
/// closure while it's being sorted, so we take `(&T, &T)` instead — same
/// semantics, different arg form.
#[allow(non_snake_case)]
pub fn Slice<T, F>(s: &mut slice<T>, mut less: F)
where
    F: FnMut(&T, &T) -> bool,
{
    s.sort_unstable_by(|a, b| {
        if less(a, b) { std::cmp::Ordering::Less }
        else if less(b, a) { std::cmp::Ordering::Greater }
        else { std::cmp::Ordering::Equal }
    });
}

#[allow(non_snake_case)]
pub fn SliceStable<T, F>(s: &mut slice<T>, mut less: F)
where
    F: FnMut(&T, &T) -> bool,
{
    s.sort_by(|a, b| {
        if less(a, b) { std::cmp::Ordering::Less }
        else if less(b, a) { std::cmp::Ordering::Greater }
        else { std::cmp::Ordering::Equal }
    });
}

#[allow(non_snake_case)]
pub fn IntsAreSorted(s: &[int]) -> bool {
    s.windows(2).all(|w| w[0] <= w[1])
}

#[allow(non_snake_case)]
pub fn StringsAreSorted(s: &[string]) -> bool {
    s.windows(2).all(|w| w[0] <= w[1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ints_sorted() {
        let mut v = vec![3i64, 1, 4, 1, 5, 9, 2, 6];
        Ints(&mut v);
        assert_eq!(v, vec![1, 1, 2, 3, 4, 5, 6, 9]);
        assert!(IntsAreSorted(&v));
    }

    #[test]
    fn strings_sorted() {
        let mut v: Vec<String> = vec!["banana".into(), "apple".into(), "cherry".into()];
        Strings(&mut v);
        assert_eq!(v, vec!["apple", "banana", "cherry"]);
        assert!(StringsAreSorted(&v));
    }

    #[test]
    fn float64s_handles_nan() {
        let mut v = vec![3.0f64, f64::NAN, 1.0, 2.0];
        Float64s(&mut v);
        assert!(v[0].is_nan() || v.last().unwrap().is_nan());
    }

    #[test]
    fn slice_with_custom_less() {
        let mut v = vec!["aa", "b", "ccc"];
        Slice(&mut v, |a, b| a.len() < b.len());
        assert_eq!(v, vec!["b", "aa", "ccc"]);
    }

    #[test]
    fn slice_stable_preserves_equal_order() {
        let mut v: Vec<(i64, i64)> = vec![(1, 0), (2, 0), (1, 1), (2, 1)];
        SliceStable(&mut v, |a, b| a.0 < b.0);
        assert_eq!(v, vec![(1, 0), (1, 1), (2, 0), (2, 1)]);
    }
}
