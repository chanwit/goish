// strings: Go's strings package, ported.
//
//   Go                                goish
//   ───────────────────────────────   ──────────────────────────────────
//   strings.Contains(s, "x")          strings::Contains(s, "x")
//   strings.HasPrefix(s, "p")         strings::HasPrefix(s, "p")
//   strings.HasSuffix(s, "p")         strings::HasSuffix(s, "p")
//   strings.Index(s, "x")             strings::Index(s, "x")        // -1 if absent
//   strings.LastIndex(s, "x")         strings::LastIndex(s, "x")
//   strings.Count(s, "a")             strings::Count(s, "a")
//   strings.Split(s, ",")             strings::Split(s, ",")        // → slice<string>
//   strings.SplitN(s, ",", n)         strings::SplitN(s, ",", n)
//   strings.Join(elems, ",")          strings::Join(&elems, ",")
//   strings.Replace(s, a, b, n)       strings::Replace(s, a, b, n)  // n<0 = all
//   strings.ReplaceAll(s, a, b)       strings::ReplaceAll(s, a, b)
//   strings.ToUpper(s)                strings::ToUpper(s)
//   strings.ToLower(s)                strings::ToLower(s)
//   strings.TrimSpace(s)              strings::TrimSpace(s)
//   strings.Trim(s, "x")              strings::Trim(s, "x")
//   strings.TrimPrefix(s, "p")        strings::TrimPrefix(s, "p")
//   strings.TrimSuffix(s, "p")        strings::TrimSuffix(s, "p")
//   strings.Fields(s)                 strings::Fields(s)
//   strings.Repeat(s, n)              strings::Repeat(s, n)
//   strings.EqualFold(s, t)           strings::EqualFold(s, t)
//
// All functions take `impl AsRef<str>` so users can pass `String`, `&String`,
// or `&str` without spelling out the conversion.

use crate::types::{int, slice, string};

pub fn Contains(s: impl AsRef<str>, substr: impl AsRef<str>) -> bool {
    s.as_ref().contains(substr.as_ref())
}

/// strings.Compare(a, b) — returns -1 / 0 / 1 per lexicographic order.
pub fn Compare(a: impl AsRef<str>, b: impl AsRef<str>) -> int {
    use std::cmp::Ordering::*;
    match a.as_ref().cmp(b.as_ref()) {
        Less => -1,
        Equal => 0,
        Greater => 1,
    }
}

/// strings.Clone(s) — returns a fresh copy of s. In Go this disentangles
/// a string's underlying storage; in goish `String` is owned so it's a
/// plain clone.
pub fn Clone(s: impl AsRef<str>) -> string {
    s.as_ref().to_string()
}

pub fn HasPrefix(s: impl AsRef<str>, prefix: impl AsRef<str>) -> bool {
    s.as_ref().starts_with(prefix.as_ref())
}

pub fn HasSuffix(s: impl AsRef<str>, suffix: impl AsRef<str>) -> bool {
    s.as_ref().ends_with(suffix.as_ref())
}

/// strings.Index — byte index of first occurrence, or -1.
pub fn Index(s: impl AsRef<str>, substr: impl AsRef<str>) -> int {
    match s.as_ref().find(substr.as_ref()) {
        Some(i) => i as int,
        None => -1,
    }
}

pub fn LastIndex(s: impl AsRef<str>, substr: impl AsRef<str>) -> int {
    match s.as_ref().rfind(substr.as_ref()) {
        Some(i) => i as int,
        None => -1,
    }
}

pub fn Count(s: impl AsRef<str>, substr: impl AsRef<str>) -> int {
    let s = s.as_ref();
    let substr = substr.as_ref();
    if substr.is_empty() {
        return (s.chars().count() + 1) as int;
    }
    s.matches(substr).count() as int
}

pub fn Split(s: impl AsRef<str>, sep: impl AsRef<str>) -> slice<string> {
    let s = s.as_ref();
    let sep = sep.as_ref();
    if sep.is_empty() {
        return s.chars().map(|c| c.to_string()).collect();
    }
    s.split(sep).map(String::from).collect()
}

/// strings.SplitN — like Split but stops after n substrings (n<0 = all, n==0 = empty).
pub fn SplitN(s: impl AsRef<str>, sep: impl AsRef<str>, n: int) -> slice<string> {
    if n == 0 {
        return slice::new();
    }
    let s = s.as_ref();
    let sep = sep.as_ref();
    if n < 0 {
        return Split(s, sep);
    }
    s.splitn(n as usize, sep).map(String::from).collect()
}

pub fn Join(elems: &[string], sep: impl AsRef<str>) -> string {
    elems.join(sep.as_ref())
}

/// strings.Replace — replace first n occurrences (n<0 = all).
pub fn Replace(s: impl AsRef<str>, old: impl AsRef<str>, new: impl AsRef<str>, n: int) -> string {
    let s = s.as_ref();
    let old = old.as_ref();
    let new = new.as_ref();
    if n < 0 {
        s.replace(old, new)
    } else {
        s.replacen(old, new, n as usize)
    }
}

pub fn ReplaceAll(s: impl AsRef<str>, old: impl AsRef<str>, new: impl AsRef<str>) -> string {
    s.as_ref().replace(old.as_ref(), new.as_ref())
}

pub fn ToUpper(s: impl AsRef<str>) -> string {
    s.as_ref().to_uppercase()
}

pub fn ToLower(s: impl AsRef<str>) -> string {
    s.as_ref().to_lowercase()
}

pub fn TrimSpace(s: impl AsRef<str>) -> string {
    s.as_ref().trim().to_string()
}

pub fn TrimPrefix(s: impl AsRef<str>, prefix: impl AsRef<str>) -> string {
    let s = s.as_ref();
    s.strip_prefix(prefix.as_ref()).unwrap_or(s).to_string()
}

pub fn TrimSuffix(s: impl AsRef<str>, suffix: impl AsRef<str>) -> string {
    let s = s.as_ref();
    s.strip_suffix(suffix.as_ref()).unwrap_or(s).to_string()
}

pub fn Trim(s: impl AsRef<str>, cutset: impl AsRef<str>) -> string {
    let cutset = cutset.as_ref().to_string();
    s.as_ref().trim_matches(|c: char| cutset.contains(c)).to_string()
}

pub fn Fields(s: impl AsRef<str>) -> slice<string> {
    s.as_ref().split_whitespace().map(String::from).collect()
}

pub fn Repeat(s: impl AsRef<str>, count: int) -> string {
    if count < 0 {
        panic!("strings: negative Repeat count");
    }
    s.as_ref().repeat(count as usize)
}

/// ASCII-only fold (Go does full Unicode; close enough for now).
/// `strings.EqualFold(s, t)` — Unicode case-insensitive equality. Uses
/// Rust's `char::to_lowercase` for folding so Greek/Latin/Cyrillic etc
/// compare correctly (not just ASCII).
pub fn EqualFold(s: impl AsRef<str>, t: impl AsRef<str>) -> bool {
    let mut si = s.as_ref().chars();
    let mut ti = t.as_ref().chars();
    loop {
        let a = si.next();
        let b = ti.next();
        match (a, b) {
            (None, None) => return true,
            (None, _) | (_, None) => return false,
            (Some(a), Some(b)) => {
                if a == b { continue; }
                let a_low: Vec<char> = a.to_lowercase().collect();
                let b_low: Vec<char> = b.to_lowercase().collect();
                if a_low != b_low { return false; }
            }
        }
    }
}

// ── strings.Builder ────────────────────────────────────────────────────
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   var b strings.Builder               let mut b = strings::Builder::new();
//   b.WriteString("hello ")             b.WriteString("hello ");
//   b.WriteByte('!')                    b.WriteByte(b'!');
//   b.WriteRune('λ')                    b.WriteRune('λ');
//   s := b.String()                     let s = b.String();
//   n := b.Len()                        let n = b.Len();
//   b.Reset()                           b.Reset();

#[derive(Debug, Clone, Default)]
pub struct Builder {
    inner: string,
}

impl Builder {
    pub fn new() -> Self { Builder::default() }

    pub fn WriteString(&mut self, s: impl AsRef<str>) -> (int, crate::errors::error) {
        let s = s.as_ref();
        self.inner.push_str(s);
        (s.len() as int, crate::errors::nil)
    }

    pub fn WriteByte(&mut self, b: crate::types::byte) -> crate::errors::error {
        // Go's WriteByte takes a byte; we accept only ASCII-valid bytes since
        // Builder backs to a String (UTF-8). Non-ASCII bytes panic (matches
        // Go's runtime behavior on invalid UTF-8 conversion).
        if b < 0x80 {
            self.inner.push(b as char);
            crate::errors::nil
        } else {
            crate::errors::New("strings.Builder: non-ASCII byte; use WriteRune")
        }
    }

    pub fn WriteRune(&mut self, r: char) -> (int, crate::errors::error) {
        let n = r.len_utf8();
        self.inner.push(r);
        (n as int, crate::errors::nil)
    }

    pub fn String(&self) -> string {
        self.inner.clone()
    }

    /// `b.Cap()` — underlying capacity of the backing buffer. Used by
    /// Go's tests; here we forward to String::capacity.
    #[allow(non_snake_case)]
    pub fn Cap(&self) -> int { self.inner.capacity() as int }

    /// `b.Write(p)` — append raw bytes (must be valid UTF-8 for goish;
    /// Go's Builder accepts arbitrary bytes since []byte ⊂ string there).
    #[allow(non_snake_case)]
    pub fn Write(&mut self, p: &[crate::types::byte]) -> (int, crate::errors::error) {
        match std::str::from_utf8(p) {
            Ok(s) => { self.inner.push_str(s); (p.len() as int, crate::errors::nil) },
            Err(_) => (0, crate::errors::New("strings.Builder.Write: invalid UTF-8")),
        }
    }

    pub fn Len(&self) -> int {
        self.inner.len() as int
    }

    pub fn Reset(&mut self) {
        self.inner.clear();
    }

    pub fn Grow(&mut self, n: int) {
        if n > 0 {
            self.inner.reserve(n as usize);
        }
    }

    /// Lowercase alias for the polymorphic `len!()` macro.
    pub fn len(&self) -> usize {
        self.inner.len()
    }
}

impl std::fmt::Display for Builder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

// ── strings.Reader ─────────────────────────────────────────────────────
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   r := strings.NewReader("hello")     let r = strings::NewReader("hello");
//   n, err := r.Read(p)                 let (n, err) = r.Read(&mut p);

#[derive(Debug, Clone)]
pub struct Reader {
    data: String,
    pos: usize,
}

impl Reader {
    pub fn Len(&self) -> int {
        (self.data.len().saturating_sub(self.pos)) as int
    }

    pub fn Size(&self) -> crate::types::int64 {
        self.data.len() as crate::types::int64
    }

    pub fn Read(&mut self, p: &mut [crate::types::byte]) -> (int, crate::errors::error) {
        if self.pos >= self.data.len() {
            return (0, crate::io::EOF());
        }
        let bytes = self.data.as_bytes();
        let n = (bytes.len() - self.pos).min(p.len());
        p[..n].copy_from_slice(&bytes[self.pos..self.pos + n]);
        self.pos += n;
        (n as int, crate::errors::nil)
    }

    pub fn ReadByte(&mut self) -> (crate::types::byte, crate::errors::error) {
        let bytes = self.data.as_bytes();
        if self.pos >= bytes.len() {
            return (0, crate::io::EOF());
        }
        let b = bytes[self.pos];
        self.pos += 1;
        (b, crate::errors::nil)
    }

    pub fn UnreadByte(&mut self) -> crate::errors::error {
        if self.pos == 0 {
            return crate::errors::New("strings.Reader.UnreadByte: at beginning of string");
        }
        self.pos -= 1;
        crate::errors::nil
    }

    /// `r.ReadAt(p, off)` — read at an absolute offset without advancing
    /// the internal cursor. Matches Go's io.ReaderAt interface.
    #[allow(non_snake_case)]
    pub fn ReadAt(&self, p: &mut [crate::types::byte], off: crate::types::int64) -> (int, crate::errors::error) {
        if off < 0 {
            return (0, crate::errors::New("strings.Reader.ReadAt: negative offset"));
        }
        let off = off as usize;
        if off >= self.data.len() {
            return (0, crate::io::EOF());
        }
        let bytes = self.data.as_bytes();
        let available = bytes.len() - off;
        let n = available.min(p.len());
        p[..n].copy_from_slice(&bytes[off..off + n]);
        if n < p.len() {
            (n as int, crate::io::EOF())
        } else {
            (n as int, crate::errors::nil)
        }
    }

    pub fn Seek(&mut self, offset: crate::types::int64, whence: int) -> (crate::types::int64, crate::errors::error) {
        let new_pos: i64 = match whence {
            0 => offset,
            1 => self.pos as i64 + offset,
            2 => self.data.len() as i64 + offset,
            _ => return (0, crate::errors::New("strings.Reader.Seek: invalid whence")),
        };
        if new_pos < 0 {
            return (0, crate::errors::New("strings.Reader.Seek: negative position"));
        }
        self.pos = new_pos as usize;
        (new_pos, crate::errors::nil)
    }
}

impl std::io::Read for Reader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let bytes = self.data.as_bytes();
        if self.pos >= bytes.len() {
            return Ok(0);
        }
        let n = (bytes.len() - self.pos).min(buf.len());
        buf[..n].copy_from_slice(&bytes[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }
}

/// strings.NewReader(s) — construct a Reader over the string.
#[allow(non_snake_case)]
pub fn NewReader(s: impl Into<String>) -> Reader {
    Reader { data: s.into(), pos: 0 }
}

// ── strings.Replacer ───────────────────────────────────────────────────
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   r := strings.NewReplacer("a","1", "b","2")
//                                       let r = strings::NewReplacer(&["a","1", "b","2"]);
//   r.Replace(s)                        r.Replace(s)

#[derive(Debug, Clone)]
pub struct Replacer {
    pairs: Vec<(String, String)>,
}

impl Replacer {
    pub fn Replace(&self, s: impl AsRef<str>) -> string {
        let s = s.as_ref();
        let mut out = String::with_capacity(s.len());
        let bytes = s.as_bytes();
        let mut i = 0usize;
        'outer: while i < bytes.len() {
            for (old, new) in &self.pairs {
                if old.is_empty() { continue; }
                let ob = old.as_bytes();
                if i + ob.len() <= bytes.len() && &bytes[i..i + ob.len()] == ob {
                    out.push_str(new);
                    i += ob.len();
                    continue 'outer;
                }
            }
            // No pair matched; copy one UTF-8 char.
            let ch = s[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
        out
    }

    pub fn WriteString<W: std::io::Write>(&self, w: &mut W, s: impl AsRef<str>) -> (int, crate::errors::error) {
        let result = self.Replace(s);
        match w.write(result.as_bytes()) {
            Ok(n) => (n as int, crate::errors::nil),
            Err(e) => (0, crate::errors::New(&e.to_string())),
        }
    }
}

/// strings.NewReplacer("old1","new1","old2","new2",...)
///
/// Takes a slice of alternating old/new strings. Panics on odd count.
#[allow(non_snake_case)]
pub fn NewReplacer(pairs: &[impl AsRef<str>]) -> Replacer {
    if pairs.len() % 2 != 0 {
        panic!("strings.NewReplacer: odd argument count");
    }
    let pairs = pairs.chunks(2)
        .map(|c| (c[0].as_ref().to_string(), c[1].as_ref().to_string()))
        .collect();
    Replacer { pairs }
}

// ── strings.Map ────────────────────────────────────────────────────────
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   strings.Map(fn, s)                  strings::Map(|r| …, s)
//
// fn returns a char; if it returns '\0' the rune is dropped (Go uses
// negative rune; we use '\0' since char cannot be negative).

#[allow(non_snake_case)]
pub fn Map(mut f: impl FnMut(char) -> char, s: impl AsRef<str>) -> string {
    let mut out = String::with_capacity(s.as_ref().len());
    for c in s.as_ref().chars() {
        let r = f(c);
        if r != '\0' {
            out.push(r);
        }
    }
    out
}

// ── strings.ContainsAny / IndexAny / ContainsRune ──────────────────────

#[allow(non_snake_case)]
pub fn ContainsAny(s: impl AsRef<str>, chars: impl AsRef<str>) -> bool {
    let set: Vec<char> = chars.as_ref().chars().collect();
    s.as_ref().chars().any(|c| set.contains(&c))
}

#[allow(non_snake_case)]
pub fn ContainsRune(s: impl AsRef<str>, r: char) -> bool {
    s.as_ref().contains(r)
}

#[allow(non_snake_case)]
pub fn IndexAny(s: impl AsRef<str>, chars: impl AsRef<str>) -> int {
    let s = s.as_ref();
    let set: Vec<char> = chars.as_ref().chars().collect();
    for (i, c) in s.char_indices() {
        if set.contains(&c) {
            return i as int;
        }
    }
    -1
}

#[allow(non_snake_case)]
pub fn IndexByte(s: impl AsRef<str>, b: crate::types::byte) -> int {
    s.as_ref().as_bytes().iter().position(|x| *x == b).map(|i| i as int).unwrap_or(-1)
}

#[allow(non_snake_case)]
pub fn IndexRune(s: impl AsRef<str>, r: char) -> int {
    let s = s.as_ref();
    s.find(r).map(|i| i as int).unwrap_or(-1)
}

/// `strings.Cut(s, sep)` — slices s around the first instance of sep.
/// Returns (before, after, found). If sep not in s, returns (s, "", false).
#[allow(non_snake_case)]
pub fn Cut(s: impl AsRef<str>, sep: impl AsRef<str>) -> (string, string, bool) {
    let s = s.as_ref(); let sep = sep.as_ref();
    match s.find(sep) {
        Some(i) => (s[..i].to_string(), s[i + sep.len()..].to_string(), true),
        None => (s.to_string(), String::new(), false),
    }
}

/// `strings.CutPrefix(s, prefix)` — if prefix matches, returns (after, true); else (s, false).
#[allow(non_snake_case)]
pub fn CutPrefix(s: impl AsRef<str>, prefix: impl AsRef<str>) -> (string, bool) {
    let s = s.as_ref(); let p = prefix.as_ref();
    match s.strip_prefix(p) {
        Some(rest) => (rest.to_string(), true),
        None => (s.to_string(), false),
    }
}

/// `strings.CutSuffix(s, suffix)` — if suffix matches, returns (before, true); else (s, false).
#[allow(non_snake_case)]
pub fn CutSuffix(s: impl AsRef<str>, suffix: impl AsRef<str>) -> (string, bool) {
    let s = s.as_ref(); let suf = suffix.as_ref();
    match s.strip_suffix(suf) {
        Some(rest) => (rest.to_string(), true),
        None => (s.to_string(), false),
    }
}

/// `strings.TrimLeft(s, cutset)` — drop leading runes in `cutset`.
#[allow(non_snake_case)]
pub fn TrimLeft(s: impl AsRef<str>, cutset: impl AsRef<str>) -> string {
    let cut: Vec<char> = cutset.as_ref().chars().collect();
    s.as_ref().trim_start_matches(|c: char| cut.contains(&c)).to_string()
}

/// `strings.TrimRight(s, cutset)` — drop trailing runes in `cutset`.
#[allow(non_snake_case)]
pub fn TrimRight(s: impl AsRef<str>, cutset: impl AsRef<str>) -> string {
    let cut: Vec<char> = cutset.as_ref().chars().collect();
    s.as_ref().trim_end_matches(|c: char| cut.contains(&c)).to_string()
}

/// `strings.LastIndexByte(s, c)` — last index of the byte `c` in `s`, or -1.
#[allow(non_snake_case)]
pub fn LastIndexByte(s: impl AsRef<str>, c: crate::types::byte) -> int {
    s.as_ref().as_bytes().iter().rposition(|&x| x == c).map(|i| i as int).unwrap_or(-1)
}

/// `strings.LastIndexAny(s, chars)` — greatest index of a rune in `chars`
/// appearing in `s`, or -1.
#[allow(non_snake_case)]
pub fn LastIndexAny(s: impl AsRef<str>, chars: impl AsRef<str>) -> int {
    let s = s.as_ref(); let chars = chars.as_ref();
    let set: Vec<char> = chars.chars().collect();
    let mut best: i64 = -1;
    for (i, c) in s.char_indices() {
        if set.contains(&c) { best = i as i64; }
    }
    best
}

/// `strings.IndexFunc(s, f)` — first index where f(rune) is true, or -1.
#[allow(non_snake_case)]
pub fn IndexFunc(s: impl AsRef<str>, mut f: impl FnMut(char) -> bool) -> int {
    for (i, c) in s.as_ref().char_indices() {
        if f(c) { return i as int; }
    }
    -1
}

/// `strings.LastIndexFunc(s, f)` — last index where f(rune) is true, or -1.
#[allow(non_snake_case)]
pub fn LastIndexFunc(s: impl AsRef<str>, mut f: impl FnMut(char) -> bool) -> int {
    let mut best: i64 = -1;
    for (i, c) in s.as_ref().char_indices() {
        if f(c) { best = i as i64; }
    }
    best
}

/// `strings.TrimFunc(s, f)` — trim runes satisfying f from both ends.
#[allow(non_snake_case)]
pub fn TrimFunc(s: impl AsRef<str>, mut f: impl FnMut(char) -> bool) -> string {
    s.as_ref().trim_matches(|c: char| f(c)).to_string()
}

/// `strings.TrimLeftFunc(s, f)` — trim runes satisfying f from the start.
#[allow(non_snake_case)]
pub fn TrimLeftFunc(s: impl AsRef<str>, mut f: impl FnMut(char) -> bool) -> string {
    s.as_ref().trim_start_matches(|c: char| f(c)).to_string()
}

/// `strings.TrimRightFunc(s, f)` — trim runes satisfying f from the end.
#[allow(non_snake_case)]
pub fn TrimRightFunc(s: impl AsRef<str>, mut f: impl FnMut(char) -> bool) -> string {
    s.as_ref().trim_end_matches(|c: char| f(c)).to_string()
}

/// `strings.SplitAfter(s, sep)` — like Split but keeps sep at the end of
/// each chunk.
#[allow(non_snake_case)]
pub fn SplitAfter(s: impl AsRef<str>, sep: impl AsRef<str>) -> slice<string> {
    let s = s.as_ref(); let sep = sep.as_ref();
    if sep.is_empty() {
        // Go's behavior: split after every rune.
        return s.chars().map(|c| c.to_string()).collect();
    }
    let mut out = Vec::new();
    let mut start = 0usize;
    loop {
        match s[start..].find(sep) {
            Some(i) => {
                let end = start + i + sep.len();
                out.push(s[start..end].to_string());
                start = end;
            }
            None => {
                out.push(s[start..].to_string());
                break;
            }
        }
    }
    out
}

/// `strings.FieldsFunc(s, f)` — split s around runs of runes where f is true.
#[allow(non_snake_case)]
pub fn FieldsFunc(s: impl AsRef<str>, mut f: impl FnMut(char) -> bool) -> slice<string> {
    s.as_ref()
        .split(|c: char| f(c))
        .filter(|seg| !seg.is_empty())
        .map(|seg| seg.to_string())
        .collect()
}

#[allow(non_snake_case)]
pub fn Title(s: impl AsRef<str>) -> string {
    // Go's deprecated strings.Title: uppercase the first letter of each word.
    let s = s.as_ref();
    let mut out = String::with_capacity(s.len());
    let mut at_word_boundary = true;
    for c in s.chars() {
        if c.is_whitespace() {
            at_word_boundary = true;
            out.push(c);
        } else if at_word_boundary {
            for uc in c.to_uppercase() {
                out.push(uc);
            }
            at_word_boundary = false;
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_and_prefix() {
        assert!(Contains("hello world", "world"));
        assert!(!Contains("hello", "xyz"));
        assert!(HasPrefix("foobar", "foo"));
        assert!(HasSuffix("foobar", "bar"));
    }

    #[test]
    fn index_returns_minus_one_when_absent() {
        assert_eq!(Index("hello", "ll"), 2);
        assert_eq!(Index("hello", "z"), -1);
        assert_eq!(LastIndex("banana", "an"), 3);
    }

    #[test]
    fn count_substr_and_empty() {
        assert_eq!(Count("banana", "a"), 3);
        assert_eq!(Count("xx", ""), 3); // Go: chars+1
    }

    #[test]
    fn split_and_join() {
        let v = Split("a,b,c", ",");
        assert_eq!(v, vec!["a", "b", "c"]);
        assert_eq!(Join(&v, "-"), "a-b-c");
    }

    #[test]
    fn split_n_caps_results() {
        let v = SplitN("a,b,c,d", ",", 2);
        assert_eq!(v, vec!["a", "b,c,d"]);
        let v = SplitN("a,b,c", ",", -1);
        assert_eq!(v.len(), 3);
        let v = SplitN("a,b,c", ",", 0);
        assert!(v.is_empty());
    }

    #[test]
    fn replace_and_replace_all() {
        assert_eq!(Replace("aaa", "a", "b", 2), "bba");
        assert_eq!(ReplaceAll("aaa", "a", "b"), "bbb");
    }

    #[test]
    fn case_change() {
        assert_eq!(ToUpper("hello"), "HELLO");
        assert_eq!(ToLower("HELLO"), "hello");
    }

    #[test]
    fn trim_variants() {
        assert_eq!(TrimSpace("  hi  "), "hi");
        assert_eq!(TrimPrefix("foobar", "foo"), "bar");
        assert_eq!(TrimSuffix("foobar", "bar"), "foo");
        assert_eq!(Trim("---abc--", "-"), "abc");
    }

    #[test]
    fn fields_splits_on_whitespace() {
        assert_eq!(Fields("  a  b\tc\n"), vec!["a", "b", "c"]);
    }

    #[test]
    fn repeat_and_equalfold() {
        assert_eq!(Repeat("ab", 3), "ababab");
        assert!(EqualFold("HELLO", "hello"));
        assert!(!EqualFold("hello", "world"));
    }

    #[test]
    fn builder_writes_and_resets() {
        let mut b = Builder::new();
        b.WriteString("hello ");
        b.WriteString("world");
        b.WriteByte(b'!');
        b.WriteRune('λ');
        assert_eq!(b.String(), "hello world!λ");
        assert_eq!(b.Len(), "hello world!λ".len() as int);
        b.Reset();
        assert_eq!(b.Len(), 0);
    }

    #[test]
    fn builder_writerune_returns_bytes_written() {
        let mut b = Builder::new();
        let (n, _) = b.WriteRune('a');
        assert_eq!(n, 1);
        let (n, _) = b.WriteRune('λ');
        assert_eq!(n, 2);
        let (n, _) = b.WriteRune('漢');
        assert_eq!(n, 3);
    }

    #[test]
    fn reader_reads_bytes() {
        let mut r = NewReader("hello");
        let mut buf = [0u8; 3];
        let (n, _) = r.Read(&mut buf);
        assert_eq!(n, 3);
        assert_eq!(&buf, b"hel");
        let (n, _) = r.Read(&mut buf);
        assert_eq!(n, 2);
        assert_eq!(&buf[..2], b"lo");
    }

    #[test]
    fn reader_seek() {
        let mut r = NewReader("abcdef");
        r.Seek(2, 0);
        let (b, _) = r.ReadByte();
        assert_eq!(b, b'c');
        r.Seek(-1, 2);
        let (b, _) = r.ReadByte();
        assert_eq!(b, b'f');
    }

    #[test]
    fn replacer_replaces_multiple() {
        let r = NewReplacer(&["a", "1", "b", "2", "c", "3"]);
        assert_eq!(r.Replace("abc cab"), "123 312");
    }

    #[test]
    fn replacer_leaves_unmatched() {
        let r = NewReplacer(&["foo", "FOO"]);
        assert_eq!(r.Replace("foo bar baz"), "FOO bar baz");
    }

    #[test]
    fn map_transforms_chars() {
        let shout = Map(|c| c.to_ascii_uppercase(), "hello");
        assert_eq!(shout, "HELLO");
        let drop_vowels = Map(|c| if "aeiouAEIOU".contains(c) { '\0' } else { c }, "HELLO");
        assert_eq!(drop_vowels, "HLL");
    }

    #[test]
    fn contains_any_and_index_any() {
        assert!(ContainsAny("hello", "xyz!o"));
        assert!(!ContainsAny("hello", "xyz"));
        assert_eq!(IndexAny("hello", "lo"), 2);
        assert_eq!(IndexAny("abc", "xyz"), -1);
        assert_eq!(IndexRune("héllo", 'é'), 1);
        assert_eq!(IndexByte("hello", b'l'), 2);
    }

    #[test]
    fn title_upcases_words() {
        assert_eq!(Title("hello world"), "Hello World");
    }
}
