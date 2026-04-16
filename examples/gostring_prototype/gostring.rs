// GoString v2 — Arc<str> wrapper with Go-shape ergonomics.
//
// Design goals:
//   - Memory-safe (no leaks, refcounted)
//   - Cheap clone (atomic refcount bump, O(1))
//   - Go-like call sites via methods: .cat(), .at(), .slice(), .index()
//   - Rich comparison (== works with &str, String, GoString)
//   - Deref<Target=str> for passthrough to any str method
//
// Trade-off vs the Copy-leaked v1:
//   - NOT Copy — still need .clone() (but it's O(1))
//   - In exchange: actual memory reclamation, no leak
//
// The ergonomics aim to answer #120's translation pains:
//   Go               GoString
//   ───────────────  ────────────────────────
//   a + b + c        a.cat(&b).cat(&c)     or cat!(a, b, c)
//   s[0]             s.at(0)               (takes Go int, not usize)
//   s[i:j]           s.slice(i, j)         (takes int, not usize)
//   len(s)           s.len()               (returns Go int)
//   s == "foo"       s == "foo"            (PartialEq<&str>)
//   strings.Index    s.index("sub")

#![allow(dead_code)]
#![allow(non_snake_case)]

use std::sync::Arc;

/// Go's string, implemented as Arc<str>. Cheap clone, immutable, thread-safe.
#[derive(Clone, Default, Eq)]
pub struct GoString(Arc<str>);

// Size check: Arc<str> is 16 bytes on 64-bit (ptr + len). Matches Go.
const _SIZE_CHECK: () = assert!(std::mem::size_of::<GoString>() == 16);

/// Variant 2 — 24 bytes with (buf, off, len) for O(1) substring.
/// Trades 8 bytes for the ability to sub-slice without re-allocating.
#[derive(Clone, Eq)]
pub struct GoString24 {
    buf: Arc<str>,
    off: u32,
    len: u32,
}

const _SIZE_CHECK_24: () = assert!(std::mem::size_of::<GoString24>() == 24);

impl Default for GoString24 {
    fn default() -> Self {
        GoString24 { buf: Arc::from(""), off: 0, len: 0 }
    }
}

impl GoString24 {
    pub fn new() -> Self { Self::default() }

    pub fn len(&self) -> i64 { self.len as i64 }
    pub fn is_empty(&self) -> bool { self.len == 0 }

    pub fn as_str(&self) -> &str {
        let start = self.off as usize;
        let end = start + self.len as usize;
        &self.buf[start..end]
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.buf.as_bytes()[self.off as usize..(self.off + self.len) as usize]
    }

    /// Go's `s[i:j]` — substring. O(1), shares backing buffer.
    /// Accepts `int`-like indices so callers don't cast.
    pub fn slice(&self, i: impl Into<i64>, j: impl Into<i64>) -> GoString24 {
        let i = i.into();
        let j = j.into();
        let lo = i as u32;
        let hi = j as u32;
        if lo > hi || hi > self.len {
            panic!("runtime error: slice bounds out of range [{}:{}] with length {}", i, j, self.len);
        }
        GoString24 {
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

    /// Go's `s + t` — concatenate with any stringish thing.
    pub fn cat(&self, other: impl AsRef<str>) -> GoString24 {
        let other = other.as_ref();
        let mut buf = String::with_capacity(self.as_str().len() + other.len());
        buf.push_str(self.as_str());
        buf.push_str(other);
        GoString24::from(buf)
    }

    /// Variadic concatenation. Pre-sizes the buffer for one allocation.
    ///   GoString::concat(&["/", p.as_str(), "/"])
    /// Returns a new GoString24.
    pub fn concat<S: AsRef<str>>(parts: &[S]) -> GoString24 {
        let total: usize = parts.iter().map(|s| s.as_ref().len()).sum();
        let mut buf = String::with_capacity(total);
        for s in parts { buf.push_str(s.as_ref()); }
        GoString24::from(buf)
    }

    /// Go's `strings.Index(s, sub)` — first byte index of sub, or -1.
    pub fn index(&self, sub: impl AsRef<str>) -> i64 {
        self.as_str().find(sub.as_ref()).map(|i| i as i64).unwrap_or(-1)
    }

    pub fn has_prefix(&self, p: impl AsRef<str>) -> bool {
        self.as_str().starts_with(p.as_ref())
    }

    pub fn has_suffix(&self, s: impl AsRef<str>) -> bool {
        self.as_str().ends_with(s.as_ref())
    }

    pub fn contains(&self, sub: impl AsRef<str>) -> bool {
        self.as_str().contains(sub.as_ref())
    }
}

impl From<&str> for GoString24 {
    fn from(s: &str) -> Self {
        GoString24 { buf: Arc::from(s), off: 0, len: s.len() as u32 }
    }
}
impl From<String> for GoString24 {
    fn from(s: String) -> Self {
        let len = s.len() as u32;
        GoString24 { buf: Arc::from(s), off: 0, len }
    }
}

impl PartialEq for GoString24 {
    fn eq(&self, other: &GoString24) -> bool { self.as_str() == other.as_str() }
}
impl PartialEq<&str> for GoString24 { fn eq(&self, o: &&str) -> bool { self.as_str() == *o } }
impl PartialEq<str>  for GoString24 { fn eq(&self, o: &str)  -> bool { self.as_str() == o } }

impl std::ops::Deref for GoString24 {
    type Target = str;
    fn deref(&self) -> &str { self.as_str() }
}

impl AsRef<str> for GoString24 {
    fn as_ref(&self) -> &str { self.as_str() }
}

impl std::fmt::Display for GoString24 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { self.as_str().fmt(f) }
}
impl std::fmt::Debug for GoString24 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { self.as_str().fmt(f) }
}

impl GoString {
    /// Empty string. O(1), no allocation (uses a shared empty Arc).
    pub fn new() -> Self {
        GoString(Arc::from(""))
    }

    /// Go's `len(s)` — returns Go int (i64), not usize.
    pub fn len(&self) -> i64 {
        self.0.len() as i64
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Go's `s[i]` — byte at position i. Takes Go int.
    /// Panics on out-of-bounds (matches Go's runtime panic).
    pub fn at(&self, i: i64) -> u8 {
        let idx = i as usize;
        if idx >= self.0.len() {
            panic!("runtime error: index out of range [{}] with length {}", i, self.0.len());
        }
        self.0.as_bytes()[idx]
    }

    /// Go's `s[i:j]` — substring. Takes Go ints.
    /// Allocates a new Arc<str> (shares backing bytes via clone-from-slice).
    pub fn slice(&self, i: i64, j: i64) -> GoString {
        let lo = i as usize;
        let hi = j as usize;
        if lo > hi || hi > self.0.len() {
            panic!("runtime error: slice bounds out of range [{}:{}] with length {}", i, j, self.0.len());
        }
        GoString(Arc::from(&self.0[lo..hi]))
    }

    /// Go's `s + t` — concatenation. Allocates.
    pub fn cat(&self, other: &GoString) -> GoString {
        let mut buf = String::with_capacity(self.0.len() + other.0.len());
        buf.push_str(&self.0);
        buf.push_str(&other.0);
        GoString(Arc::from(buf))
    }

    /// Go's `strings.Index(s, sub)` — first byte-index of sub, -1 if absent.
    pub fn index(&self, sub: impl AsRef<str>) -> i64 {
        match self.0.find(sub.as_ref()) {
            Some(i) => i as i64,
            None => -1,
        }
    }

    pub fn has_prefix(&self, p: impl AsRef<str>) -> bool {
        self.0.starts_with(p.as_ref())
    }

    pub fn has_suffix(&self, s: impl AsRef<str>) -> bool {
        self.0.ends_with(s.as_ref())
    }

    pub fn contains(&self, sub: impl AsRef<str>) -> bool {
        self.0.contains(sub.as_ref())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

// Deref → &str so any str method works.
impl std::ops::Deref for GoString {
    type Target = str;
    fn deref(&self) -> &str { &self.0 }
}

impl AsRef<str> for GoString {
    fn as_ref(&self) -> &str { &self.0 }
}

// Conversions in.
impl From<&str>    for GoString { fn from(s: &str)    -> Self { GoString(Arc::from(s)) } }
impl From<String>  for GoString { fn from(s: String)  -> Self { GoString(Arc::from(s)) } }
impl From<&String> for GoString { fn from(s: &String) -> Self { GoString(Arc::from(s.as_str())) } }
impl From<Arc<str>> for GoString { fn from(a: Arc<str>) -> Self { GoString(a) } }

// Comparisons — every shape that could plausibly appear in Go port.
impl PartialEq for GoString {
    fn eq(&self, other: &GoString) -> bool { self.0 == other.0 }
}
impl PartialEq<str>     for GoString { fn eq(&self, o: &str)    -> bool { &*self.0 == o } }
impl PartialEq<&str>    for GoString { fn eq(&self, o: &&str)   -> bool { &*self.0 == *o } }
impl PartialEq<String>  for GoString { fn eq(&self, o: &String) -> bool { &*self.0 == o.as_str() } }
impl PartialEq<GoString> for str     { fn eq(&self, o: &GoString) -> bool { self == &*o.0 } }
impl PartialEq<GoString> for &str    { fn eq(&self, o: &GoString) -> bool { *self == &*o.0 } }
impl PartialEq<GoString> for String  { fn eq(&self, o: &GoString) -> bool { self.as_str() == &*o.0 } }

impl std::hash::Hash for GoString {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl Ord for GoString {
    fn cmp(&self, other: &GoString) -> std::cmp::Ordering { self.0.cmp(&other.0) }
}
impl PartialOrd for GoString {
    fn partial_cmp(&self, other: &GoString) -> Option<std::cmp::Ordering> { Some(self.cmp(other)) }
}

// Add operator → concatenation, matching Go's `a + b`.
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
    fn add(self, rhs: &str) -> GoString { self.cat(&GoString::from(rhs)) }
}

// Display / Debug.
impl std::fmt::Display for GoString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl std::fmt::Debug for GoString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

// ─── Translation-feel demo (method-only, no macros) ─────────────────

fn CanonicalURLPath(p: GoString24) -> GoString24 {
    // Go:
    //   if p == "" || p[0] != '/' { return "/" + p }
    //   if p != "/" && strings.HasSuffix(p, "/") { return p[:len(p)-1] }
    //   return p
    if p.is_empty() || p.at(0) != b'/' {
        return GoString24::from("/").cat(&p);
    }
    if &*p != "/" && p.has_suffix("/") {
        return p.slice(0, p.len() - 1);
    }
    p
}

fn demo_translation() {
    println!("─── Go translation fidelity demo (CanonicalURLPath, methods only) ─");

    struct Case { p: GoString24, want: GoString24 }
    let cases = vec![
        Case { p: "".into(),   want: "/".into()  },
        Case { p: "/a".into(), want: "/a".into() },
        Case { p: "a".into(),  want: "/a".into() },
        Case { p: "/a/".into(),want: "/a".into() },
        Case { p: "/".into(),  want: "/".into()  },
        Case { p: "//".into(), want: "/".into()  },
    ];

    for (i, tt) in cases.iter().enumerate() {
        let got = CanonicalURLPath(tt.p.clone());
        let ok = if got == tt.want { "ok " } else { "FAIL" };
        println!("  [{}] {}  p={:<6} want={:<5} got={}", ok, i, tt.p, tt.want, got);
    }
}

// ─── Benchmark ──────────────────────────────────────────────────────

use std::time::Instant;

fn bench<F: FnMut()>(name: &str, iters: usize, mut f: F) {
    let start = Instant::now();
    for _ in 0..iters { f(); }
    let d = start.elapsed();
    println!("{:40} {:>10} iters in {:>8.2} ms ({:>6.0} ns/iter)",
        name, iters, d.as_secs_f64() * 1000.0,
        d.as_nanos() as f64 / iters as f64);
}

fn main() {
    const TABLE_SIZE: usize = 100;
    const ITERS: usize = 10_000;

    let gs_table: Vec<GoString> = (0..TABLE_SIZE)
        .map(|i| GoString::from(format!("/path/to/item/number/{}", i)))
        .collect();
    let gs24_table: Vec<GoString24> = (0..TABLE_SIZE)
        .map(|i| GoString24::from(format!("/path/to/item/number/{}", i)))
        .collect();
    let str_table: Vec<String> = (0..TABLE_SIZE)
        .map(|i| format!("/path/to/item/number/{}", i))
        .collect();

    println!("─── Three designs compared ─────────────────────────────────────");
    println!("size_of::<String>()    = {} bytes (ptr + len + cap)", std::mem::size_of::<String>());
    println!("size_of::<GoString>()  = {} bytes (Arc<str>)", std::mem::size_of::<GoString>());
    println!("size_of::<GoString24>()= {} bytes (Arc<str> + off + len)", std::mem::size_of::<GoString24>());

    println!("\n─── clone — 100 strings × 10k iters ────────────────────────────");

    bench("String.clone() — heap alloc+copy", ITERS, || {
        for s in &str_table { std::hint::black_box(s.clone()); }
    });
    bench("GoString.clone()   — Arc bump", ITERS, || {
        for s in &gs_table { std::hint::black_box(s.clone()); }
    });
    bench("GoString24.clone() — Arc bump", ITERS, || {
        for s in &gs24_table { std::hint::black_box(s.clone()); }
    });

    println!("\n─── substring: s[3..20] ────────────────────────────────────────");

    bench("String[3..20].to_string() — alloc+copy", ITERS, || {
        for s in &str_table { std::hint::black_box(s[3..20].to_string()); }
    });
    bench("GoString.slice(3, 20)     — alloc new Arc", ITERS, || {
        for s in &gs_table { std::hint::black_box(s.slice(3, 20)); }
    });
    bench("GoString24.slice(3, 20)   — O(1) Arc bump", ITERS, || {
        for s in &gs24_table { std::hint::black_box(s.slice(3, 20)); }
    });

    // Long-string substring — where O(1) matters
    let long_str: String = "x".repeat(4096);
    let long_gs: GoString = GoString::from(long_str.clone());
    let long_gs24: GoString24 = GoString24::from(long_str.clone());

    println!("\n─── substring of 4096-byte string (s[100..200]) ────────────────");

    bench("String[100..200].to_string()        ", ITERS, || {
        std::hint::black_box(long_str[100..200].to_string());
    });
    bench("GoString.slice(100, 200)   — alloc  ", ITERS, || {
        std::hint::black_box(long_gs.slice(100, 200));
    });
    bench("GoString24.slice(100, 200) — O(1)   ", ITERS, || {
        std::hint::black_box(long_gs24.slice(100, 200));
    });

    println!("\n─── concat ─────────────────────────────────────────────────────");

    let a: GoString24 = "prefix-".into();
    let b: GoString24 = "middle-".into();
    let c: GoString24 = "suffix".into();

    bench("GoString24::concat(&[...]) — one alloc", ITERS, || {
        let r = GoString24::concat(&[a.as_str(), b.as_str(), c.as_str()]);
        std::hint::black_box(&r);
    });
    bench("a.cat(&b).cat(&c)           — two allocs", ITERS, || {
        let r = a.cat(&b).cat(&c);
        std::hint::black_box(&r);
    });
    bench("format!(\"{}{}{}\", a, b, c) — one alloc", ITERS, || {
        let r = format!("{}{}{}", a, b, c);
        std::hint::black_box(&r);
    });

    println!("\n─── byte access ────────────────────────────────────────────────");

    bench("gs.at(5)           (GoString24 method)", ITERS * 10, || {
        for s in &gs24_table { std::hint::black_box(s.at(5)); }
    });
    bench("s.as_bytes()[5]    (String manual)", ITERS * 10, || {
        for s in &str_table { std::hint::black_box(s.as_bytes()[5]); }
    });

    println!("\n─── equality — s == \"literal\" ──────────────────────────────────");

    bench("GoString24 == &str", ITERS * 10, || {
        for s in &gs24_table {
            std::hint::black_box(s == "/path/to/item/number/42");
        }
    });
    bench("String == &str", ITERS * 10, || {
        for s in &str_table {
            std::hint::black_box(s == "/path/to/item/number/42");
        }
    });

    println!();
    demo_translation();
}
