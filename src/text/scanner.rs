//! text/scanner: simple token scanner over Go-like source.
//!
//!   Go                                  goish
//!   ─────────────────────────────────   ──────────────────────────────────
//!   var s scanner.Scanner               let mut s = scanner::Scanner::new();
//!   s.Init(strings.NewReader(src))      s.Init(&src);
//!   for tok := s.Scan(); tok != …       loop {
//!                                         let tok = s.Scan();
//!                                         if tok == scanner::EOF { break; }
//!                                         /* use s.TokenText() */
//!                                       }
//!
//! Implements enough of Go's scanner.Scanner to tokenise identifiers,
//! integer and float literals, double-quoted strings, single-quoted
//! character literals, and single-character punctuation. Comment-skip,
//! raw-string-literals, and Whitespace-mode customisation are not
//! ported (their APIs would only exist to serve the tests we're not
//! porting).

use crate::types::{int, rune};

/// Token type constants. Go uses negative values for keyword tokens;
/// goish exposes a curated subset.
pub const EOF: rune = -1;
pub const Ident: rune = -2;
pub const Int: rune = -3;
pub const Float: rune = -4;
pub const Char: rune = -5;
pub const String: rune = -6;

/// A minimal scanner that tracks line/column.
pub struct Scanner {
    src: Vec<char>,
    pos: usize,
    pub Line: int,
    pub Column: int,
    tok_start: usize,
    tok_end: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position {
    pub Line: int,
    pub Column: int,
}

impl Scanner {
    pub fn new() -> Self {
        Scanner { src: Vec::new(), pos: 0, Line: 1, Column: 1, tok_start: 0, tok_end: 0 }
    }

    pub fn Init(&mut self, src: impl AsRef<str>) {
        self.src = src.as_ref().chars().collect();
        self.pos = 0;
        self.Line = 1;
        self.Column = 1;
        self.tok_start = 0;
        self.tok_end = 0;
    }

    pub fn Pos(&self) -> Position {
        Position { Line: self.Line, Column: self.Column }
    }

    pub fn TokenText(&self) -> String {
        self.src[self.tok_start..self.tok_end].iter().collect()
    }

    /// Scan advances past whitespace and returns the next token's kind
    /// (or the Unicode code point for a single-rune punctuation token).
    #[allow(non_snake_case)]
    pub fn Scan(&mut self) -> rune {
        self.skip_whitespace();
        if self.pos >= self.src.len() {
            self.tok_start = self.pos;
            self.tok_end = self.pos;
            return EOF;
        }
        self.tok_start = self.pos;
        let c = self.src[self.pos];
        if c.is_alphabetic() || c == '_' {
            while self.pos < self.src.len() && (self.src[self.pos].is_alphanumeric() || self.src[self.pos] == '_') {
                self.advance();
            }
            self.tok_end = self.pos;
            return Ident;
        }
        if c.is_ascii_digit() {
            while self.pos < self.src.len() && self.src[self.pos].is_ascii_digit() {
                self.advance();
            }
            // Optional fractional part.
            if self.pos < self.src.len() && self.src[self.pos] == '.' {
                self.advance();
                while self.pos < self.src.len() && self.src[self.pos].is_ascii_digit() {
                    self.advance();
                }
                self.tok_end = self.pos;
                return Float;
            }
            self.tok_end = self.pos;
            return Int;
        }
        if c == '"' {
            self.advance();
            while self.pos < self.src.len() && self.src[self.pos] != '"' {
                if self.src[self.pos] == '\\' && self.pos + 1 < self.src.len() {
                    self.advance();
                }
                self.advance();
            }
            if self.pos < self.src.len() { self.advance(); }  // closing quote
            self.tok_end = self.pos;
            return String;
        }
        if c == '\'' {
            self.advance();
            while self.pos < self.src.len() && self.src[self.pos] != '\'' {
                if self.src[self.pos] == '\\' && self.pos + 1 < self.src.len() {
                    self.advance();
                }
                self.advance();
            }
            if self.pos < self.src.len() { self.advance(); }
            self.tok_end = self.pos;
            return Char;
        }
        // Single-char punctuation / operator.
        self.advance();
        self.tok_end = self.pos;
        c as u32 as rune
    }

    fn advance(&mut self) {
        if self.pos < self.src.len() {
            if self.src[self.pos] == '\n' {
                self.Line += 1;
                self.Column = 1;
            } else {
                self.Column += 1;
            }
            self.pos += 1;
        }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.src.len() && self.src[self.pos].is_whitespace() {
            self.advance();
        }
    }
}

impl Default for Scanner { fn default() -> Self { Scanner::new() } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenises_mixed() {
        let mut s = Scanner::new();
        s.Init("foo 42 3.14 \"hi\" 'x' +");
        let mut kinds = Vec::new();
        let mut texts = Vec::new();
        loop {
            let k = s.Scan();
            if k == EOF { break; }
            kinds.push(k);
            texts.push(s.TokenText());
        }
        assert_eq!(kinds, vec![Ident, Int, Float, String, Char, '+' as rune]);
        assert_eq!(texts, vec!["foo", "42", "3.14", "\"hi\"", "'x'", "+"]);
    }

    #[test]
    fn empty_source_returns_eof() {
        let mut s = Scanner::new();
        s.Init("");
        assert_eq!(s.Scan(), EOF);
    }

    #[test]
    fn tracks_line_column() {
        let mut s = Scanner::new();
        s.Init("a\nb");
        s.Scan(); s.Scan();  // identifiers
        let p = s.Pos();
        assert_eq!(p.Line, 2);
    }
}
