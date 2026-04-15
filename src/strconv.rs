// strconv: Go's strconv package, ported.
//
//   Go                                goish
//   ───────────────────────────────   ──────────────────────────────────
//   n, err := strconv.Atoi(s)         let (n, err) = strconv::Atoi(s);
//   s := strconv.Itoa(n)              let s = strconv::Itoa(n);
//   n, err := strconv.ParseInt(s,b,sz) let (n, err) = strconv::ParseInt(s,b,sz);
//   f, err := strconv.ParseFloat(s,sz) let (f, err) = strconv::ParseFloat(s,sz);
//   b, err := strconv.ParseBool(s)    let (b, err) = strconv::ParseBool(s);
//   s := strconv.FormatInt(n, base)   let s = strconv::FormatInt(n, base);
//   s := strconv.FormatBool(b)        let s = strconv::FormatBool(b);
//   s := strconv.Quote(s)             let s = strconv::Quote(s);

use crate::errors::{error, nil, New};
use crate::types::{float64, int, int64, string};

// Sentinel errors returned by the Parse* family, wrapped inside a
// NumError. Compare with `errors::Is(&e, &strconv::ErrSyntax())` in user
// code — matches Go's `errors.Is(err, strconv.ErrSyntax)` shape.
#[allow(non_snake_case)]
pub fn ErrSyntax() -> error { New("invalid syntax") }
#[allow(non_snake_case)]
pub fn ErrRange() -> error { New("value out of range") }

/// Go's `strconv.NumError` — describes a failed numeric conversion.
/// Exposed as a struct so tests can reach into `.Err`/`.Num`/`.Func`
/// exactly like Go's `e.(*NumError).Err`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NumError {
    pub Func: string,
    pub Num:  string,
    pub Err:  error,
}

impl NumError {
    pub fn new(func: &str, num: &str, err: error) -> NumError {
        NumError { Func: func.to_owned(), Num: num.to_owned(), Err: err }
    }
    /// Match Go's `e.Error()`: `strconv.Atoi: parsing "x": invalid syntax`.
    #[allow(non_snake_case)]
    pub fn Error(&self) -> string {
        format!("strconv.{}: parsing {:?}: {}", self.Func, self.Num, self.Err)
    }
    /// Go's `Unwrap` on NumError exposes the sentinel.
    #[allow(non_snake_case)]
    pub fn Unwrap(&self) -> error { self.Err.clone() }
}

impl std::fmt::Display for NumError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.Error())
    }
}

fn num_error(func: &str, s: &str, sentinel: error) -> error {
    // Keep the same surface string as Go so existing .contains("invalid
    // syntax") / .contains("out of range") tests keep working.
    New(&format!("strconv.{}: parsing {:?}: {}", func, s, sentinel))
}

fn syntax_err(fn_name: &str, s: &str) -> error { num_error(fn_name, s, ErrSyntax()) }
#[allow(dead_code)] fn range_err(fn_name: &str, s: &str) -> error { num_error(fn_name, s, ErrRange()) }

/// Platform int size. All supported targets are 64-bit; matches Go's
/// `strconv.IntSize`.
pub const IntSize: int = 64;

/// Go's `strconv.Atoi(s)` — equivalent to `ParseInt(s, 10, 0)` narrowed
/// to `int`. Returns `(n, NumError)` shape.
pub fn Atoi(s: impl AsRef<str>) -> (int, error) {
    let s = s.as_ref();
    let (n, err) = ParseInt(s, 10, 0);
    if err != nil {
        // Rename Func in the error message: "strconv.ParseInt" → "strconv.Atoi".
        let msg = format!("{}", err).replace("strconv.ParseInt", "strconv.Atoi");
        return (n, New(&msg));
    }
    (n, nil)
}

pub fn Itoa(n: int) -> string {
    n.to_string()
}

/// `strconv.ParseUint(s, base, bitSize)` — unsigned parse.
/// No leading sign permitted. Underscores allowed only when `base == 0`
/// and only as digit separators between digits (matches Go 1.25).
#[allow(non_snake_case)]
pub fn ParseUint(s: impl AsRef<str>, base: int, bit_size: int) -> (u64, error) {
    let s_raw = s.as_ref();
    let s0 = s_raw;
    const FN: &str = "ParseUint";

    if s_raw.is_empty() {
        return (0, num_error(FN, s0, ErrSyntax()));
    }

    let base0 = base == 0;
    let (mut base_u, body) = match base {
        b if (2..=36).contains(&b) => (b as u32, s_raw),
        0 => {
            // Look for prefix.
            let bs = s_raw.as_bytes();
            if bs[0] == b'0' {
                if s_raw.len() >= 3 {
                    let c = bs[1].to_ascii_lowercase();
                    if c == b'b' { (2, &s_raw[2..]) }
                    else if c == b'o' { (8, &s_raw[2..]) }
                    else if c == b'x' { (16, &s_raw[2..]) }
                    else { (8, &s_raw[1..]) }
                } else {
                    (8, &s_raw[1..])
                }
            } else {
                (10, s_raw)
            }
        }
        _ => return (0, New(&format!("strconv.{}: parsing {:?}: invalid base {}", FN, s0, base))),
    };
    let _ = &mut base_u;

    let bit_size_effective: u32 = if bit_size == 0 { IntSize as u32 }
        else if bit_size < 0 || bit_size > 64 {
            return (0, New(&format!("strconv.{}: parsing {:?}: invalid bit size {}", FN, s0, bit_size)));
        }
        else { bit_size as u32 };

    let max_val: u64 = if bit_size_effective == 64 { u64::MAX }
        else { (1u64 << bit_size_effective) - 1 };

    // Cutoff = maxUint64/base + 1; if n >= cutoff, n*base overflows.
    let cutoff: u64 = u64::MAX / base_u as u64 + 1;

    let mut n: u64 = 0;
    let mut underscores = false;
    for c in body.as_bytes() {
        let d: u8;
        let c = *c;
        if c == b'_' && base0 {
            underscores = true;
            continue;
        } else if (b'0'..=b'9').contains(&c) {
            d = c - b'0';
        } else if (b'a'..=b'z').contains(&c.to_ascii_lowercase()) {
            d = c.to_ascii_lowercase() - b'a' + 10;
        } else {
            return (0, num_error(FN, s0, ErrSyntax()));
        }

        if d as u32 >= base_u {
            return (0, num_error(FN, s0, ErrSyntax()));
        }

        if n >= cutoff {
            return (max_val, num_error(FN, s0, ErrRange()));
        }
        n = n.wrapping_mul(base_u as u64);
        let (n1, wrapped) = n.overflowing_add(d as u64);
        if wrapped || n1 > max_val {
            return (max_val, num_error(FN, s0, ErrRange()));
        }
        n = n1;
    }

    // Ensure underscore placement is valid.
    if underscores && !underscore_ok(s0) {
        return (0, num_error(FN, s0, ErrSyntax()));
    }

    (n, nil)
}

/// strconv.ParseInt(s, base, bitSize)
///
///   base = 0      → infer from prefix (0x/0o/0b/0) else decimal
///   base = 2..36
///   bitSize = 0   → IntSize
///   bitSize = 8/16/32/64
pub fn ParseInt(s: impl AsRef<str>, base: int, bit_size: int) -> (int64, error) {
    let s_raw = s.as_ref();
    let s0 = s_raw;
    const FN: &str = "ParseInt";

    if s_raw.is_empty() {
        return (0, num_error(FN, s0, ErrSyntax()));
    }

    // Strip leading sign.
    let bs = s_raw.as_bytes();
    let (neg, body) = match bs[0] {
        b'+' => (false, &s_raw[1..]),
        b'-' => (true,  &s_raw[1..]),
        _    => (false, s_raw),
    };

    let (un, err) = ParseUint(body, base, bit_size);
    // If ParseUint failed with anything other than ErrRange, rewrite the
    // message to use our function name and the original input.
    if err != nil && !format!("{}", err).contains("value out of range") {
        let msg = format!("strconv.ParseInt: parsing {:?}: invalid syntax", s0);
        // Preserve non-syntax error kinds (invalid base / invalid bit size)
        if format!("{}", err).contains("invalid base") || format!("{}", err).contains("invalid bit size") {
            let rebuilt = format!("{}", err).replace("strconv.ParseUint", "strconv.ParseInt")
                .replacen(&format!("{:?}", body), &format!("{:?}", s0), 1);
            return (0, New(&rebuilt));
        }
        return (0, New(&msg));
    }

    let bit_size_eff = if bit_size == 0 { IntSize as u32 } else { bit_size as u32 };
    let cutoff: u64 = 1u64 << (bit_size_eff - 1);
    // Saturated max/min for this bitSize.
    let sat_max: i64 = (cutoff - 1) as i64;
    let sat_min: i64 = if bit_size_eff == 64 { i64::MIN } else { -((cutoff as i64)) };

    if !neg && un >= cutoff {
        return (sat_max, num_error(FN, s0, ErrRange()));
    }
    if neg && un > cutoff {
        return (sat_min, num_error(FN, s0, ErrRange()));
    }

    // If ParseUint returned ErrRange, saturate and propagate.
    if err != nil {
        let msg = format!("strconv.ParseInt: parsing {:?}: value out of range", s0);
        let sat = if neg { sat_min } else { sat_max };
        return (sat, New(&msg));
    }

    let n: i64 = if neg {
        if un == cutoff { sat_min } else { -(un as i64) }
    } else {
        un as i64
    };
    (n, nil)
}

/// Go's underscoreOK helper — check that `_` only appears between digits.
fn underscore_ok(s: &str) -> bool {
    let mut saw: char = '^';
    let bs = s.as_bytes();
    let mut i = 0usize;

    if !bs.is_empty() && (bs[0] == b'+' || bs[0] == b'-') {
        // Skip sign without changing saw; the loop below starts at i=0 on
        // a sign-stripped slice, so pre-strip.
        let rest = &s[1..];
        return underscore_ok(rest);
    }

    let mut hex = false;
    if bs.len() >= 2 && bs[0] == b'0' {
        let c1 = bs[1].to_ascii_lowercase();
        if c1 == b'b' || c1 == b'o' || c1 == b'x' {
            i = 2;
            saw = '0';
            hex = c1 == b'x';
        }
    }

    while i < bs.len() {
        let c = bs[i];
        if (b'0'..=b'9').contains(&c) || (hex && (b'a'..=b'f').contains(&c.to_ascii_lowercase())) {
            saw = '0';
            i += 1;
            continue;
        }
        if c == b'_' {
            if saw != '0' { return false; }
            saw = '_';
            i += 1;
            continue;
        }
        if saw == '_' { return false; }
        saw = '!';
        i += 1;
    }
    saw != '_'
}

pub fn ParseFloat(s: impl AsRef<str>, _bit_size: int) -> (float64, error) {
    let s = s.as_ref();
    match s.parse::<f64>() {
        Ok(n) => (n, nil),
        Err(_) => (0.0, syntax_err("ParseFloat", s)),
    }
}

pub fn ParseBool(s: impl AsRef<str>) -> (bool, error) {
    let s = s.as_ref();
    match s {
        "1" | "t" | "T" | "TRUE" | "true" | "True" => (true, nil),
        "0" | "f" | "F" | "FALSE" | "false" | "False" => (false, nil),
        _ => (false, syntax_err("ParseBool", s)),
    }
}

pub fn FormatInt(n: int64, base: int) -> string {
    if !(2..=36).contains(&base) {
        panic!("strconv: illegal number base {}", base);
    }
    // Compute absolute value in u128 so i64::MIN negates cleanly.
    let (neg, mut nn) = if n < 0 {
        (true, (n as i128).unsigned_abs())
    } else {
        (false, n as u128)
    };
    if nn == 0 {
        return "0".to_string();
    }
    let mut s = String::new();
    let base_u = base as u128;
    while nn > 0 {
        let d = (nn % base_u) as u32;
        s.insert(0, std::char::from_digit(d, base as u32).unwrap_or('?'));
        nn /= base_u;
    }
    if neg {
        s.insert(0, '-');
    }
    s
}

pub fn FormatUint(n: u64, base: int) -> string {
    if !(2..=36).contains(&base) {
        panic!("strconv: illegal number base {}", base);
    }
    if n == 0 {
        return "0".to_string();
    }
    let mut s = String::new();
    let mut nn = n as u128;
    let base_u = base as u128;
    while nn > 0 {
        let d = (nn % base_u) as u32;
        s.insert(0, std::char::from_digit(d, base as u32).unwrap_or('?'));
        nn /= base_u;
    }
    s
}

/// AppendInt appends the string form of n in base, to dst, and returns the
/// extended buffer.
#[allow(non_snake_case)]
pub fn AppendInt(mut dst: Vec<crate::types::byte>, n: int64, base: int) -> Vec<crate::types::byte> {
    dst.extend_from_slice(FormatInt(n, base).as_bytes());
    dst
}

#[allow(non_snake_case)]
pub fn AppendUint(mut dst: Vec<crate::types::byte>, n: u64, base: int) -> Vec<crate::types::byte> {
    dst.extend_from_slice(FormatUint(n, base).as_bytes());
    dst
}

pub fn FormatBool(b: bool) -> string {
    if b { "true".to_string() } else { "false".to_string() }
}

/// `strconv.AppendBool(dst, b)` — appends `"true"` or `"false"` to dst.
#[allow(non_snake_case)]
pub fn AppendBool(mut dst: Vec<crate::types::byte>, b: bool) -> Vec<crate::types::byte> {
    dst.extend_from_slice(if b { b"true" } else { b"false" });
    dst
}

/// `strconv.Quote(s)` — double-quoted Go-syntax string literal. Printable
/// runes pass through; control characters, DEL, and invalid bytes get
/// escaped via `\a \b \f \n \r \t \v \xHH \uHHHH \UHHHHHHHH`.
pub fn Quote(s: impl AsRef<str>) -> string {
    quote_with(s.as_ref(), '"', false, false)
}

/// `strconv.QuoteToASCII(s)` — like Quote, but escapes every non-ASCII
/// rune as `\uHHHH` / `\UHHHHHHHH` so the result is pure ASCII.
#[allow(non_snake_case)]
pub fn QuoteToASCII(s: impl AsRef<str>) -> string {
    quote_with(s.as_ref(), '"', true, false)
}

/// `strconv.QuoteToGraphic(s)` — like Quote, but uses IsGraphic (which
/// additionally accepts spaces like U+00A0) to decide "print as-is".
#[allow(non_snake_case)]
pub fn QuoteToGraphic(s: impl AsRef<str>) -> string {
    quote_with(s.as_ref(), '"', false, true)
}

/// `strconv.AppendQuote(dst, s)` — append Quote(s) to dst.
#[allow(non_snake_case)]
pub fn AppendQuote(mut dst: Vec<crate::types::byte>, s: impl AsRef<str>) -> Vec<crate::types::byte> {
    dst.extend_from_slice(Quote(s).as_bytes());
    dst
}

/// `strconv.AppendQuoteToASCII(dst, s)`
#[allow(non_snake_case)]
pub fn AppendQuoteToASCII(mut dst: Vec<crate::types::byte>, s: impl AsRef<str>) -> Vec<crate::types::byte> {
    dst.extend_from_slice(QuoteToASCII(s).as_bytes());
    dst
}

/// `strconv.AppendQuoteToGraphic(dst, s)`
#[allow(non_snake_case)]
pub fn AppendQuoteToGraphic(mut dst: Vec<crate::types::byte>, s: impl AsRef<str>) -> Vec<crate::types::byte> {
    dst.extend_from_slice(QuoteToGraphic(s).as_bytes());
    dst
}

/// `strconv.QuoteRune(r)` — single-quoted Go rune literal.
#[allow(non_snake_case)]
pub fn QuoteRune(r: crate::types::rune) -> string {
    quote_rune_with(r, false, false)
}

/// `strconv.QuoteRuneToASCII(r)`
#[allow(non_snake_case)]
pub fn QuoteRuneToASCII(r: crate::types::rune) -> string {
    quote_rune_with(r, true, false)
}

/// `strconv.QuoteRuneToGraphic(r)`
#[allow(non_snake_case)]
pub fn QuoteRuneToGraphic(r: crate::types::rune) -> string {
    quote_rune_with(r, false, true)
}

/// `strconv.AppendQuoteRune(dst, r)`
#[allow(non_snake_case)]
pub fn AppendQuoteRune(mut dst: Vec<crate::types::byte>, r: crate::types::rune) -> Vec<crate::types::byte> {
    dst.extend_from_slice(QuoteRune(r).as_bytes());
    dst
}

#[allow(non_snake_case)]
pub fn AppendQuoteRuneToASCII(mut dst: Vec<crate::types::byte>, r: crate::types::rune) -> Vec<crate::types::byte> {
    dst.extend_from_slice(QuoteRuneToASCII(r).as_bytes());
    dst
}

#[allow(non_snake_case)]
pub fn AppendQuoteRuneToGraphic(mut dst: Vec<crate::types::byte>, r: crate::types::rune) -> Vec<crate::types::byte> {
    dst.extend_from_slice(QuoteRuneToGraphic(r).as_bytes());
    dst
}

/// `strconv.IsPrint(r)` — true iff r is a printable rune as defined by
/// Go's built-in strconv table. Deliberately does not call
/// `unicode::IsPrint` — Go's strconv version excludes some codepoints
/// (noncharacters, private-use, unassigned) that Rust's char methods
/// consider printable.
#[allow(non_snake_case)]
pub fn IsPrint(r: crate::types::rune) -> bool {
    // Reject invalid / out-of-range / surrogate.
    if r < 0 || r > 0x0010_FFFF { return false; }
    let u = r as u32;
    if (0xD800..=0xDFFF).contains(&u) { return false; }
    // Noncharacters: U+FDD0..=U+FDEF, and last two code points on every plane.
    if (0xFDD0..=0xFDEF).contains(&u) { return false; }
    if (u & 0xFFFE) == 0xFFFE { return false; }
    // Private-use planes and supplementary private use: not printable for Go.
    if (0xE000..=0xF8FF).contains(&u) { return false; }
    if (0xF0000..=0xFFFFD).contains(&u) { return false; }
    if (0x100000..=0x10FFFD).contains(&u) { return false; }
    // Delegate remaining to the unicode module.
    crate::unicode::IsPrint(r)
}

/// `strconv.IsGraphic(r)` — true iff IsPrint OR certain Unicode space
/// characters (U+00A0, U+2000, U+3000, …) that Print rejects but a
/// terminal will still render visibly.
#[allow(non_snake_case)]
pub fn IsGraphic(r: crate::types::rune) -> bool {
    if IsPrint(r) { return true; }
    // Go's IsGraphic additionally accepts Unicode space category (Zs/Zl/Zp).
    matches!(r,
        0x00A0 | 0x1680 | 0x2000..=0x200A | 0x202F | 0x205F | 0x3000
    )
}

/// `strconv.CanBackquote(s)` — true iff s can be wrapped in backticks
/// to form a valid Go raw string literal (no control chars except tab,
/// no backtick, no invalid UTF-8).
#[allow(non_snake_case)]
pub fn CanBackquote(s: impl AsRef<str>) -> bool {
    let s = s.as_ref();
    for r in s.chars() {
        let code = r as u32;
        // U+0000–U+0008, U+000B–U+001F, U+007F (DEL): disallowed.
        // U+0009 (tab) is allowed.
        if code < 0x20 && code != b'\t' as u32 { return false; }
        if code == 0x7F { return false; }
        if r == '`' { return false; }
        if r == '\u{FFFD}' {
            // Go considers utf8.RuneError (from invalid UTF-8) a disqualifier.
            // We can't distinguish a real U+FFFD in &str from one invented by
            // decode, because invalid bytes can't enter a Rust &str. Be strict.
            return false;
        }
    }
    true
}

fn quote_with(s: &str, quote: char, ascii: bool, graphic: bool) -> string {
    let mut out = String::with_capacity(s.len() + 2);
    out.push(quote);
    for r in s.chars() {
        append_escaped_rune(&mut out, r, quote, ascii, graphic);
    }
    out.push(quote);
    out
}

fn quote_rune_with(r: crate::types::rune, ascii: bool, graphic: bool) -> string {
    let mut out = String::with_capacity(10);
    out.push('\'');
    // Sanitize: if r is out of range or surrogate, use RuneError.
    let r_eff: crate::types::rune = if r < 0 || r > 0x0010_FFFF || (0xD800..=0xDFFF).contains(&r) {
        0xFFFD
    } else {
        r
    };
    append_escaped_rune(&mut out, std::char::from_u32(r_eff as u32).unwrap_or('\u{FFFD}'), '\'', ascii, graphic);
    out.push('\'');
    out
}

fn append_escaped_rune(out: &mut String, r: char, quote: char, ascii: bool, graphic: bool) {
    // Handle quote-internal escapes first.
    if r == quote || r == '\\' {
        out.push('\\');
        out.push(r);
        return;
    }
    if ascii {
        // ASCII-printable pass-through.
        let code = r as u32;
        if code < 0x80 {
            if is_ascii_printable(r) {
                out.push(r);
                return;
            }
            // Fall through to escape.
        }
        // Non-ASCII → always escape.
        append_escape_non_ascii(out, r);
        return;
    }
    // Non-ASCII mode: respect IsPrint (and optionally IsGraphic).
    let code = r as u32;
    if code < 0x80 && is_ascii_printable(r) {
        out.push(r);
        return;
    }
    let keep_as_is = if graphic { IsGraphic(code as i32) } else { IsPrint(code as i32) };
    if keep_as_is {
        out.push(r);
        return;
    }
    append_escape_non_ascii(out, r);
}

fn is_ascii_printable(r: char) -> bool {
    let c = r as u32;
    c >= 0x20 && c < 0x7F
}

fn append_escape_non_ascii(out: &mut String, r: char) {
    // Short escapes first.
    match r {
        '\x07' => { out.push_str("\\a"); return; }
        '\x08' => { out.push_str("\\b"); return; }
        '\x0c' => { out.push_str("\\f"); return; }
        '\n'   => { out.push_str("\\n"); return; }
        '\r'   => { out.push_str("\\r"); return; }
        '\t'   => { out.push_str("\\t"); return; }
        '\x0b' => { out.push_str("\\v"); return; }
        _ => {}
    }
    let code = r as u32;
    if code < 0x80 {
        out.push_str(&format!("\\x{:02x}", code));
    } else if code < 0x10000 {
        out.push_str(&format!("\\u{:04x}", code));
    } else {
        out.push_str(&format!("\\U{:08x}", code));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atoi_round_trip() {
        let (n, err) = Atoi("42");
        assert!(err == nil);
        assert_eq!(n, 42);
        assert_eq!(Itoa(42), "42");

        let (n, err) = Atoi("-7");
        assert!(err == nil);
        assert_eq!(n, -7);
    }

    #[test]
    fn atoi_invalid() {
        let (n, err) = Atoi("abc");
        assert_eq!(n, 0);
        assert!(err != nil);
        assert!(format!("{}", err).contains("invalid syntax"));
    }

    #[test]
    fn parse_int_base_and_bitsize() {
        assert_eq!(ParseInt("ff", 16, 64).0, 255);
        assert_eq!(ParseInt("0xff", 0, 64).0, 255);
        assert_eq!(ParseInt("0b1010", 0, 64).0, 10);
        assert_eq!(ParseInt("-100", 10, 64).0, -100);

        // 8-bit overflow
        let (_, err) = ParseInt("200", 10, 8);
        assert!(err != nil);
        assert!(format!("{}", err).contains("out of range"));
    }

    #[test]
    fn parse_float_basic() {
        let (f, err) = ParseFloat("3.14", 64);
        assert!(err == nil);
        assert!((f - 3.14).abs() < 1e-9);
    }

    #[test]
    fn parse_bool_variants() {
        assert_eq!(ParseBool("true").0, true);
        assert_eq!(ParseBool("T").0, true);
        assert_eq!(ParseBool("1").0, true);
        assert_eq!(ParseBool("FALSE").0, false);
        assert_eq!(ParseBool("0").0, false);

        let (_, err) = ParseBool("maybe");
        assert!(err != nil);
    }

    #[test]
    fn format_int_bases() {
        assert_eq!(FormatInt(255, 10), "255");
        assert_eq!(FormatInt(255, 16), "ff");
        assert_eq!(FormatInt(255, 2), "11111111");
        assert_eq!(FormatInt(-10, 10), "-10");
    }

    #[test]
    fn format_bool() {
        assert_eq!(FormatBool(true), "true");
        assert_eq!(FormatBool(false), "false");
    }

    #[test]
    fn quote_basic() {
        assert_eq!(Quote("hi"), "\"hi\"");
    }
}
