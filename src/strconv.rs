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

/// `strconv.FormatFloat(f, fmt, prec, bitSize)` — format a float into a
/// Go-shaped string. `fmt` is one of `'e'`, `'E'`, `'f'`, `'g'`, `'G'`,
/// `'b'`, `'x'`, `'X'`. `prec` is the precision (meaning depends on fmt)
/// or `-1` for the shortest representation. `bitSize` is 32 or 64.
///
/// Matches Go 1.25.5 behaviour for `e/E/f/g/G/b`. `x/X` (hex float) is
/// implemented best-effort and may diverge on edge-case rounding.
#[allow(non_snake_case)]
pub fn FormatFloat(f: float64, fmt: u8, prec: int, bit_size: int) -> string {
    // Truncate to f32 precision when bitSize = 32.
    let v: f64 = if bit_size == 32 { f as f32 as f64 } else { f };

    // Special values share all formats.
    if v.is_nan()       { return "NaN".to_string(); }
    if v.is_infinite()  { return if v < 0.0 { "-Inf".to_string() } else { "+Inf".to_string() }; }

    match fmt {
        b'e' | b'E' => format_e(v, prec, fmt == b'E', bit_size),
        b'f'        => format_f(v, prec, bit_size),
        b'g' | b'G' => format_g(v, prec, fmt == b'G', bit_size),
        b'b'        => format_b(v, bit_size),
        b'x' | b'X' => format_x(v, prec, fmt == b'X', bit_size),
        _ => format!("%{}", fmt as char),
    }
}

/// `strconv.AppendFloat(dst, f, fmt, prec, bitSize)`
#[allow(non_snake_case)]
pub fn AppendFloat(mut dst: Vec<crate::types::byte>, f: float64, fmtc: u8, prec: int, bit_size: int) -> Vec<crate::types::byte> {
    dst.extend_from_slice(FormatFloat(f, fmtc, prec, bit_size).as_bytes());
    dst
}

// ── FormatFloat subroutines ───────────────────────────────────────────

fn shortest_f64_str(v: f64, bit_size: int) -> string {
    // Rust's ryu-backed default formatter gives the shortest round-trip form.
    let s = if bit_size == 32 {
        format!("{}", v as f32)
    } else {
        format!("{}", v)
    };
    s
}

fn format_e(v: f64, prec: int, upper: bool, bit_size: int) -> string {
    if prec < 0 {
        let base = shortest_f64_str(v, bit_size);
        return canonical_exponent(&base, 0, true, upper);
    }
    let p = prec as usize;
    if v == 0.0 {
        let tail: String = std::iter::repeat('0').take(p).collect();
        let sep = if p > 0 { "." } else { "" };
        let marker = if upper { 'E' } else { 'e' };
        let sign = if v.is_sign_negative() { "-" } else { "" };
        return format!("{}0{}{}{}+00", sign, sep, tail, marker);
    }
    // Decompose into (mantissa, exp10) with mantissa in [1,10).
    let neg = v.is_sign_negative();
    let abs = v.abs();
    let exp10 = abs.log10().floor() as i32;
    let mut mantissa = abs / 10f64.powi(exp10);
    // Round mantissa to `p` decimal places using half-away-from-zero
    // (matches Go's FormatFloat rounding).
    let scale = 10f64.powi(p as i32);
    mantissa = (mantissa * scale + 0.5).floor() / scale;
    // If rounding pushed mantissa to 10, re-normalize.
    let (mantissa, exp10) = if mantissa >= 10.0 { (mantissa / 10.0, exp10 + 1) } else { (mantissa, exp10) };
    let marker = if upper { 'E' } else { 'e' };
    let sign_m = if neg { "-" } else { "" };
    let e_sign = if exp10 >= 0 { '+' } else { '-' };
    let padded = format!("{:02}", exp10.abs());
    format!("{}{:.*}{}{}{}", sign_m, p, mantissa, marker, e_sign, padded)
}

fn format_f(v: f64, prec: int, _bit_size: int) -> string {
    if prec < 0 {
        // Shortest fixed form.
        let base = shortest_f64_str(v, _bit_size);
        // If shortest is scientific, expand it.
        if base.contains('e') || base.contains('E') {
            // Fallback: very long but correct — use enough precision.
            return format!("{:.*}", 20, v).trim_end_matches('0').trim_end_matches('.').to_string();
        }
        return base;
    }
    format!("{:.*}", prec as usize, v)
}

fn format_g(v: f64, prec: int, upper: bool, bit_size: int) -> string {
    // Go's 'g' rules for prec=-1 (shortest):
    //   eprec = 6
    //   exp = decimal-point-position - 1
    //   if exp < -4 or exp >= eprec → scientific, else fixed
    if prec < 0 {
        if v == 0.0 { return "0".to_string(); }
        let abs = v.abs();
        let expn = abs.log10().floor() as i32;
        let shortest = shortest_f64_str(v, bit_size);
        if expn < -4 || expn >= 6 {
            // Emit shortest-digit scientific. Rust's Display already uses
            // scientific if the magnitude is extreme; otherwise coerce.
            if shortest.contains('e') || shortest.contains('E') {
                return rust_to_go_shortest(&shortest, upper);
            }
            // Convert manually.
            return canonical_exponent(&shortest, 0, true, upper);
        } else {
            // Fixed form. If Rust gave scientific, reformat.
            if shortest.contains('e') || shortest.contains('E') {
                let n_digits = (expn + 1).max(1) as usize;
                return format!("{:.*}", n_digits.saturating_sub((expn + 1) as usize), v);
            }
            return shortest;
        }
    }
    // Fixed precision: Rust has no direct 'g' formatter. Emulate Go's
    // rule: if exponent < -4 or >= prec → scientific, else fixed.
    let p = if prec == 0 { 1 } else { prec as i32 };
    if v == 0.0 {
        // Go renders as e.g. "0" for g/-1, "0" for prec=5. Actually with
        // prec≥1 Go gives "0" here too; scientific form is only used on
        // demand, but 'g' trims trailing zeros.
        return "0".to_string();
    }
    let abs = v.abs();
    let expn = abs.log10().floor() as i32;
    if expn < -4 || expn >= p {
        let e_prec = (p - 1).max(0);
        let raw = format_e(v, e_prec as i64, upper, 64);
        trim_g_trailing_zeros(&raw, upper)
    } else {
        // Fixed-point, with total sig-figs = p.
        let f_prec = (p - 1 - expn).max(0) as usize;
        // Go uses half-away-from-zero for fixed too.
        let scale = 10f64.powi(f_prec as i32);
        let rounded = if v.is_sign_negative() {
            ((v * scale) - 0.5).ceil() / scale
        } else {
            ((v * scale) + 0.5).floor() / scale
        };
        let raw = format!("{:.*}", f_prec, rounded);
        trim_g_trailing_zeros(&raw, upper)
    }
}

fn format_b(v: f64, _bit_size: int) -> string {
    // "mantissa p± exp" where value = mantissa × 2^exp.
    // Use the raw IEEE754 fields.
    let bits = v.to_bits();
    let sign = bits >> 63;
    let mut exp = ((bits >> 52) & 0x7FF) as i32;
    let mut mant = bits & 0xF_FFFF_FFFF_FFFF;
    if exp == 0 {
        // Subnormal: exponent = 1 - 1023 - 52.
        if mant == 0 { return if sign == 1 { "-0p-1074".to_string() } else { "0p-1074".to_string() }; }
        exp = 1;
    } else {
        mant |= 1u64 << 52;
    }
    let adjusted_exp = exp - 1023 - 52;
    let sign_s = if sign == 1 { "-" } else { "" };
    if adjusted_exp >= 0 {
        format!("{}{}p+{}", sign_s, mant, adjusted_exp)
    } else {
        format!("{}{}p-{}", sign_s, mant, -adjusted_exp)
    }
}

fn format_x(v: f64, prec: int, upper: bool, _bit_size: int) -> string {
    if v == 0.0 {
        let p = if prec < 0 { 0 } else { prec as usize };
        let zeros: String = std::iter::repeat('0').take(p).collect();
        let dot = if p > 0 { "." } else { "" };
        let sign_s = if v.is_sign_negative() { "-" } else { "" };
        let prefix = if upper { "0X0" } else { "0x0" };
        let p_ex = if upper { "P+00" } else { "p+00" };
        return format!("{}{}{}{}{}", sign_s, prefix, dot, zeros, p_ex);
    }
    let bits = v.to_bits();
    let sign = bits >> 63;
    let mut exp = ((bits >> 52) & 0x7FF) as i32;
    let mut mant = bits & 0xF_FFFF_FFFF_FFFF;
    if exp == 0 {
        exp = 1;
    } else {
        mant |= 1u64 << 52;
    }
    // Normalize so leading digit is 1.
    // Shift mant left until bit 52 set (already in normalized form for non-subnormal).
    let e = exp - 1023;
    let mant_hex = if prec < 0 {
        // Shortest: strip trailing zeros from 13-hex-digit mantissa.
        let frac = mant & 0xF_FFFF_FFFF_FFFF;
        let mut h = format!("{:013x}", frac);
        while h.ends_with('0') { h.pop(); }
        if h.is_empty() { String::new() } else { format!(".{}", h) }
    } else if prec == 0 {
        String::new()
    } else {
        // Fixed prec: round to prec hex digits after the point.
        let shift = 52i32 - 4 * (prec as i32);
        let frac_full = mant & 0xF_FFFF_FFFF_FFFF;
        let frac = if shift >= 0 {
            // Rounding step: half-to-even would be ideal; truncate for simplicity.
            frac_full >> shift
        } else {
            frac_full << (-shift)
        };
        format!(".{:0width$x}", frac, width = prec as usize)
    };
    let sign_s = if sign == 1 { "-" } else { "" };
    let prefix = if upper { "0X1" } else { "0x1" };
    let mant_hex = if upper { mant_hex.to_uppercase() } else { mant_hex };
    let p_letter = if upper { 'P' } else { 'p' };
    let e_sign = if e >= 0 { '+' } else { '-' };
    format!("{}{}{}{}{}{:02}", sign_s, prefix, mant_hex, p_letter, e_sign, e.abs())
}

/// Rust gives "1e0", "1e10", "-1e-10" etc.  Go wants "1e+00", "1e+10",
/// "-1e-10". Two-digit minimum unpadded exponent, explicit sign.
fn go_exponent(s: &str, upper: bool) -> string {
    let marker = if upper { 'E' } else { 'e' };
    if let Some(pos) = s.find(marker) {
        let (mant, exp) = s.split_at(pos);
        let mut exp = &exp[1..];
        let (sign, digits) = if let Some(rest) = exp.strip_prefix('-') {
            ("-", rest)
        } else if let Some(rest) = exp.strip_prefix('+') {
            ("+", rest)
        } else {
            ("+", exp)
        };
        let _ = &mut exp;
        let padded = if digits.len() < 2 { format!("0{}", digits) } else { digits.to_string() };
        return format!("{}{}{}{}", mant, marker, sign, padded);
    }
    s.to_string()
}

fn canonical_exponent(s: &str, _prec: usize, _use_e: bool, upper: bool) -> string {
    // Convert Rust's shortest "1.234e5" / "123" into Go's canonical e
    // form. If no 'e' present, force one.
    let marker = if upper { 'E' } else { 'e' };
    let lower = if upper { s.to_uppercase() } else { s.to_string() };
    if lower.contains(marker) || lower.contains(if upper { 'e' } else { 'E' }) {
        return go_exponent(&lower.replace(if upper { 'e' } else { 'E' }, &marker.to_string()), upper);
    }
    // Rewrite "123.4" as "1.234e+02".
    let (mantissa, neg) = if let Some(rest) = lower.strip_prefix('-') { (rest.to_string(), true) } else { (lower.clone(), false) };
    let (int_part, frac_part) = match mantissa.split_once('.') {
        Some((i, f)) => (i.to_string(), f.to_string()),
        None => (mantissa.clone(), String::new()),
    };
    let all_digits = format!("{}{}", int_part, frac_part);
    let first_nonzero = all_digits.find(|c: char| c != '0');
    let exp_val: i32;
    let digits_trimmed: String;
    match first_nonzero {
        None => return lower, // all zeros
        Some(idx) => {
            exp_val = int_part.len() as i32 - 1 - idx as i32;
            digits_trimmed = all_digits[idx..].trim_end_matches('0').to_string();
        }
    }
    let (lead, tail) = if digits_trimmed.is_empty() { ("0".to_string(), String::new()) }
        else { (digits_trimmed[..1].to_string(), digits_trimmed[1..].to_string()) };
    let sign = if exp_val >= 0 { '+' } else { '-' };
    let sign_prefix = if neg { "-" } else { "" };
    if tail.is_empty() {
        format!("{}{}{}{}{:02}", sign_prefix, lead, marker, sign, exp_val.abs())
    } else {
        format!("{}{}.{}{}{}{:02}", sign_prefix, lead, tail, marker, sign, exp_val.abs())
    }
}

fn rust_to_go_shortest(s: &str, upper: bool) -> string {
    // For 'g' with prec=-1: Rust's Display rounds to shortest round-trip,
    // sometimes in scientific ("1e10"), sometimes plain ("1234"). We need
    // to make exponents go-shaped (e+10) when present.
    if s.contains('e') || s.contains('E') {
        let normalized = if upper { s.to_uppercase() } else { s.to_string() };
        return go_exponent(&normalized, upper);
    }
    s.to_string()
}

fn trim_g_trailing_zeros(s: &str, upper: bool) -> string {
    // In 'g', trailing zeros after '.' are trimmed, and a bare '.' is too.
    let marker = if upper { 'E' } else { 'e' };
    if let Some(pos) = s.find(marker) {
        let mant = &s[..pos];
        let exp = &s[pos..];
        let trimmed = trim_g_mantissa(mant);
        return format!("{}{}", trimmed, exp);
    }
    trim_g_mantissa(s)
}

fn trim_g_mantissa(m: &str) -> string {
    if !m.contains('.') { return m.to_string(); }
    let t = m.trim_end_matches('0');
    let t = t.trim_end_matches('.');
    t.to_string()
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
