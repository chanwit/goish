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
pub fn EqualFold(s: impl AsRef<str>, t: impl AsRef<str>) -> bool {
    s.as_ref().eq_ignore_ascii_case(t.as_ref())
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
}
