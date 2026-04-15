// regexp: Go's regexp package (RE2 syntax) — backed by the `regex` crate.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   re, err := regexp.Compile("a.c")    let (re, err) = regexp::Compile("a.c");
//   re := regexp.MustCompile("a.c")     let re = regexp::MustCompile("a.c");
//   re.MatchString("abc")               re.MatchString("abc")
//   re.FindString("aabca")              re.FindString("aabca")
//   re.FindAllString(s, n)              re.FindAllString(s, n)
//   re.ReplaceAllString(s, "X")         re.ReplaceAllString(s, "X")
//   re.Split(s, -1)                     re.Split(s, -1)
//   regexp.MatchString(pat, s)          regexp::MatchString(pat, s)
//
// Both Go's regexp and the `regex` crate use RE2-style syntax, so patterns
// port verbatim. Go uses `$1` / `${name}` for replace backrefs — also
// compatible.

use crate::errors::{error, nil, New};
use crate::types::{int, slice, string};

pub struct Regexp {
    inner: regex::Regex,
    pub(crate) src: String,
}

impl Clone for Regexp {
    fn clone(&self) -> Self {
        // Regex is Send+Sync but not Clone; rebuild from source.
        Regexp { inner: regex::Regex::new(&self.src).unwrap(), src: self.src.clone() }
    }
}

impl Regexp {
    pub fn MatchString(&self, s: impl AsRef<str>) -> bool {
        self.inner.is_match(s.as_ref())
    }

    pub fn Match(&self, b: &[crate::types::byte]) -> bool {
        match std::str::from_utf8(b) {
            Ok(s) => self.inner.is_match(s),
            Err(_) => false,
        }
    }

    /// re.FindString(s) — first match or "" if none.
    pub fn FindString(&self, s: impl AsRef<str>) -> string {
        self.inner.find(s.as_ref()).map(|m| m.as_str().to_string()).unwrap_or_default()
    }

    /// re.FindStringIndex(s) — returns [start, end] byte indices, or empty slice.
    pub fn FindStringIndex(&self, s: impl AsRef<str>) -> slice<int> {
        match self.inner.find(s.as_ref()) {
            Some(m) => vec![m.start() as int, m.end() as int],
            None => Vec::new(),
        }
    }

    /// re.FindAllString(s, n) — up to n matches (n<0 = all).
    pub fn FindAllString(&self, s: impl AsRef<str>, n: int) -> slice<string> {
        let s_ref = s.as_ref();
        let iter = self.inner.find_iter(s_ref).map(|m| m.as_str().to_string());
        if n < 0 {
            iter.collect()
        } else {
            iter.take(n as usize).collect()
        }
    }

    /// re.FindStringSubmatch(s) — first match + its capture groups.
    /// Group 0 is the whole match. Returns empty slice if no match.
    pub fn FindStringSubmatch(&self, s: impl AsRef<str>) -> slice<string> {
        match self.inner.captures(s.as_ref()) {
            Some(caps) => (0..caps.len())
                .map(|i| caps.get(i).map(|m| m.as_str().to_string()).unwrap_or_default())
                .collect(),
            None => Vec::new(),
        }
    }

    pub fn FindAllStringSubmatch(&self, s: impl AsRef<str>, n: int) -> Vec<slice<string>> {
        let iter = self.inner.captures_iter(s.as_ref()).map(|caps| {
            (0..caps.len())
                .map(|i| caps.get(i).map(|m| m.as_str().to_string()).unwrap_or_default())
                .collect::<Vec<_>>()
        });
        if n < 0 { iter.collect() } else { iter.take(n as usize).collect() }
    }

    /// re.ReplaceAllString(s, repl) — Go-style `$1` / `$name` refs.
    pub fn ReplaceAllString(&self, s: impl AsRef<str>, repl: impl AsRef<str>) -> string {
        self.inner.replace_all(s.as_ref(), repl.as_ref()).into_owned()
    }

    pub fn ReplaceAllLiteralString(&self, s: impl AsRef<str>, repl: impl AsRef<str>) -> string {
        let replacer = regex::NoExpand(repl.as_ref());
        self.inner.replace_all(s.as_ref(), replacer).into_owned()
    }

    /// re.ReplaceAllStringFunc(s, |m| ...)
    pub fn ReplaceAllStringFunc<F>(&self, s: impl AsRef<str>, mut f: F) -> string
    where
        F: FnMut(string) -> string,
    {
        self.inner.replace_all(s.as_ref(), |caps: &regex::Captures| {
            f(caps.get(0).map(|m| m.as_str().to_string()).unwrap_or_default())
        }).into_owned()
    }

    /// re.Split(s, n) — split s around matches. n<0 = all.
    pub fn Split(&self, s: impl AsRef<str>, n: int) -> slice<string> {
        let s_ref = s.as_ref();
        if n == 0 { return Vec::new(); }
        let mut out: slice<string> = Vec::new();
        let mut last = 0usize;
        let mut count = 0i64;
        for m in self.inner.find_iter(s_ref) {
            if n > 0 && count >= n - 1 { break; }
            out.push(s_ref[last..m.start()].to_string());
            last = m.end();
            count += 1;
        }
        out.push(s_ref[last..].to_string());
        out
    }

    pub fn NumSubexp(&self) -> int {
        self.inner.captures_len() as int - 1
    }

    pub fn String(&self) -> string { self.src.clone() }
}

#[allow(non_snake_case)]
pub fn Compile(pat: impl AsRef<str>) -> (Regexp, error) {
    match regex::Regex::new(pat.as_ref()) {
        Ok(r) => (Regexp { inner: r, src: pat.as_ref().to_string() }, nil),
        Err(e) => (dummy_regexp(), New(&e.to_string())),
    }
}

#[allow(non_snake_case)]
pub fn MustCompile(pat: impl AsRef<str>) -> Regexp {
    let (re, err) = Compile(pat);
    if err != nil { panic!("regexp: MustCompile: {}", err); }
    re
}

#[allow(non_snake_case)]
pub fn MatchString(pat: impl AsRef<str>, s: impl AsRef<str>) -> (bool, error) {
    let (re, err) = Compile(pat);
    if err != nil { return (false, err); }
    (re.MatchString(s), nil)
}

#[allow(non_snake_case)]
pub fn QuoteMeta(s: impl AsRef<str>) -> string {
    regex::escape(s.as_ref())
}

// Internal: a never-used "empty" regex returned as a placeholder when
// Compile fails. Callers must always check err.
fn dummy_regexp() -> Regexp {
    static DUMMY_SRC: &str = "";
    Regexp {
        inner: regex::Regex::new(DUMMY_SRC).unwrap(),
        src: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_string_basic() {
        let (re, err) = Compile(r"a.c");
        assert_eq!(err, nil);
        assert!(re.MatchString("abc"));
        assert!(re.MatchString("a_c"));
        assert!(!re.MatchString("ac"));
    }

    #[test]
    fn bad_pattern_returns_error() {
        let (_re, err) = Compile(r"([");
        assert!(err != nil);
    }

    #[test]
    fn must_compile_panics_on_bad() {
        let result = std::panic::catch_unwind(|| MustCompile(r"([") );
        assert!(result.is_err());
    }

    #[test]
    fn find_string_and_all() {
        let re = MustCompile(r"\d+");
        assert_eq!(re.FindString("a1b22c333"), "1");
        assert_eq!(re.FindAllString("a1b22c333", -1), vec!["1", "22", "333"]);
        assert_eq!(re.FindAllString("a1b22c333", 2), vec!["1", "22"]);
    }

    #[test]
    fn find_string_submatch() {
        let re = MustCompile(r"(\w+)=(\d+)");
        let caps = re.FindStringSubmatch("port=8080");
        assert_eq!(caps, vec!["port=8080", "port", "8080"]);
    }

    #[test]
    fn replace_all_string() {
        let re = MustCompile(r"\d+");
        assert_eq!(re.ReplaceAllString("a1 b22 c333", "X"), "aX bX cX");
    }

    #[test]
    fn replace_with_backref() {
        let re = MustCompile(r"(\w+)=(\d+)");
        assert_eq!(re.ReplaceAllString("port=80 ttl=60", "$2:$1"), "80:port 60:ttl");
    }

    #[test]
    fn replace_with_func() {
        let re = MustCompile(r"\d+");
        let out = re.ReplaceAllStringFunc("a1 b22", |m| format!("[{}]", m));
        assert_eq!(out, "a[1] b[22]");
    }

    #[test]
    fn split_n() {
        let re = MustCompile(r",\s*");
        assert_eq!(re.Split("a, b,  c,   d", -1), vec!["a", "b", "c", "d"]);
        assert_eq!(re.Split("a, b,  c,   d", 2), vec!["a", "b,  c,   d"]);
    }

    #[test]
    fn match_string_global_helper() {
        let (ok, err) = MatchString(r"^\d+$", "12345");
        assert_eq!(err, nil);
        assert!(ok);
    }

    #[test]
    fn quote_meta_escapes() {
        assert_eq!(QuoteMeta("1.2.3"), "1\\.2\\.3");
    }

    #[test]
    fn num_subexp_counts_groups() {
        let re = MustCompile(r"(\w+)-(\d+)");
        assert_eq!(re.NumSubexp(), 2);
    }
}
