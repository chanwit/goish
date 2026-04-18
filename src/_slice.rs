// slice<T>: Go's `[]T`, implemented as a reference-counted view over a
// shared Vec<T> — matches Go's O(1) re-slice semantics.
//
// Layout (24 bytes on 64-bit, same as Go's slice header):
//
//   pub struct slice<T> {
//       data:  Arc<Vec<T>>,  // shared backing array (SliceHeader.Data)
//       start: usize,        // offset into `data`
//       len:   usize,        // view length (SliceHeader.Len)
//   }
//
// Go semantics matched:
//   - s[i:j]  — O(1) header clone, same backing array
//   - s[i]    — O(1) read through (data[start + i])
//   - len(s)  — O(1), returns the view length, not data.len()
//
// Rust safety divergence (documented):
//   - Go allows writing through a sub-slice to modify the parent's view
//     of the backing array (shared mutable memory). Rust's borrow checker
//     forbids that without interior mutability. We use Arc::make_mut,
//     which performs copy-on-write: a mutation on a shared slice clones
//     the backing Vec first, then mutates the clone. The parent keeps
//     the original. This is closer to Rust's Cow<[T]> than Go's true
//     sharing, but covers the common cases (read-only sub-slices,
//     discard-prefix patterns) at O(1).
//
//   - `append`/`push` always succeeds: if unique, grows in place; if
//     shared, make_mut clones first. No separate `cap` model.
//
// Constructor:  `slice(v: Vec<T>)` is a free function (not a tuple
// struct constructor) so the old call-site shape `slice(vec![1,2,3])`
// still works.

#![allow(non_camel_case_types)]

use std::ops::{Deref, DerefMut, Index, IndexMut,
               Range, RangeFrom, RangeTo, RangeFull, RangeInclusive, RangeToInclusive};
use std::sync::Arc;

/// Go's `[]T`. Arc-backed view with O(1) re-slicing.
pub struct slice<T> {
    data: Arc<Vec<T>>,
    start: usize,
    len: usize,
}

/// `slice(v)` — wrap a Vec<T> as a goish slice. Free function so the
/// call shape matches the previous tuple-struct constructor.
#[allow(non_snake_case)]
pub fn slice<T>(v: Vec<T>) -> self::slice<T> {
    let len = v.len();
    self::slice { data: Arc::new(v), start: 0, len }
}

impl<T> slice<T> {
    /// Empty slice. `len() == 0`, shares a single static-style empty Arc.
    pub fn new() -> Self {
        self::slice { data: Arc::new(Vec::new()), start: 0, len: 0 }
    }

    pub fn with_capacity(n: usize) -> Self {
        self::slice { data: Arc::new(Vec::with_capacity(n)), start: 0, len: 0 }
    }

    /// View length — number of elements from `start` onward. Matches `len(s)`.
    pub fn len(&self) -> usize { self.len }
    pub fn is_empty(&self) -> bool { self.len == 0 }

    /// Go's `cap(s)` — elements available before backing would need to grow.
    /// Always ≥ len. For unique Arc, this is `data.capacity() - start`.
    pub fn capacity(&self) -> usize {
        self.data.capacity().saturating_sub(self.start).max(self.len)
    }

    /// Borrow the view as a Rust slice `&[T]`.
    pub fn as_slice(&self) -> &[T] {
        &self.data[self.start..self.start + self.len]
    }

    /// Borrow the view as a mutable Rust slice — triggers copy-on-write
    /// if the backing Arc is shared.
    pub fn as_mut_slice(&mut self) -> &mut [T] where T: Clone {
        self.uniq();
        let vec = Arc::get_mut(&mut self.data)
            .expect("slice::uniq() should leave Arc unique");
        &mut vec[self.start..self.start + self.len]
    }

    /// Convert into an owned `Vec<T>` (the view range only).
    pub fn into_vec(self) -> Vec<T> where T: Clone {
        let slice { data, start, len } = self;
        if start == 0 && len == data.len() {
            match Arc::try_unwrap(data) {
                Ok(v) => v,
                Err(arc) => (*arc).clone(),
            }
        } else {
            data[start..start + len].to_vec()
        }
    }

    /// Normalize without cloning: panics if the Arc is shared.
    /// Drains prefix and truncates suffix so `data` is exactly the view
    /// and `start = 0`. Works for non-Clone T because it only moves
    /// elements within the Vec, never copies them.
    fn normalize(&mut self) {
        let shared = Arc::strong_count(&self.data) > 1
            || Arc::weak_count(&self.data) > 0;
        if shared {
            panic!("slice mutation on shared backing. Call `.cow()` first \
                    to fork (requires `T: Clone`), or use `append!` which \
                    auto-forks.");
        }
        let v = Arc::get_mut(&mut self.data).expect("unique");
        if self.start > 0 { v.drain(..self.start); self.start = 0; }
        if v.len() > self.len { v.truncate(self.len); }
    }

    /// CoW version — clones the backing if shared. Used by APIs that
    /// need T: Clone (e.g., IndexMut, as_mut_slice) to silently fork.
    fn uniq(&mut self) where T: Clone {
        let need_copy = Arc::strong_count(&self.data) > 1
            || Arc::weak_count(&self.data) > 0
            || self.start > 0
            || self.data.len() != self.len;
        if need_copy {
            let v = self.data[self.start..self.start + self.len].to_vec();
            self.data = Arc::new(v);
            self.start = 0;
        }
    }

    /// Explicitly fork the backing if shared — O(n) if shared, O(1) if
    /// unique. After `cow()`, this slice has exclusive ownership and
    /// subsequent mutations won't panic on shared-backing.
    ///
    /// Use this when you've sub-sliced a shared slice and want to
    /// mutate without panicking:
    ///
    /// ```ignore
    /// let sub = s.SliceFrom(1);
    /// sub.cow();       // fork — now unique
    /// sub.push(42);    // OK
    /// ```
    pub fn cow(&mut self) where T: Clone { self.uniq(); }

    /// Mutable Vec — unique path (panics if shared). Requires no bound
    /// on T.
    fn as_vec_owned(&mut self) -> &mut Vec<T> {
        self.normalize();
        Arc::get_mut(&mut self.data).expect("unique after normalize")
    }

    // ── Go-shape slicing: s[i:j], s[i:], s[:j] — all O(1) ─────────────

    /// Go's `s[i:j]` — O(1) header clone.
    #[allow(non_snake_case)]
    pub fn Slice(&self, i: i64, j: i64) -> Self {
        let n = self.len as i64;
        if i < 0 || j < 0 || i > j || j > n {
            panic!("runtime error: slice bounds out of range [{}:{}] with length {}",
                   i, j, n);
        }
        self::slice {
            data: Arc::clone(&self.data),
            start: self.start + i as usize,
            len: (j - i) as usize,
        }
    }

    /// Go's `s[i:]` — O(1).
    #[allow(non_snake_case)]
    pub fn SliceFrom(&self, i: i64) -> Self {
        self.Slice(i, self.len as i64)
    }

    /// Go's `s[:j]` — O(1).
    #[allow(non_snake_case)]
    pub fn SliceTo(&self, j: i64) -> Self {
        self.Slice(0, j)
    }

    /// Go's `s[i], s[j] = s[j], s[i]`.
    #[allow(non_snake_case)]
    pub fn Swap(&mut self, i: i64, j: i64) {
        let n = self.len;
        if i < 0 || (i as u64) >= n as u64 {
            panic!("runtime error: index out of range [{}] with length {}", i, n);
        }
        if j < 0 || (j as u64) >= n as u64 {
            panic!("runtime error: index out of range [{}] with length {}", j, n);
        }
        let v = self.as_vec_owned();
        v.swap(i as usize, j as usize);
    }

    // ── Vec-style mutation methods ────────────────────────────────────
    //
    // These DO NOT require T: Clone. They use the unique-or-panic path.
    // Shared slices must be forked via `.into_vec()` before mutation.

    pub fn push(&mut self, x: T) {
        let v = self.as_vec_owned();
        v.push(x);
        self.len += 1;
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 { return None; }
        let v = self.as_vec_owned();
        let r = v.pop();
        if r.is_some() { self.len -= 1; }
        r
    }

    pub fn insert(&mut self, idx: usize, x: T) {
        let v = self.as_vec_owned();
        v.insert(idx, x);
        self.len += 1;
    }

    pub fn remove(&mut self, idx: usize) -> T {
        let v = self.as_vec_owned();
        let r = v.remove(idx);
        self.len -= 1;
        r
    }

    pub fn swap_remove(&mut self, idx: usize) -> T {
        let v = self.as_vec_owned();
        let r = v.swap_remove(idx);
        self.len -= 1;
        r
    }

    pub fn clear(&mut self) {
        let v = self.as_vec_owned();
        v.clear();
        self.len = 0;
    }

    pub fn truncate(&mut self, new_len: usize) {
        if new_len < self.len {
            self.len = new_len;
        }
    }

    pub fn resize(&mut self, new_len: usize, value: T) where T: Clone {
        let v = self.as_vec_owned();
        v.resize(new_len, value);
        self.len = new_len;
    }

    pub fn resize_with<F>(&mut self, new_len: usize, f: F) where T: Clone, F: FnMut() -> T {
        let v = self.as_vec_owned();
        v.resize_with(new_len, f);
        self.len = new_len;
    }

    pub fn extend_from_slice(&mut self, other: &[T]) where T: Clone {
        let v = self.as_vec_owned();
        v.extend_from_slice(other);
        self.len += other.len();
    }

    pub fn retain<F>(&mut self, f: F) where F: FnMut(&T) -> bool {
        let v = self.as_vec_owned();
        v.retain(f);
        self.len = v.len();
    }

    pub fn sort(&mut self) where T: Ord {
        let v = self.as_vec_owned();
        v.sort();
    }
    pub fn sort_by<F>(&mut self, f: F) where F: FnMut(&T, &T) -> std::cmp::Ordering {
        let v = self.as_vec_owned();
        v.sort_by(f);
    }
    pub fn sort_unstable_by<F>(&mut self, f: F) where F: FnMut(&T, &T) -> std::cmp::Ordering {
        let v = self.as_vec_owned();
        v.sort_unstable_by(f);
    }
    pub fn reverse(&mut self) {
        let v = self.as_vec_owned();
        v.reverse();
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> { self.as_slice().iter() }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> where T: Clone {
        self.as_mut_slice().iter_mut()
    }

    pub fn first(&self) -> Option<&T> { self.as_slice().first() }
    pub fn last(&self) -> Option<&T> { self.as_slice().last() }
    pub fn contains(&self, x: &T) -> bool where T: PartialEq { self.as_slice().contains(x) }
}

// ── Deref / AsRef ─────────────────────────────────────────────────────

impl<T> Deref for slice<T> {
    type Target = [T];
    fn deref(&self) -> &[T] { self.as_slice() }
}
impl<T: Clone> DerefMut for slice<T> {
    fn deref_mut(&mut self) -> &mut [T] { self.as_mut_slice() }
}
impl<T> AsRef<[T]> for slice<T> { fn as_ref(&self) -> &[T] { self.as_slice() } }
impl<T: Clone> AsMut<[T]> for slice<T> { fn as_mut(&mut self) -> &mut [T] { self.as_mut_slice() } }
impl<T> std::borrow::Borrow<[T]> for slice<T> {
    fn borrow(&self) -> &[T] { self.as_slice() }
}

// ── Conversions ───────────────────────────────────────────────────────

impl<T> From<Vec<T>> for slice<T> {
    fn from(v: Vec<T>) -> Self {
        let len = v.len();
        slice { data: Arc::new(v), start: 0, len }
    }
}
impl<T: Clone> From<slice<T>> for Vec<T> {
    fn from(s: slice<T>) -> Vec<T> { s.into_vec() }
}
impl<T: Clone> From<&[T]> for slice<T> {
    fn from(s: &[T]) -> Self { self::slice::from(s.to_vec()) }
}
impl<T, const N: usize> From<[T; N]> for slice<T> {
    fn from(a: [T; N]) -> Self { self::slice::from(Vec::from(a)) }
}

// ── Indexing: s[i] with i: int (i64) — O(1) through start offset ──────

impl<T> Index<i64> for slice<T> {
    type Output = T;
    fn index(&self, i: i64) -> &T {
        if i < 0 || (i as u64) >= self.len as u64 {
            panic!("runtime error: index out of range [{}] with length {}", i, self.len);
        }
        &self.data[self.start + i as usize]
    }
}
impl<T: Clone> IndexMut<i64> for slice<T> {
    fn index_mut(&mut self, i: i64) -> &mut T {
        let n = self.len;
        if i < 0 || (i as u64) >= n as u64 {
            panic!("runtime error: index out of range [{}] with length {}", i, n);
        }
        &mut self.as_mut_slice()[i as usize]
    }
}

// Range flavours over usize — return a Rust slice view.
macro_rules! impl_slice_range {
    ($($r:ty),+ $(,)?) => { $(
        impl<T> Index<$r> for slice<T> {
            type Output = [T];
            fn index(&self, r: $r) -> &[T] { &self.as_slice()[r] }
        }
        impl<T: Clone> IndexMut<$r> for slice<T> {
            fn index_mut(&mut self, r: $r) -> &mut [T] { &mut self.as_mut_slice()[r] }
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
    /// Owned iteration. If the backing Arc is unique, unwraps it
    /// in-place (no clone). If shared, panics — caller should `.clone()`
    /// into a new slice first, or iterate by reference (`&slice`).
    fn into_iter(self) -> Self::IntoIter {
        let slice { data, start, len } = self;
        let mut v = Arc::try_unwrap(data).unwrap_or_else(|_| {
            panic!("slice::into_iter on a shared backing (strong_count > 1). \
                    Iterate by reference with `&s` / `range!(s)`, or clone into \
                    a separate Vec via `s.as_slice().to_vec()` first.")
        });
        if start > 0 { v.drain(..start); }
        v.truncate(len);
        v.into_iter()
    }
}
impl<'a, T> IntoIterator for &'a slice<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter { self.as_slice().iter() }
}
impl<'a, T: Clone> IntoIterator for &'a mut slice<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;
    fn into_iter(self) -> Self::IntoIter { self.as_mut_slice().iter_mut() }
}
impl<T> FromIterator<T> for slice<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        self::slice::from(Vec::from_iter(iter))
    }
}
impl<T: Clone> Extend<T> for slice<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        let v = self.as_vec_owned();
        let before = v.len();
        v.extend(iter);
        let added = v.len() - before;
        self.len += added;
    }
}

// ── Derived-like traits ───────────────────────────────────────────────

impl<T> Clone for slice<T> {
    /// O(1) — bumps the Arc refcount. Shared backing with the source.
    fn clone(&self) -> Self {
        slice {
            data: Arc::clone(&self.data),
            start: self.start,
            len: self.len,
        }
    }
}
impl<T: std::fmt::Debug> std::fmt::Debug for slice<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_slice().fmt(f)
    }
}

/// Generic Display for slice<T> — matches Go's `%v` format on slices:
/// `[elem1 elem2 elem3]` with single-space separators. Works for any
/// `T: Display`, including `u8` (which prints numerically, matching Go's
/// `%v` on `[]byte`). For byte→string conversion, use
/// `string::from(bytes)` or `String::from_utf8_lossy(bytes.as_slice())`.
impl<T: std::fmt::Display> std::fmt::Display for slice<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[")?;
        for (i, elem) in self.as_slice().iter().enumerate() {
            if i > 0 { f.write_str(" ")?; }
            <T as std::fmt::Display>::fmt(elem, f)?;
        }
        f.write_str("]")
    }
}
impl<T> Default for slice<T> {
    fn default() -> Self { Self::new() }
}
impl<T: PartialEq> PartialEq for slice<T> {
    fn eq(&self, other: &Self) -> bool { self.as_slice() == other.as_slice() }
}
impl<T: Eq> Eq for slice<T> {}
impl<T: std::hash::Hash> std::hash::Hash for slice<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) { self.as_slice().hash(state); }
}
impl<T: PartialOrd> PartialOrd for slice<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.as_slice().partial_cmp(other.as_slice())
    }
}
impl<T: Ord> Ord for slice<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_slice().cmp(other.as_slice())
    }
}

// Cross-type equality.
impl<T, U> PartialEq<Vec<U>> for slice<T> where T: PartialEq<U> {
    fn eq(&self, other: &Vec<U>) -> bool { self.as_slice() == other.as_slice() }
}
impl<T, U> PartialEq<slice<U>> for Vec<T> where T: PartialEq<U> {
    fn eq(&self, other: &slice<U>) -> bool { self.as_slice() == other.as_slice() }
}
impl<T, U> PartialEq<[U]> for slice<T> where T: PartialEq<U> {
    fn eq(&self, other: &[U]) -> bool { self.as_slice() == other }
}
impl<T, U, const N: usize> PartialEq<[U; N]> for slice<T> where T: PartialEq<U> {
    fn eq(&self, other: &[U; N]) -> bool { self.as_slice() == other.as_slice() }
}
impl<T, U, const N: usize> PartialEq<&[U; N]> for slice<T> where T: PartialEq<U> {
    fn eq(&self, other: &&[U; N]) -> bool { self.as_slice() == other.as_slice() }
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
    fn slice_from_to_and_slice_o1() {
        let s: slice<i64> = slice(vec![10, 20, 30, 40, 50]);
        assert_eq!(s.SliceFrom(2), vec![30, 40, 50]);
        assert_eq!(s.SliceTo(2), vec![10, 20]);
        assert_eq!(s.Slice(1, 4), vec![20, 30, 40]);
        assert_eq!(s.SliceFrom(5), Vec::<i64>::new());
    }

    #[test]
    fn reslice_shares_backing_on_read() {
        // O(1) — both views share the same Arc<Vec>.
        let s: slice<i64> = slice(vec![1, 2, 3, 4, 5]);
        let s2 = s.SliceFrom(1);
        assert_eq!(s2.len(), 4);
        // Original still intact.
        assert_eq!(s[0i64], 1);
        assert_eq!(s2[0i64], 2);
    }

    #[test]
    fn cow_on_write_to_subslice() {
        // Go allows s2[0] = 99 to mutate the parent's view. Rust's safety
        // demands copy-on-write: the mutation affects OUR clone, not the
        // parent. Different semantics, but documented.
        let s: slice<i64> = slice(vec![1, 2, 3, 4, 5]);
        let mut s2 = s.SliceFrom(1);
        s2[0i64] = 99;
        assert_eq!(s2[0i64], 99);
        // Parent unchanged (CoW):
        assert_eq!(s[1i64], 2);
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
        assert_eq!(s, vec![30, 20, 10]);
    }

    #[test]
    #[should_panic]
    fn swap_out_of_range_panics() {
        let mut s: slice<i64> = slice(vec![1, 2]);
        s.Swap(0, 5);
    }

    #[test]
    fn display_is_go_v_format() {
        // Go: fmt.Sprintf("%v", []byte{104,101}) → "[104 101]"
        // Not "he" — that's only for %s (type-aware verb).
        let b: slice<u8> = slice(b"he".to_vec());
        assert_eq!(format!("{}", b), "[104 101]");

        // Generic slice — same shape, any T: Display.
        let v: slice<i64> = slice(vec![1, 2, 3]);
        assert_eq!(format!("{}", v), "[1 2 3]");

        let s: slice<String> = slice(vec!["a".into(), "b".into()]);
        assert_eq!(format!("{}", s), "[a b]");
    }

    #[test]
    fn from_vec_roundtrip() {
        let v = vec![1i64, 2, 3];
        let s: slice<i64> = v.into();
        let v2: Vec<i64> = s.into();
        assert_eq!(v2, vec![1, 2, 3]);
    }

    #[test]
    fn push_pop_insert_remove() {
        let mut s: slice<i64> = slice::new();
        s.push(1);
        s.push(2);
        s.push(3);
        assert_eq!(s, vec![1, 2, 3]);
        assert_eq!(s.pop(), Some(3));
        assert_eq!(s, vec![1, 2]);
        s.insert(0, 0);
        assert_eq!(s, vec![0, 1, 2]);
        let r = s.remove(1);
        assert_eq!(r, 1);
        assert_eq!(s, vec![0, 2]);
    }

    #[test]
    fn clone_is_o1_shared_backing() {
        let s: slice<i64> = slice(vec![1, 2, 3, 4, 5]);
        let s2 = s.clone();
        // Both share the same Arc — strong count is 2.
        assert_eq!(Arc::strong_count(&s.data), 2);
        // Content identical.
        assert_eq!(s, s2);
    }

    #[test]
    fn slice_from_shares_arc() {
        // O(1) — SliceFrom bumps the Arc refcount, doesn't copy elements.
        let s: slice<i64> = slice(vec![1, 2, 3, 4, 5]);
        let before = Arc::strong_count(&s.data);
        let sub = s.SliceFrom(1);
        let after = Arc::strong_count(&s.data);
        assert_eq!(after, before + 1);
        assert_eq!(sub.as_slice(), &[2, 3, 4, 5]);
        assert_eq!(sub.start, 1);
        assert_eq!(sub.len, 4);
    }

    #[test]
    fn o1_large_slice_is_not_a_copy() {
        // Sanity check: slicing a million-element vec shouldn't scale with n.
        let big: slice<u64> = slice((0u64..1_000_000).collect());
        let t0 = std::time::Instant::now();
        for _ in 0..10_000 {
            let _ = big.SliceFrom(500_000);
        }
        let dt = t0.elapsed();
        // Should complete in well under a second on any reasonable machine —
        // if we were copying 500k u64s per call we'd spend seconds here.
        assert!(dt.as_millis() < 500, "SliceFrom on 1M-element slice 10k times took {:?} — suggests it's not O(1)", dt);
    }

    #[test]
    fn mutation_on_unique_slice_works() {
        // Non-Clone T works for push on unique slices.
        struct NonClone(i64);
        let mut s: slice<NonClone> = slice::new();
        s.push(NonClone(1));
        s.push(NonClone(2));
        assert_eq!(s.len(), 2);
    }

    #[test]
    #[should_panic(expected = "mutation on shared backing")]
    fn mutation_on_shared_slice_panics() {
        let s: slice<i64> = slice(vec![1, 2, 3]);
        let _view = s.clone();  // now shared
        let mut s = s;
        s.push(4);  // panics — can't mutate shared backing without T: Clone CoW
    }

    #[test]
    fn append_macro_cow_on_shared() {
        // append! auto-forks shared backing — matches Go's append never-fails.
        let s1: slice<i64> = slice(vec![1, 2, 3]);
        let s2 = s1.SliceFrom(1);  // shared with s1
        let s2 = crate::append!(s2, 99i64);
        assert_eq!(s2, vec![2, 3, 99]);
        // Parent unchanged (CoW):
        assert_eq!(s1, vec![1, 2, 3]);
    }

    #[test]
    fn cow_then_mutate_works_on_formerly_shared() {
        let s1: slice<i64> = slice(vec![1, 2, 3]);
        let mut s2 = s1.SliceFrom(1);  // shared
        s2.cow();                       // fork
        s2.push(99);                    // now works — unique backing
        assert_eq!(s2, vec![2, 3, 99]);
        assert_eq!(s1, vec![1, 2, 3]);
    }

    #[test]
    fn into_vec_on_unique_unwraps_without_clone() {
        let s: slice<i64> = slice(vec![1, 2, 3]);
        let v = s.into_vec();
        assert_eq!(v, vec![1, 2, 3]);
    }
}
