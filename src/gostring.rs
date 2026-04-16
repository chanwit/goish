// gostring: Go's `string` type ported to Rust.
//
// Internal layout: (Arc<str>, off: u32, len: u32) — 24 bytes.
// - Immutable: bytes never mutate after construction.
// - Cheap clone: Arc refcount bump, O(1).
// - O(1) substring: slice shares backing buffer via Arc::clone + offset.
// - Thread-safe: Arc is Send + Sync; bytes are immutable.
//
// Trade-off vs std::string::String:
//   + Clone is O(1) (not O(n)).
//   + Substring is O(1) (not O(n)).
//   + Size: 24 bytes (same as String).
//   + Comparisons work with &str directly (`s == "foo"`).
//   - Not Copy (Rust forbids Copy on Drop types).
//
// The shipping goish `string` alias points here.

#![allow(non_snake_case)]

use std::sync::Arc;

/// Go's `string`. Immutable, cheap clone, O(1) substring.
#[derive(Clone, Eq)]
pub struct GoString {
    buf: Arc<str>,
    off: u32,
    len: u32,
}

// Size check: must be 24 bytes on 64-bit.
const _: () = assert!(std::mem::size_of::<GoString>() == 24);

impl Default for GoString {
    fn default() -> Self {
        GoString { buf: Arc::from(""), off: 0, len: 0 }
    }
}

impl GoString {
    /// Empty string. O(1).
    pub fn new() -> Self { Self::default() }

    /// Go's `len(s)` — returns `int` (i64), not usize.
    pub fn len(&self) -> i64 { self.len as i64 }

    pub fn is_empty(&self) -> bool { self.len == 0 }

    /// Borrow the bytes as a `&str`. Safe — UTF-8 invariant preserved by
    /// construction; slice boundaries at char boundaries (enforced by
    /// Rust's str indexing).
    pub fn as_str(&self) -> &str {
        let start = self.off as usize;
        let end = start + self.len as usize;
        &self.buf[start..end]
    }

    pub fn as_bytes(&self) -> &[u8] {
        let start = self.off as usize;
        let end = start + self.len as usize;
        &self.buf.as_bytes()[start..end]
    }

    /// Go's `s[i:j]` — substring. O(1), shares backing buffer.
    pub fn slice(&self, i: impl Into<i64>, j: impl Into<i64>) -> GoString {
        let i = i.into();
        let j = j.into();
        let lo = i as u32;
        let hi = j as u32;
        if lo > hi || hi > self.len {
            panic!("runtime error: slice bounds out of range [{}:{}] with length {}", i, j, self.len);
        }
        GoString {
            buf: Arc::clone(&self.buf),
            off: self.off + lo,
            len: hi - lo,
        }
    }

    /// Go's `s[i]` — byte at position i.
    pub fn at(&self, i: impl Into<i64>) -> u8 {
        let i = i.into();
        let idx = i as u32;
        if idx >= self.len {
            panic!("runtime error: index out of range [{}] with length {}", i, self.len);
        }
        self.buf.as_bytes()[(self.off + idx) as usize]
    }

    /// Go's `s + t` — concatenate with any stringish value. Allocates.
    pub fn cat(&self, other: impl AsRef<str>) -> GoString {
        let other = other.as_ref();
        let mut buf = std::string::String::with_capacity(self.as_str().len() + other.len());
        buf.push_str(self.as_str());
        buf.push_str(other);
        GoString::from(buf)
    }

    /// Variadic concatenation with a single allocation.
    ///   GoString::concat(&["/", p.as_str(), "/"])
    pub fn concat<S: AsRef<str>>(parts: &[S]) -> GoString {
        let total: usize = parts.iter().map(|s| s.as_ref().len()).sum();
        let mut buf = std::string::String::with_capacity(total);
        for s in parts { buf.push_str(s.as_ref()); }
        GoString::from(buf)
    }

    /// Go's `strings.Index(s, sub)` — first byte index of `sub`, or -1.
    pub fn index(&self, sub: impl AsRef<str>) -> i64 {
        self.as_str().find(sub.as_ref()).map(|i| i as i64).unwrap_or(-1)
    }

    pub fn has_prefix(&self, p: impl AsRef<str>) -> bool {
        self.as_str().starts_with(p.as_ref())
    }

    pub fn has_suffix(&self, s: impl AsRef<str>) -> bool {
        self.as_str().ends_with(s.as_ref())
    }

    // .contains() is inherited from str via Deref — supports &str, char,
    // char-closures, etc. Not overridden here.

    /// Go's Stringer — returns self.
    pub fn String(&self) -> GoString { self.clone() }
}

// ── Conversions in ──────────────────────────────────────────────────

impl From<&str> for GoString {
    fn from(s: &str) -> Self {
        let len = s.len() as u32;
        GoString { buf: Arc::from(s), off: 0, len }
    }
}
impl From<std::string::String> for GoString {
    fn from(s: std::string::String) -> Self {
        let len = s.len() as u32;
        GoString { buf: Arc::from(s), off: 0, len }
    }
}
impl From<&std::string::String> for GoString {
    fn from(s: &std::string::String) -> Self { GoString::from(s.as_str()) }
}
impl From<Arc<str>> for GoString {
    fn from(a: Arc<str>) -> Self {
        let len = a.len() as u32;
        GoString { buf: a, off: 0, len }
    }
}
impl From<char> for GoString {
    fn from(c: char) -> Self { GoString::from(c.to_string()) }
}
impl<'a> From<std::borrow::Cow<'a, str>> for GoString {
    fn from(c: std::borrow::Cow<'a, str>) -> Self { GoString::from(c.into_owned()) }
}

// ── Deref / AsRef / Borrow ──────────────────────────────────────────

impl std::ops::Deref for GoString {
    type Target = str;
    fn deref(&self) -> &str { self.as_str() }
}
impl AsRef<str>  for GoString { fn as_ref(&self) -> &str  { self.as_str() } }
impl AsRef<[u8]> for GoString { fn as_ref(&self) -> &[u8] { self.as_bytes() } }
impl std::borrow::Borrow<str> for GoString {
    fn borrow(&self) -> &str { self.as_str() }
}

// ── Comparison ─────────────────────────────────────────────────────

impl PartialEq for GoString {
    fn eq(&self, other: &GoString) -> bool { self.as_str() == other.as_str() }
}
impl PartialEq<str>    for GoString { fn eq(&self, o: &str)    -> bool { self.as_str() == o } }
impl PartialEq<&str>   for GoString { fn eq(&self, o: &&str)   -> bool { self.as_str() == *o } }
impl PartialEq<std::string::String> for GoString {
    fn eq(&self, o: &std::string::String) -> bool { self.as_str() == o.as_str() }
}
impl PartialEq<GoString> for str {
    fn eq(&self, o: &GoString) -> bool { self == o.as_str() }
}
impl PartialEq<GoString> for &str {
    fn eq(&self, o: &GoString) -> bool { *self == o.as_str() }
}
impl PartialEq<GoString> for std::string::String {
    fn eq(&self, o: &GoString) -> bool { self.as_str() == o.as_str() }
}

impl Ord for GoString {
    fn cmp(&self, other: &GoString) -> std::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}
impl PartialOrd for GoString {
    fn partial_cmp(&self, other: &GoString) -> Option<std::cmp::Ordering> { Some(self.cmp(other)) }
}

impl std::hash::Hash for GoString {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

// ── `p[i]` — Go's `s[i]` byte index ─────────────────────────────────
//
// Indexed by Go's `int` (i64). Returns `u8` by reference — the byte lives
// in our Arc<str> buffer, which is address-stable. Out-of-range panics
// just like Go.

impl std::ops::Index<i64> for GoString {
    type Output = u8;
    fn index(&self, i: i64) -> &u8 {
        if i < 0 || (i as u64) >= self.len as u64 {
            panic!("runtime error: index out of range [{}] with length {}", i, self.len);
        }
        &self.buf.as_bytes()[(self.off + i as u32) as usize]
    }
}

// Range indexing — delegates to str. Without these, adding Index<i64> above
// would block auto-deref resolution of `s[a..b]`, `s[..n]`, etc.
macro_rules! impl_range_index {
    ($($r:ty),+ $(,)?) => { $(
        impl std::ops::Index<$r> for GoString {
            type Output = str;
            fn index(&self, r: $r) -> &str { &self.as_str()[r] }
        }
    )+ };
}
impl_range_index!(
    std::ops::Range<usize>,
    std::ops::RangeTo<usize>,
    std::ops::RangeFrom<usize>,
    std::ops::RangeFull,
    std::ops::RangeInclusive<usize>,
    std::ops::RangeToInclusive<usize>,
);

// ── `+` operator — Go's `a + b` ─────────────────────────────────────

impl std::ops::Add for GoString {
    type Output = GoString;
    fn add(self, rhs: GoString) -> GoString { self.cat(&rhs) }
}
impl std::ops::Add<&GoString> for GoString {
    type Output = GoString;
    fn add(self, rhs: &GoString) -> GoString { self.cat(rhs) }
}
impl std::ops::Add<&str> for GoString {
    type Output = GoString;
    fn add(self, rhs: &str) -> GoString { self.cat(rhs) }
}

// `+=` for Go's `s += "..."` / `s += t`.
impl std::ops::AddAssign<&str> for GoString {
    fn add_assign(&mut self, rhs: &str) { *self = self.cat(rhs); }
}
impl std::ops::AddAssign<GoString> for GoString {
    fn add_assign(&mut self, rhs: GoString) { *self = self.cat(&rhs); }
}
impl std::ops::AddAssign<&GoString> for GoString {
    fn add_assign(&mut self, rhs: &GoString) { *self = self.cat(rhs); }
}

// ── Display / Debug ────────────────────────────────────────────────

impl std::fmt::Display for GoString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}
impl std::fmt::Debug for GoString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

// ── Iteration ──────────────────────────────────────────────────────

impl<'a> IntoIterator for &'a GoString {
    type Item = char;
    type IntoIter = std::str::Chars<'a>;
    fn into_iter(self) -> Self::IntoIter { self.as_str().chars() }
}

// ── Macros ─────────────────────────────────────────────────────────

/// Variadic concatenation. Single allocation.
///
///   cat!(a, b, c)  → `GoString::concat(&[a.as_ref(), b.as_ref(), c.as_ref()])`
#[macro_export]
macro_rules! cat {
    ($($s:expr),+ $(,)?) => {{
        $crate::gostring::GoString::concat(&[$( ($s).as_ref() ),+] as &[&str])
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_matches() {
        assert_eq!(std::mem::size_of::<GoString>(), 24);
    }

    #[test]
    fn basic_construct() {
        let s: GoString = "hello".into();
        assert_eq!(s.len(), 5);
        assert_eq!(s.as_str(), "hello");
        assert_eq!(s, "hello");
    }

    #[test]
    fn slice_is_o1() {
        let s: GoString = "hello world".into();
        let sub = s.slice(6, 11);
        assert_eq!(sub.as_str(), "world");
        assert!(Arc::ptr_eq(&s.buf, &sub.buf)); // shares backing buffer
    }

    #[test]
    fn at_byte() {
        let s: GoString = "hello".into();
        assert_eq!(s.at(0), b'h');
        assert_eq!(s.at(4), b'o');
    }

    #[test]
    #[should_panic]
    fn at_out_of_range_panics() {
        let s: GoString = "abc".into();
        let _ = s.at(5);
    }

    #[test]
    fn cat_binary() {
        let a: GoString = "hello ".into();
        let b: GoString = "world".into();
        assert_eq!(a.cat(&b), "hello world");
    }

    #[test]
    fn concat_variadic() {
        let a: GoString = "a".into();
        let b: GoString = "b".into();
        let c: GoString = "c".into();
        let r = GoString::concat(&[a.as_str(), b.as_str(), c.as_str()]);
        assert_eq!(r, "abc");
    }

    #[test]
    fn compare_with_str_literal() {
        let s: GoString = "x".into();
        assert!(s == "x");
        assert!("x" == s);
        assert!(s != "y");
    }

    #[test]
    fn add_assign_str_literal() {
        let mut p: GoString = "foo".into();
        p += "/";
        p += "bar";
        assert_eq!(p, "foo/bar");
    }

    #[test]
    fn index_byte() {
        let p: GoString = "hello".into();
        assert_eq!(p[0], b'h');
        assert_eq!(p[4], b'o');
        // Iteration with integer index — the Go-shape use case.
        let n = p.len();
        let mut sum: u64 = 0;
        for i in 0..n { sum += p[i] as u64; }
        assert_eq!(sum, (b'h' as u64) + (b'e' as u64) + (b'l' as u64) * 2 + (b'o' as u64));
    }

    #[test]
    #[should_panic]
    fn index_out_of_range_panics() {
        let p: GoString = "ab".into();
        let _ = p[5];
    }

    #[test]
    #[should_panic]
    fn index_negative_panics() {
        let p: GoString = "ab".into();
        let _ = p[-1];
    }

    #[test]
    fn add_assign_gostring() {
        let mut p: GoString = "a".into();
        let b: GoString = "b".into();
        p += &b;
        p += b;
        assert_eq!(p, "abb");
    }

    #[test]
    fn canonical_url_path() {
        fn Canonical(p: GoString) -> GoString {
            if p.is_empty() || p.at(0) != b'/' {
                return GoString::from("/").cat(&p);
            }
            if &*p != "/" && p.has_suffix("/") {
                return p.slice(0, p.len() - 1);
            }
            p
        }
        assert_eq!(Canonical("".into()),   "/");
        assert_eq!(Canonical("/a".into()), "/a");
        assert_eq!(Canonical("a".into()),  "/a");
        assert_eq!(Canonical("/a/".into()),"/a");
        assert_eq!(Canonical("/".into()),  "/");
        assert_eq!(Canonical("//".into()), "/");
    }
}
