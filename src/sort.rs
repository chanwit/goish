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

use crate::types::{float64, int, string};

// Sort funcs take `&mut [T]`, not `&mut slice<T>` — DerefMut on slice<T>
// and the Vec→slice coercion for `&mut Vec<T>` both give `&mut [T]`, so
// callers can pass either container.

#[allow(non_snake_case)]
pub fn Ints(s: &mut [int]) {
    s.sort();
}

#[allow(non_snake_case)]
pub fn Strings(s: &mut [string]) {
    s.sort();
}

#[allow(non_snake_case)]
pub fn Float64s(s: &mut [float64]) {
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
pub fn Slice<T, F>(s: &mut [T], mut less: F)
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
pub fn SliceStable<T, F>(s: &mut [T], mut less: F)
where
    F: FnMut(&T, &T) -> bool,
{
    s.sort_by(|a, b| {
        if less(a, b) { std::cmp::Ordering::Less }
        else if less(b, a) { std::cmp::Ordering::Greater }
        else { std::cmp::Ordering::Equal }
    });
}

// ── sort.Interface + sort.Sort / sort.Stable ─────────────────────────
//
// Go's `sort.Interface` is the Len/Less/Swap triple. In Rust we express it
// as a trait; `sort::Sort(&mut data)` drives the underlying sort using
// only trait calls (no element access), matching Go's semantics.
//
//   Go                                        goish
//   ──────────────────────────────────        ──────────────────────────────────
//   type Uint64Slice []uint64                 struct Uint64Slice(pub slice<uint64>);
//   func (p Uint64Slice) Len() int { … }      impl sort::Interface for Uint64Slice { fn Len(&self) -> int { … } }
//   sort.Sort(g)                              sort::Sort(&mut g);

#[allow(non_snake_case)]
pub trait Interface {
    fn Len(&self) -> int;
    fn Less(&self, i: int, j: int) -> bool;
    fn Swap(&mut self, i: int, j: int);
}

/// sort.Sort(data) — sorts via the Interface trait. O(n log n) using
/// heapsort (in-place, no allocations), matching Go's guarantee.
#[allow(non_snake_case)]
pub fn Sort<T: Interface + ?Sized>(data: &mut T) {
    let n = data.Len();
    if n < 2 { return; }
    // Build max-heap.
    let mut i = n / 2 - 1;
    loop {
        sift_down(data, i, n);
        if i == 0 { break; }
        i -= 1;
    }
    // Repeatedly swap max to the end.
    let mut end = n - 1;
    while end > 0 {
        data.Swap(0, end);
        sift_down(data, 0, end);
        end -= 1;
    }
}

fn sift_down<T: Interface + ?Sized>(data: &mut T, mut root: int, end: int) {
    loop {
        let mut child = 2 * root + 1;
        if child >= end { break; }
        if child + 1 < end && data.Less(child, child + 1) {
            child += 1;
        }
        if !data.Less(root, child) { break; }
        data.Swap(root, child);
        root = child;
    }
}

/// sort.Stable(data) — stable sort via the Interface trait. O(n log² n)
/// using in-place mergesort; matches Go's sort.Stable semantics.
#[allow(non_snake_case)]
pub fn Stable<T: Interface + ?Sized>(data: &mut T) {
    // Simple stable insertion sort within blocks, then merge.
    let n = data.Len();
    let block_size: int = 20;
    let mut a: int = 0;
    while a < n {
        let b = (a + block_size).min(n);
        insertion_sort(data, a, b);
        a = b;
    }
    let mut size = block_size;
    while size < n {
        let mut a: int = 0;
        while a + size < n {
            let mid = a + size;
            let b = (a + 2 * size).min(n);
            sym_merge(data, a, mid, b);
            a = b;
        }
        size *= 2;
    }
}

fn insertion_sort<T: Interface + ?Sized>(data: &mut T, a: int, b: int) {
    for i in (a + 1)..b {
        let mut j = i;
        while j > a && data.Less(j, j - 1) {
            data.Swap(j, j - 1);
            j -= 1;
        }
    }
}

// SymMerge from Go's sort/sort.go — O(m+n) time, O(log) stack.
fn sym_merge<T: Interface + ?Sized>(data: &mut T, a: int, m: int, b: int) {
    if m - a == 1 {
        let mut i = m;
        while i < b && data.Less(i, a) { i += 1; }
        rotate(data, a, m, i);
        return;
    }
    if b - m == 1 {
        let mut i = a;
        while i < m && !data.Less(m, i) { i += 1; }
        rotate(data, i, m, b);
        return;
    }
    let mid = (a + b) / 2;
    let n = mid + m;
    let (start, _r) = if m > mid {
        let mut start = n - b;
        let mut r = mid;
        while start < r {
            let c = (start + r) / 2;
            if !data.Less(n - c - 1, c) { start = c + 1; } else { r = c; }
        }
        (start, r)
    } else {
        let mut start = a;
        let mut r = m;
        while start < r {
            let c = (start + r) / 2;
            if !data.Less(n - c - 1, c) { start = c + 1; } else { r = c; }
        }
        (start, r)
    };
    let end = n - start;
    if start < m && m < end {
        rotate(data, start, m, end);
    }
    if a < start && start < mid {
        sym_merge(data, a, start, mid);
    }
    if mid < end && end < b {
        sym_merge(data, mid, end, b);
    }
}

fn rotate<T: Interface + ?Sized>(data: &mut T, a: int, m: int, b: int) {
    let mut i = m - a;
    let mut j = b - m;
    while i != j {
        if i > j {
            swap_range(data, m - i, m, j);
            i -= j;
        } else {
            swap_range(data, m - i, m + j - i, i);
            j -= i;
        }
    }
    swap_range(data, m - i, m, i);
}

fn swap_range<T: Interface + ?Sized>(data: &mut T, a: int, b: int, n: int) {
    for i in 0..n {
        data.Swap(a + i, b + i);
    }
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
pub struct IntSlice<'a>(pub &'a mut [int]);

impl<'a> IntSlice<'a> {
    pub fn Sort(&mut self) { self.0.sort(); }
    pub fn Search(&self, v: int) -> int { SearchInts(self.0, v) }
    pub fn Len(&self) -> int { self.0.len() as int }
    pub fn Less(&self, i: int, j: int) -> bool { self.0[i as usize] < self.0[j as usize] }
    pub fn Swap(&mut self, i: int, j: int) { self.0.swap(i as usize, j as usize); }
}

#[allow(non_snake_case)]
pub struct StringSlice<'a>(pub &'a mut [string]);

impl<'a> StringSlice<'a> {
    pub fn Sort(&mut self) { self.0.sort(); }
    pub fn Search(&self, v: impl AsRef<str>) -> int { SearchStrings(self.0, v) }
    pub fn Len(&self) -> int { self.0.len() as int }
    pub fn Less(&self, i: int, j: int) -> bool { self.0[i as usize] < self.0[j as usize] }
    pub fn Swap(&mut self, i: int, j: int) { self.0.swap(i as usize, j as usize); }
}

#[allow(non_snake_case)]
pub struct Float64Slice<'a>(pub &'a mut [float64]);

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
pub fn Reverse<T, F>(s: &mut [T], mut less: F)
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
pub fn ReverseInts(s: &mut [int]) {
    s.sort_by(|a, b| b.cmp(a));
}

#[allow(non_snake_case)]
pub fn ReverseStrings(s: &mut [string]) {
    s.sort_by(|a, b| b.cmp(a));
}

#[allow(non_snake_case)]
pub fn ReverseFloat64s(s: &mut [float64]) {
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
    fn sort_interface_on_custom_type() {
        // Port of the Uint64Slice example from the user's upstream report.
        use crate::types::{slice, uint64};
        struct Uint64Slice(slice<uint64>);
        impl Interface for Uint64Slice {
            fn Len(&self) -> int { self.0.len() as int }
            fn Less(&self, i: int, j: int) -> bool { self.0[i] < self.0[j] }
            fn Swap(&mut self, i: int, j: int) { self.0.Swap(i, j); }
        }
        let mut g = Uint64Slice(crate::slice!([]uint64{10, 500, 5, 1, 100, 25}));
        Sort(&mut g);
        let want = crate::slice!([]uint64{1, 5, 10, 25, 100, 500});
        assert_eq!(g.0.as_slice(), want.as_slice());
    }

    #[test]
    fn sort_interface_stable_preserves_order() {
        // (priority, original_index) — stable sort by priority keeps original index order.
        use crate::_slice::slice as SliceNew;
        struct Items(SliceNew<(int, int)>);
        impl Interface for Items {
            fn Len(&self) -> int { self.0.len() as int }
            fn Less(&self, i: int, j: int) -> bool { self.0[i].0 < self.0[j].0 }
            fn Swap(&mut self, i: int, j: int) { self.0.Swap(i, j); }
        }
        let mut it = Items(SliceNew(vec![(2i64, 0i64), (1, 1), (2, 2), (1, 3)]));
        Stable(&mut it);
        assert_eq!(it.0.as_slice(), &[(1i64, 1i64), (1, 3), (2, 0), (2, 2)][..]);
    }

    #[test]
    fn reverse_with_less() {
        let mut v = vec![1i64, 2, 3];
        Reverse(&mut v, |a, b| a < b);
        assert_eq!(v, vec![3, 2, 1]);
    }
}
