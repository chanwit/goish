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

#[allow(non_snake_case)]
pub fn Float64sAreSorted(s: &[float64]) -> bool {
    s.windows(2).all(|w| w[0].total_cmp(&w[1]).is_le())
}

// ── sort.Search and SearchX ──────────────────────────────────────────
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   i := sort.Search(n, func(i) bool)   let i = sort::Search(n, |i| …);
//   i := sort.SearchInts(s, v)          let i = sort::SearchInts(&s, v);
//   i := sort.SearchStrings(s, v)       let i = sort::SearchStrings(&s, v);
//
// Returns the smallest index i in [0,n) at which f(i) is true, or n if none
// is. f must be monotonic (all false, then all true) as in Go.
#[allow(non_snake_case)]
pub fn Search(n: int, mut f: impl FnMut(int) -> bool) -> int {
    let mut lo: int = 0;
    let mut hi: int = n;
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if !f(mid) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

#[allow(non_snake_case)]
pub fn SearchInts(s: &[int], v: int) -> int {
    Search(s.len() as int, |i| s[i as usize] >= v)
}

#[allow(non_snake_case)]
pub fn SearchStrings(s: &[string], v: impl AsRef<str>) -> int {
    let v = v.as_ref();
    Search(s.len() as int, |i| &*s[i as usize] >= v)
}

#[allow(non_snake_case)]
pub fn SearchFloat64s(s: &[float64], v: float64) -> int {
    Search(s.len() as int, |i| s[i as usize] >= v)
}

// ── sort.Interface façades: IntSlice / StringSlice / Float64Slice ─────
//
// In Go these are types with Len/Less/Swap that satisfy sort.Interface.
// Here they're thin newtypes that offer .Sort() and .Search(v) as the
// typical call sites.

#[allow(non_snake_case)]
pub struct IntSlice<'a>(pub &'a mut slice<int>);

impl<'a> IntSlice<'a> {
    pub fn Sort(&mut self) { self.0.sort(); }
    pub fn Search(&self, v: int) -> int { SearchInts(self.0, v) }
    pub fn Len(&self) -> int { self.0.len() as int }
    pub fn Less(&self, i: int, j: int) -> bool { self.0[i as usize] < self.0[j as usize] }
    pub fn Swap(&mut self, i: int, j: int) { self.0.swap(i as usize, j as usize); }
}

#[allow(non_snake_case)]
pub struct StringSlice<'a>(pub &'a mut slice<string>);

impl<'a> StringSlice<'a> {
    pub fn Sort(&mut self) { self.0.sort(); }
    pub fn Search(&self, v: impl AsRef<str>) -> int { SearchStrings(self.0, v) }
    pub fn Len(&self) -> int { self.0.len() as int }
    pub fn Less(&self, i: int, j: int) -> bool { self.0[i as usize] < self.0[j as usize] }
    pub fn Swap(&mut self, i: int, j: int) { self.0.swap(i as usize, j as usize); }
}

#[allow(non_snake_case)]
pub struct Float64Slice<'a>(pub &'a mut slice<float64>);

impl<'a> Float64Slice<'a> {
    pub fn Sort(&mut self) { self.0.sort_by(|a, b| a.total_cmp(b)); }
    pub fn Search(&self, v: float64) -> int { SearchFloat64s(self.0, v) }
    pub fn Len(&self) -> int { self.0.len() as int }
    pub fn Less(&self, i: int, j: int) -> bool { self.0[i as usize] < self.0[j as usize] }
    pub fn Swap(&mut self, i: int, j: int) { self.0.swap(i as usize, j as usize); }
}

// ── Reverse ──────────────────────────────────────────────────────────
//
// Go: sort.Sort(sort.Reverse(sort.IntSlice(s)))
//
// In goish, because our IntSlice etc. offer `.Sort()` directly, Reverse
// is provided as `ReverseInts`/`ReverseStrings`/etc. helpers, plus a
// generic `Reverse(&mut slice, less)` that flips the comparator.

#[allow(non_snake_case)]
pub fn Reverse<T, F>(s: &mut slice<T>, mut less: F)
where
    F: FnMut(&T, &T) -> bool,
{
    s.sort_by(|a, b| {
        if less(a, b) { std::cmp::Ordering::Greater }
        else if less(b, a) { std::cmp::Ordering::Less }
        else { std::cmp::Ordering::Equal }
    });
}

#[allow(non_snake_case)]
pub fn ReverseInts(s: &mut slice<int>) {
    s.sort_by(|a, b| b.cmp(a));
}

#[allow(non_snake_case)]
pub fn ReverseStrings(s: &mut slice<string>) {
    s.sort_by(|a, b| b.cmp(a));
}

#[allow(non_snake_case)]
pub fn ReverseFloat64s(s: &mut slice<float64>) {
    s.sort_by(|a, b| b.total_cmp(a));
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
        let mut v: Vec<string> = vec!["banana".into(), "apple".into(), "cherry".into()];
        Strings(&mut v);
        assert_eq!(v, vec![string::from("apple"), "banana".into(), "cherry".into()]);
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

    #[test]
    fn search_finds_insertion_point() {
        let s = vec![1i64, 3, 5, 7, 9];
        assert_eq!(SearchInts(&s, 5), 2);
        assert_eq!(SearchInts(&s, 4), 2);
        assert_eq!(SearchInts(&s, 0), 0);
        assert_eq!(SearchInts(&s, 10), 5);
        let strs: Vec<string> = vec!["apple".into(), "banana".into(), "cherry".into()];
        assert_eq!(SearchStrings(&strs, "banana"), 1);
        assert_eq!(SearchStrings(&strs, "blueberry"), 2);
    }

    #[test]
    fn search_custom_predicate() {
        // smallest i such that i*i >= 17 in [0, 10)
        assert_eq!(Search(10, |i| i * i >= 17), 5);
    }

    #[test]
    fn int_slice_wrapper() {
        let mut v = vec![3i64, 1, 4, 1, 5];
        let mut s = IntSlice(&mut v);
        s.Sort();
        assert_eq!(s.Search(4), 3);
        assert_eq!(*s.0, vec![1, 1, 3, 4, 5]);
    }

    #[test]
    fn reverse_sorts_descending() {
        let mut v = vec![3i64, 1, 4, 1, 5];
        ReverseInts(&mut v);
        assert_eq!(v, vec![5, 4, 3, 1, 1]);
        let mut v: Vec<string> = vec!["b".into(), "a".into(), "c".into()];
        ReverseStrings(&mut v);
        assert_eq!(v, vec![string::from("c"), "b".into(), "a".into()]);
    }

    #[test]
    fn reverse_with_less() {
        let mut v = vec![1i64, 2, 3];
        Reverse(&mut v, |a, b| a < b);
        assert_eq!(v, vec![3, 2, 1]);
    }
}
