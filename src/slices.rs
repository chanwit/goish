//! slices: Go's slices package — generic slice operations.
//!
//!   Go                                  goish
//!   ─────────────────────────────────   ──────────────────────────────────
//!   slices.Contains(s, v)               slices::Contains(&s, &v)
//!   slices.Index(s, v)                  slices::Index(&s, &v)
//!   slices.Sort(s)                      slices::Sort(&mut s)
//!   slices.SortFunc(s, cmp)             slices::SortFunc(&mut s, cmp)
//!   slices.Reverse(s)                   slices::Reverse(&mut s)
//!   slices.Clone(s)                     slices::Clone(&s)
//!   slices.Max(s)                       slices::Max(&s)
//!   slices.Min(s)                       slices::Min(&s)
//!   slices.BinarySearch(s, v)           slices::BinarySearch(&s, &v)
//!   slices.Insert(s, i, vs...)          slices::Insert(&mut s, i, &[vs...])
//!   slices.Delete(s, i, j)              slices::Delete(&mut s, i, j)
//!   slices.Compact(s)                   slices::Compact(&mut s)
//!   slices.Concat(ss...)                slices::Concat(&[&s1, &s2, ...])
//!   slices.Equal(a, b)                  slices::Equal(&a, &b)

use crate::types::int;

// ── Search / equality ────────────────────────────────────────────────

/// `slices.Equal(s1, s2)` — true iff same length and element-wise equal.
#[allow(non_snake_case)]
pub fn Equal<T: PartialEq>(s1: &[T], s2: &[T]) -> bool {
    s1 == s2
}

/// `slices.EqualFunc(s1, s2, eq)` — element-wise equality via predicate.
#[allow(non_snake_case)]
pub fn EqualFunc<T, U, F: Fn(&T, &U) -> bool>(s1: &[T], s2: &[U], eq: F) -> bool {
    if s1.len() != s2.len() { return false; }
    s1.iter().zip(s2).all(|(a, b)| eq(a, b))
}

/// `slices.Compare(s1, s2)` — lexicographic comparison; -1 / 0 / +1.
#[allow(non_snake_case)]
pub fn Compare<T: PartialOrd>(s1: &[T], s2: &[T]) -> int {
    for (a, b) in s1.iter().zip(s2) {
        let c = crate::cmp::Compare(a, b);
        if c != 0 { return c; }
    }
    if s1.len() < s2.len() { -1 }
    else if s1.len() > s2.len() { 1 }
    else { 0 }
}

/// `slices.Index(s, v)` — first index of v, or -1.
#[allow(non_snake_case)]
pub fn Index<T: PartialEq>(s: &[T], v: &T) -> int {
    match s.iter().position(|x| x == v) {
        Some(i) => i as int,
        None => -1,
    }
}

/// `slices.IndexFunc(s, f)` — first index where f returns true, or -1.
#[allow(non_snake_case)]
pub fn IndexFunc<T, F: Fn(&T) -> bool>(s: &[T], f: F) -> int {
    match s.iter().position(f) {
        Some(i) => i as int,
        None => -1,
    }
}

/// `slices.Contains(s, v)` — reports whether v is present.
#[allow(non_snake_case)]
pub fn Contains<T: PartialEq>(s: &[T], v: &T) -> bool {
    s.iter().any(|x| x == v)
}

/// `slices.ContainsFunc(s, f)` — reports whether any element satisfies f.
#[allow(non_snake_case)]
pub fn ContainsFunc<T, F: Fn(&T) -> bool>(s: &[T], f: F) -> bool {
    s.iter().any(f)
}

// ── Structural mutations ─────────────────────────────────────────────

/// `slices.Insert(s, i, values...)` — inserts values at position i,
/// shifting the tail right. Panics if i is out of range.
#[allow(non_snake_case)]
pub fn Insert<T: Clone>(s: &mut crate::types::slice<T>, i: int, values: &[T]) {
    let i = i as usize;
    if i > s.len() { panic!("slices.Insert: index out of range"); }
    let mut v: Vec<T> = std::mem::take(s).into_vec();
    let mut splice: Vec<T> = values.to_vec();
    let tail: Vec<T> = v.drain(i..).collect();
    v.append(&mut splice);
    v.extend(tail);
    *s = v.into();
}

/// `slices.Delete(s, i, j)` — removes elements s[i..j]. Panics if out of range.
#[allow(non_snake_case)]
pub fn Delete<T: Clone>(s: &mut crate::types::slice<T>, i: int, j: int) {
    let i = i as usize;
    let j = j as usize;
    if i > j || j > s.len() { panic!("slices.Delete: invalid range"); }
    let mut v: Vec<T> = std::mem::take(s).into_vec();
    v.drain(i..j);
    *s = v.into();
}

/// `slices.DeleteFunc(s, del)` — removes every element for which del
/// returns true; preserves order.
#[allow(non_snake_case)]
pub fn DeleteFunc<T: Clone, F: FnMut(&T) -> bool>(s: &mut crate::types::slice<T>, mut del: F) {
    let mut v: Vec<T> = std::mem::take(s).into_vec();
    v.retain(|x| !del(x));
    *s = v.into();
}

/// `slices.Replace(s, i, j, values...)` — replaces s[i..j] with values.
#[allow(non_snake_case)]
pub fn Replace<T: Clone>(s: &mut crate::types::slice<T>, i: int, j: int, values: &[T]) {
    let i = i as usize;
    let j = j as usize;
    if i > j || j > s.len() { panic!("slices.Replace: invalid range"); }
    let mut v: Vec<T> = std::mem::take(s).into_vec();
    v.splice(i..j, values.iter().cloned());
    *s = v.into();
}

/// `slices.Clone(s)` — shallow copy.
#[allow(non_snake_case)]
pub fn Clone<T: Clone>(s: &[T]) -> crate::types::slice<T> {
    s.to_vec().into()
}

/// `slices.Compact(s)` — removes consecutive runs of duplicates.
#[allow(non_snake_case)]
pub fn Compact<T: Clone + PartialEq>(s: &mut crate::types::slice<T>) {
    let mut v: Vec<T> = std::mem::take(s).into_vec();
    v.dedup();
    *s = v.into();
}

/// `slices.CompactFunc(s, eq)` — compact using custom equality.
#[allow(non_snake_case)]
pub fn CompactFunc<T: Clone, F: FnMut(&mut T, &mut T) -> bool>(s: &mut crate::types::slice<T>, eq: F) {
    let mut v: Vec<T> = std::mem::take(s).into_vec();
    v.dedup_by(eq);
    *s = v.into();
}

/// `slices.Concat(ss...)` — flattens several slices into a new slice.
#[allow(non_snake_case)]
pub fn Concat<T: Clone>(slices: &[&[T]]) -> crate::types::slice<T> {
    let total: usize = slices.iter().map(|s| s.len()).sum();
    let mut out: Vec<T> = Vec::with_capacity(total);
    for s in slices { out.extend_from_slice(s); }
    out.into()
}

/// `slices.Repeat(x, count)` — concatenates x with itself `count` times.
#[allow(non_snake_case)]
pub fn Repeat<T: Clone>(x: &[T], count: int) -> crate::types::slice<T> {
    if count <= 0 { return crate::types::slice::new(); }
    let mut out: Vec<T> = Vec::with_capacity(x.len() * count as usize);
    for _ in 0..count { out.extend_from_slice(x); }
    out.into()
}

/// `slices.Reverse(s)` — reverses s in place.
#[allow(non_snake_case)]
pub fn Reverse<T>(s: &mut [T]) {
    s.reverse();
}

// ── Sorting ──────────────────────────────────────────────────────────

/// `slices.Sort(s)` — in-place ascending sort. For floats, NaNs sort last
/// (Go's behavior: NaNs are ordered equal among themselves and less than
/// all non-NaN via cmp.Less, but Rust's sort requires total order).
#[allow(non_snake_case)]
pub fn Sort<T: PartialOrd>(s: &mut [T]) {
    s.sort_by(|a, b| {
        let c = crate::cmp::Compare(a, b);
        if c < 0 { std::cmp::Ordering::Less }
        else if c > 0 { std::cmp::Ordering::Greater }
        else { std::cmp::Ordering::Equal }
    });
}

/// `slices.SortFunc(s, cmp)` — sort using user comparator that returns
/// -1 / 0 / +1.
#[allow(non_snake_case)]
pub fn SortFunc<T, F: FnMut(&T, &T) -> int>(s: &mut [T], mut cmp: F) {
    s.sort_by(|a, b| {
        let c = cmp(a, b);
        if c < 0 { std::cmp::Ordering::Less }
        else if c > 0 { std::cmp::Ordering::Greater }
        else { std::cmp::Ordering::Equal }
    });
}

/// `slices.SortStableFunc(s, cmp)` — like SortFunc, preserving equal-key order.
#[allow(non_snake_case)]
pub fn SortStableFunc<T, F: FnMut(&T, &T) -> int>(s: &mut [T], mut cmp: F) {
    s.sort_by(|a, b| {
        let c = cmp(a, b);
        if c < 0 { std::cmp::Ordering::Less }
        else if c > 0 { std::cmp::Ordering::Greater }
        else { std::cmp::Ordering::Equal }
    });
    // Rust's sort_by is already stable, so this is identical in practice.
}

/// `slices.IsSorted(s)` — reports whether s is ascending.
#[allow(non_snake_case)]
pub fn IsSorted<T: PartialOrd>(s: &[T]) -> bool {
    s.windows(2).all(|w| crate::cmp::Compare(&w[0], &w[1]) <= 0)
}

/// `slices.IsSortedFunc(s, cmp)` — reports whether s is sorted by cmp.
#[allow(non_snake_case)]
pub fn IsSortedFunc<T, F: FnMut(&T, &T) -> int>(s: &[T], mut cmp: F) -> bool {
    s.windows(2).all(|w| cmp(&w[0], &w[1]) <= 0)
}

/// `slices.Min(s)` — minimum value. Panics on empty.
#[allow(non_snake_case)]
pub fn Min<T: PartialOrd + Clone>(s: &[T]) -> T {
    if s.is_empty() { panic!("slices.Min: empty slice"); }
    let mut best = &s[0];
    for x in &s[1..] {
        if crate::cmp::Compare(x, best) < 0 { best = x; }
    }
    best.clone()
}

/// `slices.Max(s)` — maximum value. Panics on empty.
#[allow(non_snake_case)]
pub fn Max<T: PartialOrd + Clone>(s: &[T]) -> T {
    if s.is_empty() { panic!("slices.Max: empty slice"); }
    let mut best = &s[0];
    for x in &s[1..] {
        if crate::cmp::Compare(x, best) > 0 { best = x; }
    }
    best.clone()
}

/// `slices.MinFunc(s, cmp)` / `slices.MaxFunc(s, cmp)` — user-comparator variants.
#[allow(non_snake_case)]
pub fn MinFunc<T: Clone, F: FnMut(&T, &T) -> int>(s: &[T], mut cmp: F) -> T {
    if s.is_empty() { panic!("slices.MinFunc: empty slice"); }
    let mut best = &s[0];
    for x in &s[1..] {
        if cmp(x, best) < 0 { best = x; }
    }
    best.clone()
}

#[allow(non_snake_case)]
pub fn MaxFunc<T: Clone, F: FnMut(&T, &T) -> int>(s: &[T], mut cmp: F) -> T {
    if s.is_empty() { panic!("slices.MaxFunc: empty slice"); }
    let mut best = &s[0];
    for x in &s[1..] {
        if cmp(x, best) > 0 { best = x; }
    }
    best.clone()
}

/// `slices.BinarySearch(s, target)` — returns (idx, found). s must be sorted.
#[allow(non_snake_case)]
pub fn BinarySearch<T: PartialOrd>(s: &[T], target: &T) -> (int, bool) {
    let mut lo = 0usize;
    let mut hi = s.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if crate::cmp::Compare(&s[mid], target) < 0 {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    let found = lo < s.len() && crate::cmp::Compare(&s[lo], target) == 0;
    (lo as int, found)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_and_index() {
        let a = [1i64, 2, 3];
        let b = [1i64, 2, 3];
        let c = [1i64, 2, 4];
        assert!(Equal(&a, &b));
        assert!(!Equal(&a, &c));
        assert_eq!(Index(&a, &2), 1);
        assert_eq!(Index(&a, &99), -1);
        assert!(Contains(&a, &3));
        assert!(!Contains(&a, &10));
    }

    #[test]
    fn sort_and_search() {
        let mut s = vec![3i64, 1, 4, 1, 5, 9, 2, 6];
        Sort(&mut s);
        assert_eq!(s, vec![1i64, 1, 2, 3, 4, 5, 6, 9]);
        assert!(IsSorted(&s));
        let (i, found) = BinarySearch(&s, &5);
        assert!(found);
        assert_eq!(i, 5);
    }

    #[test]
    fn min_max() {
        assert_eq!(Min(&[3i64, 1, 4]), 1);
        assert_eq!(Max(&[3i64, 1, 4]), 4);
    }

    #[test]
    fn insert_delete_reverse() {
        let mut s: crate::types::slice<i64> = vec![1i64, 2, 4].into();
        Insert(&mut s, 2, &[3]);
        assert_eq!(s.as_slice(), &[1, 2, 3, 4]);
        Delete(&mut s, 1, 3);
        assert_eq!(s.as_slice(), &[1i64, 4]);
        Reverse(s.as_mut_slice());
        assert_eq!(s.as_slice(), &[4i64, 1]);
    }

    #[test]
    fn compact_concat_clone() {
        let mut s: crate::types::slice<i64> = vec![1i64, 1, 2, 3, 3, 3, 4].into();
        Compact(&mut s);
        assert_eq!(s.as_slice(), &[1, 2, 3, 4]);
        let c: crate::types::slice<i64> = Concat(&[&[1, 2], &[3, 4]]);
        assert_eq!(c.as_slice(), &[1i64, 2, 3, 4]);
        let cl = Clone(s.as_slice());
        assert_eq!(cl.as_slice(), s.as_slice());
    }
}
