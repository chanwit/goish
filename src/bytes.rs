// bytes: Go's bytes package — manipulate byte slices with the same vocabulary
// as the strings package.
//
//   Go                                goish
//   ───────────────────────────────   ──────────────────────────────────
//   var buf bytes.Buffer              let mut buf = bytes::Buffer::new();
//   bytes.Equal(a, b)                 bytes::Equal(&a, &b)
//   bytes.Compare(a, b)               bytes::Compare(&a, &b)
//   bytes.Contains(s, []byte("x"))    bytes::Contains(&s, b"x")
//   bytes.Index(s, sep)               bytes::Index(&s, &sep)
//   bytes.Split(s, sep)               bytes::Split(&s, &sep)
//   bytes.Join(parts, sep)            bytes::Join(&parts, &sep)
//   bytes.Trim(s, "abc")              bytes::Trim(&s, b"abc")
//   r := bytes.NewReader(b)           let r = bytes::NewReader(b);
//
// Buffer implements `io::Write`, so any goish Fprintf! / write! call works
// directly. It also implements Display so `fmt::Println!("buf:", buf)`
// prints the contents (lossy for non-UTF-8 bytes).

use crate::errors::{error, nil};
use crate::types::{byte, int};
use std::io;

// ── Buffer ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct Buffer {
    inner: Vec<u8>,
    read_pos: usize,
}

impl Buffer {
    pub fn new() -> Self {
        Buffer::default()
    }

    /// buf.Bytes() — borrowed view of the unread contents.
    pub fn Bytes(&self) -> &[byte] {
        &self.inner[self.read_pos..]
    }

    /// buf.String() — UTF-8 contents as a string (lossy for invalid bytes).
    pub fn String(&self) -> String {
        String::from_utf8_lossy(&self.inner[self.read_pos..]).into_owned()
    }

    /// buf.Len() — number of unread bytes.
    pub fn Len(&self) -> int {
        (self.inner.len() - self.read_pos) as int
    }

    /// Lowercase alias for the polymorphic `len!()` macro.
    pub fn len(&self) -> usize {
        self.inner.len() - self.read_pos
    }

    /// buf.Cap() — current capacity.
    pub fn Cap(&self) -> int {
        self.inner.capacity() as int
    }

    /// buf.Reset() — discard all contents (capacity preserved).
    pub fn Reset(&mut self) {
        self.inner.clear();
        self.read_pos = 0;
    }

    /// buf.Grow(n) — ensure room for at least n more bytes without reallocating.
    pub fn Grow(&mut self, n: int) {
        if n > 0 {
            self.inner.reserve(n as usize);
        }
    }

    /// buf.Truncate(n) — keep the first n unread bytes, drop the rest.
    pub fn Truncate(&mut self, n: int) {
        let keep = self.read_pos + n as usize;
        self.inner.truncate(keep);
    }

    /// buf.Write(p) — append a byte slice. Always returns nil error.
    pub fn Write(&mut self, p: &[byte]) -> (int, error) {
        self.inner.extend_from_slice(p);
        (p.len() as int, nil)
    }

    /// buf.WriteString(s) — append a string's bytes.
    pub fn WriteString(&mut self, s: &str) -> (int, error) {
        self.inner.extend_from_slice(s.as_bytes());
        (s.len() as int, nil)
    }

    /// buf.WriteByte(b) — append a single byte.
    pub fn WriteByte(&mut self, b: byte) -> error {
        self.inner.push(b);
        nil
    }

    /// buf.Read(p) — consume from the front into p.
    pub fn Read(&mut self, p: &mut [byte]) -> (int, error) {
        if self.read_pos >= self.inner.len() {
            return (0, crate::io::EOF());
        }
        let available = &self.inner[self.read_pos..];
        let n = available.len().min(p.len());
        p[..n].copy_from_slice(&available[..n]);
        self.read_pos += n;
        (n as int, nil)
    }

    /// buf.ReadByte() — consume and return one byte.
    pub fn ReadByte(&mut self) -> (byte, error) {
        if self.read_pos >= self.inner.len() {
            return (0, crate::io::EOF());
        }
        let b = self.inner[self.read_pos];
        self.read_pos += 1;
        (b, nil)
    }

    /// buf.Next(n) — return the next n bytes (or fewer on EOF) and advance.
    pub fn Next(&mut self, n: int) -> Vec<byte> {
        let available = self.inner.len() - self.read_pos;
        let take = (n as usize).min(available);
        let out = self.inner[self.read_pos..self.read_pos + take].to_vec();
        self.read_pos += take;
        out
    }
}

impl io::Write for Buffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl io::Read for Buffer {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.read_pos >= self.inner.len() {
            return Ok(0);
        }
        let available = &self.inner[self.read_pos..];
        let n = available.len().min(buf.len());
        buf[..n].copy_from_slice(&available[..n]);
        self.read_pos += n;
        Ok(n)
    }
}

impl std::fmt::Display for Buffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(&self.inner[self.read_pos..]))
    }
}

/// bytes.NewBuffer(b) — take ownership of b as the initial contents.
/// Accepts anything convertible to `Vec<byte>`, including goish's
/// `slice<byte>` (so `bytes::NewBuffer(make!([]byte, n))` works without
/// `.into_vec()` at the call site).
#[allow(non_snake_case)]
pub fn NewBuffer(b: impl Into<Vec<byte>>) -> Buffer {
    Buffer { inner: b.into(), read_pos: 0 }
}

/// bytes.NewBufferString(s) — start a buffer from a string.
#[allow(non_snake_case)]
pub fn NewBufferString(s: impl AsRef<str>) -> Buffer {
    Buffer { inner: s.as_ref().as_bytes().to_vec(), read_pos: 0 }
}

// ── Reader ────────────────────────────────────────────────────────────

/// bytes.Reader — wraps a byte slice as an io.Reader + io.Seeker.
#[derive(Debug, Clone)]
pub struct Reader {
    data: Vec<byte>,
    pos: usize,
}

impl Reader {
    pub fn Len(&self) -> int {
        (self.data.len().saturating_sub(self.pos)) as int
    }

    pub fn Size(&self) -> int64_ {
        self.data.len() as int64_
    }

    pub fn Read(&mut self, p: &mut [byte]) -> (int, error) {
        if self.pos >= self.data.len() {
            return (0, crate::io::EOF());
        }
        let n = (self.data.len() - self.pos).min(p.len());
        p[..n].copy_from_slice(&self.data[self.pos..self.pos + n]);
        self.pos += n;
        (n as int, nil)
    }

    pub fn ReadByte(&mut self) -> (byte, error) {
        if self.pos >= self.data.len() {
            return (0, crate::io::EOF());
        }
        let b = self.data[self.pos];
        self.pos += 1;
        (b, nil)
    }

    pub fn UnreadByte(&mut self) -> error {
        if self.pos == 0 {
            return crate::errors::New("bytes.Reader.UnreadByte: at beginning of slice");
        }
        self.pos -= 1;
        nil
    }

    pub fn Seek(&mut self, offset: int64_, whence: int) -> (int64_, error) {
        let new_pos: i64 = match whence {
            0 => offset,                          // SeekStart
            1 => self.pos as i64 + offset,        // SeekCurrent
            2 => self.data.len() as i64 + offset, // SeekEnd
            _ => return (0, crate::errors::New("bytes.Reader.Seek: invalid whence")),
        };
        if new_pos < 0 {
            return (0, crate::errors::New("bytes.Reader.Seek: negative position"));
        }
        self.pos = new_pos as usize;
        (new_pos, nil)
    }
}

impl io::Read for Reader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.pos >= self.data.len() {
            return Ok(0);
        }
        let n = (self.data.len() - self.pos).min(buf.len());
        buf[..n].copy_from_slice(&self.data[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }
}

/// bytes.NewReader(b) — construct a Reader over the given bytes.
#[allow(non_snake_case)]
pub fn NewReader(b: impl Into<Vec<byte>>) -> Reader {
    Reader { data: b.into(), pos: 0 }
}

// ── Package-level helpers ─────────────────────────────────────────────

use crate::types::int64 as int64_;

#[allow(non_snake_case)]
pub fn Equal(a: &[byte], b: &[byte]) -> bool {
    a == b
}

#[allow(non_snake_case)]
pub fn Compare(a: &[byte], b: &[byte]) -> int {
    use std::cmp::Ordering::*;
    match a.cmp(b) {
        Less => -1,
        Equal => 0,
        Greater => 1,
    }
}

#[allow(non_snake_case)]
pub fn Contains(s: &[byte], sub: &[byte]) -> bool {
    Index(s, sub) >= 0
}

#[allow(non_snake_case)]
pub fn ContainsAny(s: &[byte], chars: impl AsRef<str>) -> bool {
    let set: Vec<byte> = chars.as_ref().as_bytes().to_vec();
    s.iter().any(|b| set.contains(b))
}

#[allow(non_snake_case)]
pub fn ContainsRune(s: &[byte], r: char) -> bool {
    let mut buf = [0u8; 4];
    let encoded = r.encode_utf8(&mut buf).as_bytes();
    Contains(s, encoded)
}

#[allow(non_snake_case)]
pub fn HasPrefix(s: &[byte], prefix: &[byte]) -> bool {
    s.starts_with(prefix)
}

#[allow(non_snake_case)]
pub fn HasSuffix(s: &[byte], suffix: &[byte]) -> bool {
    s.ends_with(suffix)
}

#[allow(non_snake_case)]
pub fn Index(s: &[byte], sep: &[byte]) -> int {
    if sep.is_empty() {
        return 0;
    }
    if sep.len() > s.len() {
        return -1;
    }
    for i in 0..=s.len() - sep.len() {
        if &s[i..i + sep.len()] == sep {
            return i as int;
        }
    }
    -1
}

#[allow(non_snake_case)]
pub fn IndexByte(s: &[byte], b: byte) -> int {
    s.iter().position(|x| *x == b).map(|i| i as int).unwrap_or(-1)
}

#[allow(non_snake_case)]
pub fn LastIndex(s: &[byte], sep: &[byte]) -> int {
    if sep.is_empty() {
        return s.len() as int;
    }
    if sep.len() > s.len() {
        return -1;
    }
    for i in (0..=s.len() - sep.len()).rev() {
        if &s[i..i + sep.len()] == sep {
            return i as int;
        }
    }
    -1
}

#[allow(non_snake_case)]
pub fn LastIndexByte(s: &[byte], b: byte) -> int {
    s.iter().rposition(|x| *x == b).map(|i| i as int).unwrap_or(-1)
}

#[allow(non_snake_case)]
pub fn Count(s: &[byte], sep: &[byte]) -> int {
    if sep.is_empty() {
        // Go: count of non-overlapping sep in s; for empty sep, it's utf-8 rune count + 1.
        return (s.len() + 1) as int;
    }
    let mut n: int = 0;
    let mut i: usize = 0;
    while i + sep.len() <= s.len() {
        if &s[i..i + sep.len()] == sep {
            n += 1;
            i += sep.len();
        } else {
            i += 1;
        }
    }
    n
}

#[allow(non_snake_case)]
pub fn Split(s: &[byte], sep: &[byte]) -> Vec<Vec<byte>> {
    SplitN(s, sep, -1)
}

#[allow(non_snake_case)]
pub fn SplitN(s: &[byte], sep: &[byte], n: int) -> Vec<Vec<byte>> {
    if n == 0 {
        return Vec::new();
    }
    if sep.is_empty() {
        // Go: splits after each UTF-8 rune. Our slices are raw bytes; split per-byte.
        let iter = s.iter().map(|b| vec![*b]);
        if n < 0 {
            return iter.collect();
        }
        let mut out: Vec<Vec<byte>> = iter.take((n - 1) as usize).collect();
        if (out.len() as int) < n {
            let consumed: usize = out.iter().map(|v| v.len()).sum();
            if consumed < s.len() {
                out.push(s[consumed..].to_vec());
            }
        }
        return out;
    }
    let mut out = Vec::new();
    let mut start = 0usize;
    while (n < 0 || (out.len() as int) < n - 1) && start <= s.len() {
        match Index(&s[start..], sep) {
            -1 => break,
            i => {
                let i = i as usize;
                out.push(s[start..start + i].to_vec());
                start += i + sep.len();
            }
        }
    }
    out.push(s[start..].to_vec());
    out
}

#[allow(non_snake_case)]
pub fn Join(parts: &[Vec<byte>], sep: &[byte]) -> Vec<byte> {
    if parts.is_empty() {
        return Vec::new();
    }
    let total: usize = parts.iter().map(|p| p.len()).sum::<usize>()
        + sep.len() * (parts.len().saturating_sub(1));
    let mut out = Vec::with_capacity(total);
    for (i, p) in parts.iter().enumerate() {
        if i > 0 {
            out.extend_from_slice(sep);
        }
        out.extend_from_slice(p);
    }
    out
}

#[allow(non_snake_case)]
pub fn Replace(s: &[byte], old: &[byte], new: &[byte], n: int) -> Vec<byte> {
    if old.is_empty() || n == 0 {
        return s.to_vec();
    }
    let mut out = Vec::with_capacity(s.len());
    let mut i = 0usize;
    let mut replaced: int = 0;
    while i <= s.len() {
        if (n < 0 || replaced < n) && i + old.len() <= s.len() && &s[i..i + old.len()] == old {
            out.extend_from_slice(new);
            i += old.len();
            replaced += 1;
        } else if i < s.len() {
            out.push(s[i]);
            i += 1;
        } else {
            break;
        }
    }
    out
}

#[allow(non_snake_case)]
pub fn ReplaceAll(s: &[byte], old: &[byte], new: &[byte]) -> Vec<byte> {
    Replace(s, old, new, -1)
}

#[allow(non_snake_case)]
pub fn ToUpper(s: &[byte]) -> Vec<byte> {
    s.iter().map(|b| b.to_ascii_uppercase()).collect()
}

#[allow(non_snake_case)]
pub fn ToLower(s: &[byte]) -> Vec<byte> {
    s.iter().map(|b| b.to_ascii_lowercase()).collect()
}

#[allow(non_snake_case)]
pub fn TrimSpace(s: &[byte]) -> Vec<byte> {
    let is_space = |b: byte| matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c);
    let start = s.iter().position(|b| !is_space(*b)).unwrap_or(s.len());
    let end = s.iter().rposition(|b| !is_space(*b)).map(|i| i + 1).unwrap_or(start);
    s[start..end].to_vec()
}

#[allow(non_snake_case)]
pub fn Trim(s: &[byte], cutset: &[byte]) -> Vec<byte> {
    let start = s.iter().position(|b| !cutset.contains(b)).unwrap_or(s.len());
    let end = s.iter().rposition(|b| !cutset.contains(b)).map(|i| i + 1).unwrap_or(start);
    s[start..end].to_vec()
}

#[allow(non_snake_case)]
pub fn TrimLeft(s: &[byte], cutset: &[byte]) -> Vec<byte> {
    let start = s.iter().position(|b| !cutset.contains(b)).unwrap_or(s.len());
    s[start..].to_vec()
}

#[allow(non_snake_case)]
pub fn TrimRight(s: &[byte], cutset: &[byte]) -> Vec<byte> {
    let end = s.iter().rposition(|b| !cutset.contains(b)).map(|i| i + 1).unwrap_or(0);
    s[..end].to_vec()
}

#[allow(non_snake_case)]
pub fn TrimPrefix(s: &[byte], prefix: &[byte]) -> Vec<byte> {
    if s.starts_with(prefix) { s[prefix.len()..].to_vec() } else { s.to_vec() }
}

#[allow(non_snake_case)]
pub fn TrimSuffix(s: &[byte], suffix: &[byte]) -> Vec<byte> {
    if s.ends_with(suffix) { s[..s.len() - suffix.len()].to_vec() } else { s.to_vec() }
}

#[allow(non_snake_case)]
pub fn Fields(s: &[byte]) -> Vec<Vec<byte>> {
    let is_space = |b: byte| matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c);
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < s.len() {
        while i < s.len() && is_space(s[i]) { i += 1; }
        let start = i;
        while i < s.len() && !is_space(s[i]) { i += 1; }
        if start < i {
            out.push(s[start..i].to_vec());
        }
    }
    out
}

#[allow(non_snake_case)]
pub fn Repeat(s: &[byte], n: int) -> Vec<byte> {
    if n < 0 {
        panic!("bytes: negative Repeat count");
    }
    let mut out = Vec::with_capacity(s.len() * n as usize);
    for _ in 0..n {
        out.extend_from_slice(s);
    }
    out
}

#[allow(non_snake_case)]
pub fn EqualFold(a: &[byte], b: &[byte]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(x, y)| x.to_ascii_lowercase() == y.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_buffer_accepts_vec_or_slice() {
        // Vec<u8> — Rust-native path.
        let b = NewBuffer(vec![b'a', b'b']);
        assert_eq!(b.Bytes(), &[b'a', b'b']);
        // slice<byte> — what `make!([]byte, n)` returns.
        let s: crate::types::slice<byte> = crate::make!([]byte, 3);
        let b = NewBuffer(s);
        assert_eq!(b.Len(), 3);
    }

    #[test]
    fn write_string_appends() {
        let mut b = Buffer::new();
        b.WriteString("hello ");
        b.WriteString("world");
        assert_eq!(b.String(), "hello world");
        assert_eq!(b.Len(), 11);
    }

    #[test]
    fn write_byte_appends() {
        let mut b = Buffer::new();
        b.WriteByte(b'X');
        b.WriteByte(b'Y');
        assert_eq!(b.String(), "XY");
    }

    #[test]
    fn reset_clears_contents() {
        let mut b = Buffer::new();
        b.WriteString("data");
        b.Reset();
        assert_eq!(b.Len(), 0);
        assert_eq!(b.String(), "");
    }

    #[test]
    fn truncate_keeps_prefix() {
        let mut b = Buffer::new();
        b.WriteString("hello world");
        b.Truncate(5);
        assert_eq!(b.String(), "hello");
    }

    #[test]
    fn fprintf_to_buffer() {
        let mut b = Buffer::new();
        let _ = crate::Fprintf!(&mut b, "n=%d %s", 42, "ok");
        assert_eq!(b.String(), "n=42 ok");
    }

    #[test]
    fn display_prints_contents() {
        let mut b = Buffer::new();
        b.WriteString("displayed");
        assert_eq!(format!("{}", b), "displayed");
    }

    #[test]
    fn buffer_read_consumes() {
        let mut b = NewBufferString("hello");
        let mut dst = [0u8; 3];
        let (n, err) = b.Read(&mut dst);
        assert_eq!(n, 3);
        assert_eq!(err, nil);
        assert_eq!(&dst, b"hel");
        assert_eq!(b.String(), "lo");
    }

    #[test]
    fn buffer_read_eof() {
        let mut b = NewBufferString("");
        let mut dst = [0u8; 4];
        let (n, err) = b.Read(&mut dst);
        assert_eq!(n, 0);
        assert!(err != nil);
    }

    #[test]
    fn reader_seek_and_read() {
        let mut r = NewReader(b"abcdef".to_vec());
        let mut dst = [0u8; 2];
        let (n, _) = r.Read(&mut dst);
        assert_eq!(n, 2);
        assert_eq!(&dst, b"ab");
        r.Seek(0, 0);
        let (n, _) = r.Read(&mut dst);
        assert_eq!(n, 2);
        assert_eq!(&dst, b"ab");
        r.Seek(-1, 2);
        let (b1, _) = r.ReadByte();
        assert_eq!(b1, b'f');
    }

    #[test]
    fn equal_and_compare() {
        assert!(Equal(b"abc", b"abc"));
        assert!(!Equal(b"abc", b"abd"));
        assert_eq!(Compare(b"abc", b"abc"), 0);
        assert_eq!(Compare(b"abc", b"abd"), -1);
        assert_eq!(Compare(b"abd", b"abc"), 1);
    }

    #[test]
    fn contains_and_index() {
        assert!(Contains(b"hello world", b"world"));
        assert!(!Contains(b"hello", b"xyz"));
        assert_eq!(Index(b"hello", b"ll"), 2);
        assert_eq!(Index(b"hello", b"z"), -1);
        assert_eq!(IndexByte(b"hello", b'l'), 2);
        assert_eq!(LastIndex(b"banana", b"an"), 3);
        assert_eq!(LastIndexByte(b"hello", b'l'), 3);
    }

    #[test]
    fn has_prefix_suffix() {
        assert!(HasPrefix(b"foobar", b"foo"));
        assert!(HasSuffix(b"foobar", b"bar"));
    }

    #[test]
    fn split_and_join() {
        let v = Split(b"a,b,c", b",");
        assert_eq!(v, vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]);
        let j = Join(&v, b"-");
        assert_eq!(j, b"a-b-c");
    }

    #[test]
    fn split_n() {
        let v = SplitN(b"a,b,c,d", b",", 2);
        assert_eq!(v, vec![b"a".to_vec(), b"b,c,d".to_vec()]);
    }

    #[test]
    fn replace_variants() {
        assert_eq!(Replace(b"aaa", b"a", b"b", 2), b"bba");
        assert_eq!(ReplaceAll(b"aaa", b"a", b"b"), b"bbb");
    }

    #[test]
    fn case_change() {
        assert_eq!(ToUpper(b"hello"), b"HELLO");
        assert_eq!(ToLower(b"HELLO"), b"hello");
    }

    #[test]
    fn trim_variants() {
        assert_eq!(TrimSpace(b"  hi  "), b"hi");
        assert_eq!(Trim(b"---abc--", b"-"), b"abc");
        assert_eq!(TrimLeft(b"---abc--", b"-"), b"abc--");
        assert_eq!(TrimRight(b"---abc--", b"-"), b"---abc");
        assert_eq!(TrimPrefix(b"foobar", b"foo"), b"bar");
        assert_eq!(TrimSuffix(b"foobar", b"bar"), b"foo");
    }

    #[test]
    fn fields_splits_on_whitespace() {
        assert_eq!(Fields(b"  a  b\tc\n"), vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]);
    }

    #[test]
    fn repeat_and_equalfold() {
        assert_eq!(Repeat(b"ab", 3), b"ababab");
        assert!(EqualFold(b"HELLO", b"hello"));
        assert!(!EqualFold(b"hello", b"world"));
    }

    #[test]
    fn count_basic() {
        assert_eq!(Count(b"banana", b"a"), 3);
        assert_eq!(Count(b"aaaa", b"aa"), 2); // non-overlapping
    }
}
