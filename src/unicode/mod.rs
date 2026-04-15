// unicode: Go's unicode package (rune predicates + case mapping).
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   unicode.IsLetter(r)                 unicode::IsLetter(r)
//   unicode.IsDigit(r)                  unicode::IsDigit(r)
//   unicode.IsSpace(r)                  unicode::IsSpace(r)
//   unicode.IsUpper(r) / IsLower        unicode::IsUpper / IsLower
//   unicode.ToUpper(r) / ToLower        unicode::ToUpper / ToLower
//
// Go's `rune` is `int32`; we accept Rust's `char` for call-site ergonomics.
// The Go-style `rune` alias in `types` gives `i32`; use `char::from_u32` to
// convert when needed.

// Sub-packages.
pub mod utf8;

use crate::types::rune;

fn rune_to_char(r: rune) -> Option<char> {
    char::from_u32(r as u32)
}

#[allow(non_snake_case)]
pub fn IsLetter(r: rune) -> bool {
    rune_to_char(r).map_or(false, |c| c.is_alphabetic())
}

#[allow(non_snake_case)]
pub fn IsDigit(r: rune) -> bool {
    rune_to_char(r).map_or(false, |c| c.is_ascii_digit())
}

/// Go's unicode.IsNumber includes all Unicode number categories (including
/// Roman numerals, etc.). Rust's `char::is_numeric` is close.
#[allow(non_snake_case)]
pub fn IsNumber(r: rune) -> bool {
    rune_to_char(r).map_or(false, |c| c.is_numeric())
}

#[allow(non_snake_case)]
pub fn IsSpace(r: rune) -> bool {
    rune_to_char(r).map_or(false, |c| c.is_whitespace())
}

#[allow(non_snake_case)]
pub fn IsUpper(r: rune) -> bool {
    rune_to_char(r).map_or(false, |c| c.is_uppercase())
}

#[allow(non_snake_case)]
pub fn IsLower(r: rune) -> bool {
    rune_to_char(r).map_or(false, |c| c.is_lowercase())
}

#[allow(non_snake_case)]
pub fn IsPunct(r: rune) -> bool {
    rune_to_char(r).map_or(false, |c| c.is_ascii_punctuation())
}

#[allow(non_snake_case)]
pub fn IsControl(r: rune) -> bool {
    rune_to_char(r).map_or(false, |c| c.is_control())
}

#[allow(non_snake_case)]
pub fn IsPrint(r: rune) -> bool {
    rune_to_char(r).map_or(false, |c| !c.is_control() && !c.is_whitespace() || c == ' ')
}

#[allow(non_snake_case)]
pub fn ToUpper(r: rune) -> rune {
    rune_to_char(r)
        .and_then(|c| c.to_uppercase().next())
        .map(|c| c as rune)
        .unwrap_or(r)
}

#[allow(non_snake_case)]
pub fn ToLower(r: rune) -> rune {
    rune_to_char(r)
        .and_then(|c| c.to_lowercase().next())
        .map(|c| c as rune)
        .unwrap_or(r)
}

/// Go's RuneError — returned by utf8 decoders on invalid input.
#[allow(non_upper_case_globals)]
pub const RuneError: rune = 0xFFFD;

/// Go's MaxRune — largest valid Unicode code point.
#[allow(non_upper_case_globals)]
pub const MaxRune: rune = 0x0010_FFFF;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn predicates_ascii() {
        assert!(IsLetter('a' as rune));
        assert!(IsLetter('漢' as rune));
        assert!(!IsLetter('1' as rune));
        assert!(IsDigit('5' as rune));
        assert!(!IsDigit('a' as rune));
        assert!(IsSpace(' ' as rune));
        assert!(IsSpace('\t' as rune));
        assert!(!IsSpace('x' as rune));
        assert!(IsUpper('A' as rune));
        assert!(!IsUpper('a' as rune));
        assert!(IsLower('a' as rune));
        assert!(IsPunct('!' as rune));
    }

    #[test]
    fn case_mapping() {
        assert_eq!(ToUpper('a' as rune), 'A' as rune);
        assert_eq!(ToLower('A' as rune), 'a' as rune);
        assert_eq!(ToUpper('λ' as rune), 'Λ' as rune);
        assert_eq!(ToUpper('1' as rune), '1' as rune);  // non-letter unchanged
    }
}
