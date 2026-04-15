// path: Go's path package — slash-separated paths, for URLs and similar.
//
// Unlike `filepath`, this package always uses '/' as the separator regardless
// of platform. Use this for URL-shaped strings; use `filepath` for actual
// filesystem paths.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   path.Join("a", "b", "c")            path::Join(&["a","b","c"])
//   path.Base("/a/b.txt")               path::Base("/a/b.txt")
//   path.Dir("/a/b.txt")                path::Dir("/a/b.txt")
//   path.Ext("name.tar.gz")             path::Ext("name.tar.gz")
//   path.Clean("a/./b//c/../d")         path::Clean("a/./b//c/../d")
//   path.IsAbs("/foo")                  path::IsAbs("/foo")
//   path.Split("/a/b.txt")              path::Split("/a/b.txt")

use crate::types::string;

const SEP: char = '/';

#[allow(non_snake_case)]
pub fn Join(parts: &[impl AsRef<str>]) -> string {
    let joined: Vec<&str> = parts.iter().map(|p| p.as_ref()).filter(|s| !s.is_empty()).collect();
    if joined.is_empty() {
        return String::new();
    }
    Clean(joined.join(&SEP.to_string()))
}

#[allow(non_snake_case)]
pub fn Base(p: impl AsRef<str>) -> string {
    let s = p.as_ref();
    if s.is_empty() {
        return ".".to_string();
    }
    let trimmed = s.trim_end_matches(SEP);
    if trimmed.is_empty() {
        return SEP.to_string();
    }
    match trimmed.rsplit_once(SEP) {
        Some((_, tail)) => tail.to_string(),
        None => trimmed.to_string(),
    }
}

#[allow(non_snake_case)]
pub fn Dir(p: impl AsRef<str>) -> string {
    let s = p.as_ref();
    let trimmed = s.trim_end_matches(SEP);
    match trimmed.rsplit_once(SEP) {
        Some((head, _)) => {
            if head.is_empty() {
                SEP.to_string()
            } else {
                Clean(head.to_string())
            }
        }
        None => ".".to_string(),
    }
}

#[allow(non_snake_case)]
pub fn Ext(p: impl AsRef<str>) -> string {
    let s = p.as_ref();
    let base = Base(s);
    match base.rfind('.') {
        Some(i) if i > 0 => base[i..].to_string(),
        _ => String::new(),
    }
}

#[allow(non_snake_case)]
pub fn Clean(p: impl Into<String>) -> string {
    let s: String = p.into();
    if s.is_empty() {
        return ".".to_string();
    }
    let absolute = s.starts_with(SEP);
    let mut stack: Vec<&str> = Vec::new();
    for part in s.split(SEP) {
        match part {
            "" | "." => continue,
            ".." => {
                if stack.last().map_or(false, |t| *t != "..") && !stack.is_empty() {
                    stack.pop();
                } else if !absolute {
                    stack.push("..");
                }
            }
            other => stack.push(other),
        }
    }
    let joined = stack.join(&SEP.to_string());
    if absolute {
        if joined.is_empty() { SEP.to_string() } else { format!("{}{}", SEP, joined) }
    } else if joined.is_empty() {
        ".".to_string()
    } else {
        joined
    }
}

#[allow(non_snake_case)]
pub fn IsAbs(p: impl AsRef<str>) -> bool {
    p.as_ref().starts_with(SEP)
}

/// path.Split(p) — splits into dir (with trailing slash) and file.
#[allow(non_snake_case)]
pub fn Split(p: impl AsRef<str>) -> (string, string) {
    let s = p.as_ref();
    match s.rfind(SEP) {
        Some(i) => (s[..=i].to_string(), s[i + 1..].to_string()),
        None => (String::new(), s.to_string()),
    }
}

/// path.Match(pattern, name) — shell-style pattern match (* ? [])
#[allow(non_snake_case)]
pub fn Match(pattern: impl AsRef<str>, name: impl AsRef<str>) -> (bool, crate::errors::error) {
    match glob_match(pattern.as_ref(), name.as_ref()) {
        Ok(b) => (b, crate::errors::nil),
        Err(e) => (false, crate::errors::New(&e)),
    }
}

fn glob_match(pat: &str, name: &str) -> Result<bool, String> {
    // Minimal impl matching Go's path.Match semantics:
    //   '*' matches any run of non-'/' characters
    //   '?' matches any single non-'/' character
    //   '[...]' character class (with optional '^' negation)
    fn m(p: &[char], n: &[char]) -> Result<bool, String> {
        let mut pi = 0usize;
        let mut ni = 0usize;
        let mut star_p: Option<usize> = None;
        let mut star_n: usize = 0;
        while ni < n.len() {
            if pi < p.len() {
                match p[pi] {
                    '*' => {
                        star_p = Some(pi);
                        star_n = ni;
                        pi += 1;
                        continue;
                    }
                    '?' => {
                        if n[ni] == '/' { /* cannot match slash */ }
                        else { pi += 1; ni += 1; continue; }
                    }
                    '[' => {
                        // find closing ]
                        let mut end = pi + 1;
                        let negate = end < p.len() && p[end] == '^';
                        if negate { end += 1; }
                        while end < p.len() && p[end] != ']' { end += 1; }
                        if end >= p.len() {
                            return Err("syntax error in pattern".into());
                        }
                        let class = &p[pi + 1 + negate as usize..end];
                        let mut matched = false;
                        let mut i = 0;
                        while i < class.len() {
                            if i + 2 < class.len() && class[i + 1] == '-' {
                                if class[i] <= n[ni] && n[ni] <= class[i + 2] {
                                    matched = true;
                                    break;
                                }
                                i += 3;
                            } else {
                                if class[i] == n[ni] { matched = true; break; }
                                i += 1;
                            }
                        }
                        let ok = matched != negate;
                        if ok && n[ni] != '/' {
                            pi = end + 1;
                            ni += 1;
                            continue;
                        }
                    }
                    c => {
                        if c == n[ni] { pi += 1; ni += 1; continue; }
                    }
                }
            }
            if let Some(sp) = star_p {
                if n[star_n] != '/' {
                    star_n += 1;
                    ni = star_n;
                    pi = sp + 1;
                    continue;
                }
            }
            return Ok(false);
        }
        while pi < p.len() && p[pi] == '*' { pi += 1; }
        Ok(pi == p.len())
    }
    let p: Vec<char> = pat.chars().collect();
    let n: Vec<char> = name.chars().collect();
    m(&p, &n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_basic() {
        assert_eq!(Join(&["a", "b", "c"]), "a/b/c");
        assert_eq!(Join(&["/a", "b"]), "/a/b");
        assert_eq!(Join(&["a", "", "b"]), "a/b");
    }

    #[test]
    fn base_cases() {
        assert_eq!(Base("/a/b/c.txt"), "c.txt");
        assert_eq!(Base("c.txt"), "c.txt");
        assert_eq!(Base("/"), "/");
        assert_eq!(Base(""), ".");
    }

    #[test]
    fn dir_cases() {
        assert_eq!(Dir("/a/b/c.txt"), "/a/b");
        assert_eq!(Dir("c.txt"), ".");
        assert_eq!(Dir("/a"), "/");
    }

    #[test]
    fn ext_cases() {
        assert_eq!(Ext("a/b.txt"), ".txt");
        assert_eq!(Ext("name.tar.gz"), ".gz");
        assert_eq!(Ext("noext"), "");
    }

    #[test]
    fn clean_normalizes() {
        assert_eq!(Clean("a/./b//c/../d"), "a/b/d");
        assert_eq!(Clean("/a/../b"), "/b");
        assert_eq!(Clean(""), ".");
        assert_eq!(Clean("/"), "/");
    }

    #[test]
    fn is_abs_and_split() {
        assert!(IsAbs("/foo"));
        assert!(!IsAbs("foo"));
        let (d, f) = Split("/a/b/c.txt");
        assert_eq!(d, "/a/b/");
        assert_eq!(f, "c.txt");
    }

    #[test]
    fn match_star_and_question() {
        assert!(Match("*.txt", "hello.txt").0);
        assert!(!Match("*.txt", "hello.md").0);
        assert!(Match("?at", "cat").0);
        assert!(!Match("?at", "cats").0);
        assert!(Match("[a-c]at", "bat").0);
        assert!(!Match("[a-c]at", "dat").0);
    }
}
