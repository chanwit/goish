// filepath: Go's path/filepath (subset, Unix-style separators).
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   filepath.Join("a", "b", "c")        filepath::Join(&["a","b","c"])
//   filepath.Base("/a/b.txt")           filepath::Base("/a/b.txt")
//   filepath.Dir("/a/b.txt")            filepath::Dir("/a/b.txt")
//   filepath.Ext("name.tar.gz")         filepath::Ext("name.tar.gz")
//   filepath.Clean("a/./b//c/../d")     filepath::Clean("a/./b//c/../d")

use crate::types::string;

const SEP: char = std::path::MAIN_SEPARATOR;

/// filepath.Join — joins components with the path separator, cleaning the result.
#[allow(non_snake_case)]
pub fn Join(parts: &[impl AsRef<str>]) -> string {
    let joined: Vec<&str> = parts.iter().map(|p| p.as_ref()).filter(|s| !s.is_empty()).collect();
    if joined.is_empty() {
        return String::new();
    }
    let combined = joined.join(&SEP.to_string());
    Clean(combined)
}

/// filepath.Base — the last element of a path.
#[allow(non_snake_case)]
pub fn Base(path: impl AsRef<str>) -> string {
    let s = path.as_ref();
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

/// filepath.Dir — all but the last element, cleaned.
#[allow(non_snake_case)]
pub fn Dir(path: impl AsRef<str>) -> string {
    let s = path.as_ref();
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

/// filepath.Ext — extension including the dot, or empty.
#[allow(non_snake_case)]
pub fn Ext(path: impl AsRef<str>) -> string {
    let s = path.as_ref();
    let base = Base(s);
    match base.rfind('.') {
        Some(i) if i > 0 => base[i..].to_string(),
        _ => String::new(),
    }
}

/// filepath.Clean — normalize a/./b//c/../d  →  a/b/d.
#[allow(non_snake_case)]
pub fn Clean(path: impl Into<String>) -> string {
    let s: String = path.into();
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
        format!("{}{}", SEP, joined)
    } else if joined.is_empty() {
        ".".to_string()
    } else {
        joined
    }
}

/// filepath.Split — splits path into dir (including trailing separator)
/// and filename components.
#[allow(non_snake_case)]
pub fn Split(path: impl AsRef<str>) -> (string, string) {
    let s = path.as_ref();
    match s.rfind(SEP) {
        Some(i) => (s[..=i].to_string(), s[i+1..].to_string()),
        None => (String::new(), s.to_string()),
    }
}

/// filepath.SplitList — splits a PATH-like string by the OS separator.
/// On Unix that's ':'. Empty input returns an empty slice.
#[allow(non_snake_case)]
pub fn SplitList(path: impl AsRef<str>) -> crate::types::slice<string> {
    let s = path.as_ref();
    if s.is_empty() { return Vec::new(); }
    let list_sep = if cfg!(windows) { ';' } else { ':' };
    s.split(list_sep).map(|s| s.to_string()).collect()
}

/// filepath.IsAbs — reports whether path is absolute.
#[allow(non_snake_case)]
pub fn IsAbs(path: impl AsRef<str>) -> bool {
    path.as_ref().starts_with(SEP)
}

/// filepath.FromSlash — replaces each '/' with the OS separator.
#[allow(non_snake_case)]
pub fn FromSlash(path: impl AsRef<str>) -> string {
    if SEP == '/' { path.as_ref().to_string() }
    else { path.as_ref().replace('/', &SEP.to_string()) }
}

/// filepath.ToSlash — replaces each OS separator with '/'.
#[allow(non_snake_case)]
pub fn ToSlash(path: impl AsRef<str>) -> string {
    if SEP == '/' { path.as_ref().to_string() }
    else { path.as_ref().replace(SEP, "/") }
}

/// filepath.IsLocal — reports whether path is relative, does not start with an
/// element equal to "." or "..", and does not contain any element escaping the
/// current directory.
#[allow(non_snake_case)]
pub fn IsLocal(path: impl AsRef<str>) -> bool {
    let s = path.as_ref();
    if s.is_empty() { return false; }
    if IsAbs(s) { return false; }
    // Check for ".." escaping. Walk components and track depth.
    let mut depth: i64 = 0;
    for part in s.split(SEP) {
        match part {
            "" | "." => {}
            ".." => {
                depth -= 1;
                if depth < 0 { return false; }
            }
            _ => { depth += 1; }
        }
    }
    true
}

/// filepath.Match — glob match. Supports `*`, `?`, and `[...]` character classes.
#[allow(non_snake_case)]
pub fn Match(pattern: impl AsRef<str>, name: impl AsRef<str>) -> (bool, crate::errors::error) {
    let p = pattern.as_ref();
    let n = name.as_ref();
    match glob_match(p, n) {
        Ok(m) => (m, crate::errors::nil),
        Err(e) => (false, crate::errors::New(e)),
    }
}

fn glob_match(mut pattern: &str, mut name: &str) -> Result<bool, &'static str> {
    // Based on Go's Match implementation (path/filepath/match.go).
    'outer: loop {
        // scanChunk: absorb leading '*' and get the next chunk (without '*').
        let mut star = false;
        while !pattern.is_empty() && pattern.starts_with('*') {
            pattern = &pattern[1..];
            star = true;
        }
        // Find next '*' not inside brackets to delimit chunk.
        let (chunk, rest) = scan_chunk_no_leading_star(pattern);
        pattern = rest;

        // If chunk is empty (pattern was all stars + nothing else):
        if chunk.is_empty() {
            // Trailing '*' — matches any suffix not containing SEP.
            if star {
                return Ok(!name.contains(SEP));
            }
            return Ok(name.is_empty());
        }

        // Try matching chunk at this position.
        if !star {
            match match_chunk(chunk, name)? {
                Some(rest_name) => { name = rest_name; continue 'outer; }
                None => return Ok(false),
            }
        }
        // With a preceding star, try every shift.
        let mut i = 0;
        loop {
            match match_chunk(chunk, &name[i..])? {
                Some(rest_name) => {
                    // If this was the last chunk, ensure we consumed all of name.
                    if pattern.is_empty() && !rest_name.is_empty() {
                        // try further shifts
                    } else {
                        name = rest_name;
                        continue 'outer;
                    }
                }
                None => {}
            }
            if i >= name.len() { break; }
            // Advance by one byte (we've already validated UTF-8 handling in match_chunk).
            let first = name[i..].chars().next().unwrap();
            if first == SEP { break; }
            i += first.len_utf8();
        }
        return Ok(false);
    }
}

fn scan_chunk_no_leading_star(pattern: &str) -> (&str, &str) {
    let bytes = pattern.as_bytes();
    let mut i = 0;
    let mut in_range = false;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if !cfg!(windows) => {
                i += 1;
                if i < bytes.len() { i += 1; }
                continue;
            }
            b'[' => { in_range = true; i += 1; }
            b']' => { in_range = false; i += 1; }
            b'*' if !in_range => break,
            _ => { i += 1; }
        }
    }
    (&pattern[..i], &pattern[i..])
}

fn match_chunk<'a>(chunk: &str, name: &'a str) -> Result<Option<&'a str>, &'static str> {
    // Fully validate the chunk syntax by walking it to completion even on
    // mismatch. Matches Go's matchChunk, which returns ErrBadPattern on
    // malformed brackets regardless of match outcome.
    let chunk_bytes = chunk.as_bytes();
    let mut pi = 0;
    let mut failed = false;
    let mut rest_name: &'a str = name;

    while pi < chunk_bytes.len() {
        let c = chunk_bytes[pi];
        match c {
            b'[' => {
                let mut nc: char = '\0';
                let mut have_nc = false;
                if !failed && !rest_name.is_empty() {
                    nc = rest_name.chars().next().unwrap();
                    rest_name = &rest_name[nc.len_utf8()..];
                    have_nc = true;
                } else {
                    failed = true;
                }
                pi += 1;
                let mut negate = false;
                if pi < chunk_bytes.len() && (chunk_bytes[pi] == b'^' || chunk_bytes[pi] == b'!') {
                    negate = true;
                    pi += 1;
                }
                let mut matched = false;
                let mut nrange = 0;
                loop {
                    if pi < chunk_bytes.len() && chunk_bytes[pi] == b']' && nrange > 0 {
                        pi += 1;
                        break;
                    }
                    if pi >= chunk_bytes.len() {
                        return Err("syntax error in pattern");
                    }
                    let (lo, np) = get_esc(chunk_bytes, pi)?;
                    pi = np;
                    let hi = if pi < chunk_bytes.len() && chunk_bytes[pi] == b'-' {
                        pi += 1;
                        let (h, np2) = get_esc(chunk_bytes, pi)?;
                        pi = np2;
                        h
                    } else { lo };
                    if have_nc && lo <= nc && nc <= hi { matched = true; }
                    nrange += 1;
                }
                if matched == negate { failed = true; }
            }
            b'?' => {
                if !failed && !rest_name.is_empty() {
                    let nc = rest_name.chars().next().unwrap();
                    if nc == SEP { failed = true; }
                    else { rest_name = &rest_name[nc.len_utf8()..]; }
                } else { failed = true; }
                pi += 1;
            }
            b'\\' if !cfg!(windows) => {
                pi += 1;
                if pi >= chunk_bytes.len() { return Err("syntax error in pattern"); }
                if !failed && !rest_name.is_empty() {
                    let nc = rest_name.chars().next().unwrap();
                    if chunk_bytes[pi] as char != nc { failed = true; }
                    else { rest_name = &rest_name[nc.len_utf8()..]; }
                } else { failed = true; }
                pi += 1;
            }
            _ => {
                if !failed && !rest_name.is_empty() {
                    let nc = rest_name.chars().next().unwrap();
                    if (c as char) != nc { failed = true; }
                    else { rest_name = &rest_name[nc.len_utf8()..]; }
                } else { failed = true; }
                pi += 1;
            }
        }
    }
    if failed { Ok(None) } else { Ok(Some(rest_name)) }
}

fn get_esc(chunk: &[u8], pi: usize) -> Result<(char, usize), &'static str> {
    if pi >= chunk.len() || chunk[pi] == b']' {
        return Err("syntax error in pattern");
    }
    if chunk[pi] == b'\\' && !cfg!(windows) {
        if pi + 1 >= chunk.len() { return Err("syntax error in pattern"); }
        return Ok((chunk[pi + 1] as char, pi + 2));
    }
    // Parse one UTF-8 char.
    let s = std::str::from_utf8(&chunk[pi..]).unwrap_or("");
    let c = s.chars().next().ok_or("syntax error in pattern")?;
    Ok((c, pi + c.len_utf8()))
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
        assert_eq!(Ext(".hidden"), "");  // leading dot not counted
    }

    #[test]
    fn clean_normalizes() {
        assert_eq!(Clean("a/./b//c/../d"), "a/b/d");
        assert_eq!(Clean("/a/../b"), "/b");
        assert_eq!(Clean(""), ".");
        assert_eq!(Clean("/"), "/");
    }
}
