// slice<T>: newtype wrapper around Vec<T> so we can impl Index<i64>.
//
// Go's `[]T` indexes with `int` (i64 in goish). Vec<T> indexes with usize.
// The orphan rule blocks `impl Index<i64> for Vec<T>` (both foreign), so
// we newtype:
//
//   pub struct slice<T>(pub Vec<T>);
//
// The Vec API stays reachable through Deref/DerefMut, and From<Vec<T>>
// plus Into<Vec<T>> keep boundary crossing cheap.

#![allow(non_camel_case_types)]

use std::ops::{Deref, DerefMut, Index, IndexMut,
               Range, RangeFrom, RangeTo, RangeFull, RangeInclusive, RangeToInclusive};

/// Go's `[]T`. Thin newtype around `Vec<T>`; all Vec methods reachable via
/// `Deref`. Adds `Index<i64>` so `ss[i]` works with Go's `int` index type.
#[repr(transparent)]
pub struct slice<T>(pub Vec<T>);

impl<T> slice<T> {
    pub fn new() -> Self { slice(Vec::new()) }
    pub fn with_capacity(n: usize) -> Self { slice(Vec::with_capacity(n)) }
    pub fn into_vec(self) -> Vec<T> { self.0 }
    pub fn as_vec(&self) -> &Vec<T> { &self.0 }
    pub fn as_vec_mut(&mut self) -> &mut Vec<T> { &mut self.0 }

    /// Go's `s[i:j]` — returns an owned slice over `[i, j)`. O(n) copy
    /// (Rust can't share backing arrays across owned slices without Arc).
    #[allow(non_snake_case)]
    pub fn Slice(&self, i: i64, j: i64) -> Self where T: Clone {
        let n = self.0.len() as i64;
        if i < 0 || j < 0 || i > j || j > n {
            panic!("runtime error: slice bounds out of range [:{}] with length {}", j, n);
        }
        slice(self.0[i as usize..j as usize].to_vec())
    }

    /// Go's `s[i:]` — slice from index i to end.
    #[allow(non_snake_case)]
    pub fn SliceFrom(&self, i: i64) -> Self where T: Clone {
        self.Slice(i, self.0.len() as i64)
    }

    /// Go's `s[:j]` — slice from start to index j.
    #[allow(non_snake_case)]
    pub fn SliceTo(&self, j: i64) -> Self where T: Clone {
        self.Slice(0, j)
    }

    /// Go's `s[i], s[j] = s[j], s[i]` — swap two elements by index.
    /// Indexed by Go's `int` (i64). Panics on out-of-range, matching Go.
    #[allow(non_snake_case)]
    pub fn Swap(&mut self, i: i64, j: i64) {
        let n = self.0.len();
        if i < 0 || (i as u64) >= n as u64 {
            panic!("runtime error: index out of range [{}] with length {}", i, n);
        }
        if j < 0 || (j as u64) >= n as u64 {
            panic!("runtime error: index out of range [{}] with length {}", j, n);
        }
        self.0.swap(i as usize, j as usize);
    }
}

// ── Deref / AsRef ─────────────────────────────────────────────────────

impl<T> Deref for slice<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Vec<T> { &self.0 }
}
impl<T> DerefMut for slice<T> {
    fn deref_mut(&mut self) -> &mut Vec<T> { &mut self.0 }
}
impl<T> AsRef<[T]> for slice<T> { fn as_ref(&self) -> &[T] { &self.0 } }
impl<T> AsMut<[T]> for slice<T> { fn as_mut(&mut self) -> &mut [T] { &mut self.0 } }
impl<T> AsRef<Vec<T>> for slice<T> { fn as_ref(&self) -> &Vec<T> { &self.0 } }
impl<T> std::borrow::Borrow<[T]> for slice<T> {
    fn borrow(&self) -> &[T] { &self.0 }
}

// ── Conversions ───────────────────────────────────────────────────────

impl<T> From<Vec<T>> for slice<T> {
    fn from(v: Vec<T>) -> Self { slice(v) }
}
impl<T> From<slice<T>> for Vec<T> {
    fn from(s: slice<T>) -> Vec<T> { s.0 }
}
impl<T: Clone> From<&[T]> for slice<T> {
    fn from(s: &[T]) -> Self { slice(s.to_vec()) }
}
impl<T, const N: usize> From<[T; N]> for slice<T> {
    fn from(a: [T; N]) -> Self { slice(Vec::from(a)) }
}

// ── Indexing: Go's `s[i]` where i: int (i64) ──────────────────────────
//
// The panic messages mirror Go's runtime error format.

impl<T> Index<i64> for slice<T> {
    type Output = T;
    fn index(&self, i: i64) -> &T {
        if i < 0 || (i as u64) >= self.0.len() as u64 {
            panic!("runtime error: index out of range [{}] with length {}", i, self.0.len());
        }
        &self.0[i as usize]
    }
}
impl<T> IndexMut<i64> for slice<T> {
    fn index_mut(&mut self, i: i64) -> &mut T {
        let n = self.0.len();
        if i < 0 || (i as u64) >= n as u64 {
            panic!("runtime error: index out of range [{}] with length {}", i, n);
        }
        &mut self.0[i as usize]
    }
}

// No Index<usize> — having both `Index<i64>` and `Index<usize>` makes
// literal `ss[0]` ambiguous (Rust falls back to i32, which matches neither).
// Callers with a `usize` index (e.g. from `.iter().enumerate()`) cast to i64
// or use `ss.as_vec()[i]` / `ss.as_slice()[i]` to hit Vec's built-in impl.

// Range flavours — without these, having any Index impl above blocks
// Deref-based auto-resolution of `ss[a..b]`.
macro_rules! impl_slice_range {
    ($($r:ty),+ $(,)?) => { $(
        impl<T> Index<$r> for slice<T> {
            type Output = [T];
            fn index(&self, r: $r) -> &[T] { &self.0[r] }
        }
        impl<T> IndexMut<$r> for slice<T> {
            fn index_mut(&mut self, r: $r) -> &mut [T] { &mut self.0[r] }
        }
    )+ };
}
impl_slice_range!(
    Range<usize>, RangeTo<usize>, RangeFrom<usize>,
    RangeFull, RangeInclusive<usize>, RangeToInclusive<usize>,
);

// ── Iteration ─────────────────────────────────────────────────────────

impl<T> IntoIterator for slice<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter { self.0.into_iter() }
}
impl<'a, T> IntoIterator for &'a slice<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter { self.0.iter() }
}
impl<'a, T> IntoIterator for &'a mut slice<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;
    fn into_iter(self) -> Self::IntoIter { self.0.iter_mut() }
}
impl<T> FromIterator<T> for slice<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        slice(Vec::from_iter(iter))
    }
}
impl<T> Extend<T> for slice<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) { self.0.extend(iter); }
}

// ── Derived-like traits (conditional on T) ────────────────────────────

impl<T: Clone> Clone for slice<T> {
    fn clone(&self) -> Self { slice(self.0.clone()) }
}
impl<T: std::fmt::Debug> std::fmt::Debug for slice<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// `Display` for byte slices — matches Go's `fmt.Sprintf("%s", bytes)`:
/// prints the bytes as lossy UTF-8 (invalid sequences → U+FFFD).
/// With `%q` verb, `Sprintf!` wraps the result in quotes.
impl std::fmt::Display for slice<u8> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&String::from_utf8_lossy(&self.0))
    }
}
impl<T> Default for slice<T> {
    fn default() -> Self { slice(Vec::new()) }
}
impl<T: PartialEq> PartialEq for slice<T> {
    fn eq(&self, other: &Self) -> bool { self.0 == other.0 }
}
impl<T: Eq> Eq for slice<T> {}
impl<T: std::hash::Hash> std::hash::Hash for slice<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) { self.0.hash(state); }
}
impl<T: PartialOrd> PartialOrd for slice<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}
impl<T: Ord> Ord for slice<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering { self.0.cmp(&other.0) }
}

// Cross-type equality — mirrors Vec<T>'s own cross-type impls, so tests can
// compare `slice<GoString>` against `Vec<&str>` / `[&str; N]` / `&[&str]`.
impl<T, U> PartialEq<Vec<U>> for slice<T> where T: PartialEq<U> {
    fn eq(&self, other: &Vec<U>) -> bool { self.0.as_slice() == other.as_slice() }
}
impl<T, U> PartialEq<slice<U>> for Vec<T> where T: PartialEq<U> {
    fn eq(&self, other: &slice<U>) -> bool { self.as_slice() == other.0.as_slice() }
}
impl<T, U> PartialEq<[U]> for slice<T> where T: PartialEq<U> {
    fn eq(&self, other: &[U]) -> bool { self.0.as_slice() == other }
}
impl<T, U, const N: usize> PartialEq<[U; N]> for slice<T> where T: PartialEq<U> {
    fn eq(&self, other: &[U; N]) -> bool { self.0.as_slice() == other.as_slice() }
}
impl<T, U, const N: usize> PartialEq<&[U; N]> for slice<T> where T: PartialEq<U> {
    fn eq(&self, other: &&[U; N]) -> bool { self.0.as_slice() == other.as_slice() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_by_i64() {
        let s: slice<i64> = slice(vec![10, 20, 30]);
        let i: i64 = 1;
        assert_eq!(s[i], 20);
        assert_eq!(s[0i64], 10);
    }

    #[test]
    #[should_panic]
    fn index_negative_panics() {
        let s: slice<i64> = slice(vec![1, 2, 3]);
        let _ = s[-1i64];
    }

    #[test]
    fn index_range_returns_slice() {
        let s: slice<i64> = slice(vec![1, 2, 3, 4, 5]);
        assert_eq!(&s[1..3], &[2, 3]);
    }

    #[test]
    fn loop_over_len_indexes_naturally() {
        let mut ss: slice<String> = slice(vec!["a".into(), "b".into(), "c".into()]);
        ss.sort();
        for i in 1..ss.len() as i64 {
            assert!(ss[i - 1] <= ss[i]);
        }
    }

    #[test]
    fn deref_vec_methods_work() {
        let mut s: slice<i64> = slice::new();
        s.push(1);
        s.push(2);
        assert_eq!(s.len(), 2);
        let sum: i64 = s.iter().sum();
        assert_eq!(sum, 3);
    }

    #[test]
    fn iterate_consumed_and_borrowed() {
        let s: slice<i64> = slice(vec![1, 2, 3]);
        let borrowed: i64 = (&s).into_iter().sum();
        assert_eq!(borrowed, 6);
        let owned: i64 = s.into_iter().sum();
        assert_eq!(owned, 6);
    }

    #[test]
    fn slice_from_to_and_slice() {
        let s: slice<i64> = slice(vec![10, 20, 30, 40, 50]);
        // Go: parts[2:]   → [30, 40, 50]
        assert_eq!(s.SliceFrom(2).as_vec(), &vec![30, 40, 50]);
        // Go: parts[:2]   → [10, 20]
        assert_eq!(s.SliceTo(2).as_vec(), &vec![10, 20]);
        // Go: parts[1:4]  → [20, 30, 40]
        assert_eq!(s.Slice(1, 4).as_vec(), &vec![20, 30, 40]);
        // Boundary: s[n:n] is empty, not an error.
        assert_eq!(s.SliceFrom(5).as_vec(), &Vec::<i64>::new());
    }

    #[test]
    #[should_panic]
    fn slice_out_of_range_panics() {
        let s: slice<i64> = slice(vec![1, 2, 3]);
        let _ = s.SliceFrom(10);
    }

    #[test]
    fn swap_by_int_indices() {
        let mut s: slice<i64> = slice(vec![10, 20, 30]);
        s.Swap(0i64, 2i64);
        assert_eq!(s.as_vec(), &vec![30, 20, 10]);
    }

    #[test]
    #[should_panic]
    fn swap_out_of_range_panics() {
        let mut s: slice<i64> = slice(vec![1, 2]);
        s.Swap(0, 5);
    }

    #[test]
    fn byte_slice_display() {
        // slice<u8> impls Display → Sprintf!("%s", bytes) / "%q" work.
        let b: slice<u8> = slice(b"hello".to_vec());
        assert_eq!(format!("{}", b), "hello");
        // Invalid UTF-8 → replacement char, not panic.
        let bad: slice<u8> = slice(vec![0xff, b'x']);
        assert!(format!("{}", bad).contains('x'));
    }

    #[test]
    fn from_vec_roundtrip() {
        let v = vec![1i64, 2, 3];
        let s: slice<i64> = v.into();
        let v2: Vec<i64> = s.into();
        assert_eq!(v2, vec![1, 2, 3]);
    }
}
