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
