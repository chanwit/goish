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

fn syntax_err(fn_name: &str, s: &str) -> error {
    New(&format!("strconv.{}: parsing {:?}: invalid syntax", fn_name, s))
}

fn range_err(fn_name: &str, s: &str) -> error {
    New(&format!("strconv.{}: parsing {:?}: value out of range", fn_name, s))
}

pub fn Atoi(s: impl AsRef<str>) -> (int, error) {
    let s = s.as_ref();
    match s.parse::<i64>() {
        Ok(n) => (n, nil),
        Err(_) => (0, syntax_err("Atoi", s)),
    }
}

pub fn Itoa(n: int) -> string {
    n.to_string()
}

/// strconv.ParseInt(s, base, bitSize)
///
///   base = 0      → infer from prefix (0x/0o/0b) else decimal
///   base = 2..36
///   bitSize = 0   → no overflow check (treated as 64)
///   bitSize = 8/16/32/64
pub fn ParseInt(s: impl AsRef<str>, base: int, bit_size: int) -> (int64, error) {
    let s = s.as_ref();
    let (sign, body) = match s.strip_prefix('-') {
        Some(rest) => (-1i64, rest),
        None => (1i64, s.strip_prefix('+').unwrap_or(s)),
    };

    let parsed = if base == 0 {
        if let Some(rest) = body.strip_prefix("0x").or_else(|| body.strip_prefix("0X")) {
            i64::from_str_radix(rest, 16)
        } else if let Some(rest) = body.strip_prefix("0o").or_else(|| body.strip_prefix("0O")) {
            i64::from_str_radix(rest, 8)
        } else if let Some(rest) = body.strip_prefix("0b").or_else(|| body.strip_prefix("0B")) {
            i64::from_str_radix(rest, 2)
        } else {
            body.parse::<i64>()
        }
    } else if (2..=36).contains(&base) {
        i64::from_str_radix(body, base as u32)
    } else {
        return (0, New(&format!("strconv.ParseInt: invalid base {}", base)));
    };

    let n = match parsed {
        Ok(v) => sign.checked_mul(v).unwrap_or(i64::MIN),
        Err(_) => return (0, syntax_err("ParseInt", s)),
    };

    if bit_size > 0 && bit_size < 64 {
        let max = 1i64 << (bit_size - 1);
        if n >= max || n < -max {
            return (n, range_err("ParseInt", s));
        }
    }
    (n, nil)
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

/// Returns a double-quoted Go-syntax string literal.
pub fn Quote(s: impl AsRef<str>) -> string {
    format!("{:?}", s.as_ref())
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
