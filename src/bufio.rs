// bufio: Go's bufio package — line-oriented reading.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   sc := bufio.NewScanner(os.Stdin)    let mut sc = bufio::NewScanner(r);
//   for sc.Scan() {                     while sc.Scan() {
//       line := sc.Text()                   let line = sc.Text();
//   }                                   }
//   if err := sc.Err(); err != nil {    if sc.Err() != nil { … }
//
// Wraps any `std::io::BufRead` (including `std::io::stdin().lock()`).
// Split function: lines (strips trailing \n and \r\n).

use crate::errors::{error, nil, New};
use crate::types::{byte, string};
use std::io::BufRead;

pub struct Scanner<R: BufRead> {
    reader: R,
    buf: String,
    last_err: error,
    done: bool,
}

#[allow(non_snake_case)]
pub fn NewScanner<R: BufRead>(r: R) -> Scanner<R> {
    Scanner {
        reader: r,
        buf: String::new(),
        last_err: nil,
        done: false,
    }
}

impl<R: BufRead> Scanner<R> {
    /// sc.Scan() — returns true when a new line is available, false at EOF.
    /// After returning false, call Err() to check for a non-EOF error.
    pub fn Scan(&mut self) -> bool {
        if self.done {
            return false;
        }
        self.buf.clear();
        match self.reader.read_line(&mut self.buf) {
            Ok(0) => {
                self.done = true;
                false
            }
            Ok(_) => {
                // Strip trailing \n and \r\n (Go behavior).
                if self.buf.ends_with('\n') {
                    self.buf.pop();
                    if self.buf.ends_with('\r') {
                        self.buf.pop();
                    }
                }
                true
            }
            Err(e) => {
                self.last_err = New(&format!("bufio.Scanner: {}", e));
                self.done = true;
                false
            }
        }
    }

    /// sc.Text() — the current line, as a string slice.
    pub fn Text(&self) -> &str {
        &self.buf
    }

    /// sc.Bytes() — the current line, as a byte slice.
    pub fn Bytes(&self) -> &[byte] {
        self.buf.as_bytes()
    }

    /// sc.Err() — non-EOF error encountered, or nil.
    pub fn Err(&self) -> &error {
        &self.last_err
    }
}

/// Convenience: read all lines from a reader into a slice<string>.
#[allow(non_snake_case)]
pub fn ReadLines<R: BufRead>(r: R) -> (crate::types::slice<string>, error) {
    let mut sc = NewScanner(r);
    let mut lines = crate::types::slice::<string>::new();
    while sc.Scan() {
        lines.push(sc.Text().to_string());
    }
    let err = sc.Err().clone();
    (lines, err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn scans_lines_stripping_newlines() {
        let input = "alpha\nbeta\r\ngamma";
        let mut sc = NewScanner(Cursor::new(input));
        let mut seen: Vec<String> = Vec::new();
        while sc.Scan() {
            seen.push(sc.Text().to_string());
        }
        assert_eq!(seen, vec!["alpha", "beta", "gamma"]);
        assert!(sc.Err() == &nil);
    }

    #[test]
    fn empty_reader_scan_returns_false() {
        let mut sc = NewScanner(Cursor::new(""));
        assert!(!sc.Scan());
        assert!(sc.Err() == &nil);
    }

    #[test]
    fn read_lines_convenience() {
        let (lines, err) = ReadLines(Cursor::new("one\ntwo\nthree\n"));
        assert!(err == nil);
        assert_eq!(lines, vec!["one", "two", "three"]);
    }
}
