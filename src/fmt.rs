// fmt: Go's fmt package, ported.
//
//   Go                                goish
//   ───────────────────────────────   ──────────────────────────────────
//   fmt.Println("hi", x)              fmt::Println!("hi", x)
//   fmt.Printf("%d items\n", n)       fmt::Printf!("%d items\n", n)
//   s := fmt.Sprintf("%.2f", pi)      let s = fmt::Sprintf!("%.2f", pi);
//   fmt.Fprintf(w, "%s\n", name)      fmt::Fprintf!(w, "%s\n", name);
//   err := fmt.Errorf("bad: %s", e)   let err = fmt::Errorf!("bad: %s", e);
//
// Verbs supported: %s %d %f %v %t %x %X %o %b %q %p %% with optional
// flags (- + 0 #), width, and .precision (e.g. %-10s, %06d, %.3f).

use std::fmt::Display;

/// Arg variant used by `Errorf!` — keeps both the Display surface and
/// the typed error identity so `%w` can reach the wrap target.
pub enum FmtArg<'a> {
    Disp(&'a (dyn Display + 'a)),
    Err(&'a crate::errors::error),
}

impl<'a> FmtArg<'a> {
    fn as_display(&self) -> &(dyn Display + 'a) {
        match self {
            FmtArg::Disp(d) => *d,
            FmtArg::Err(e) => *e,
        }
    }
    fn as_error(&self) -> Option<&'a crate::errors::error> {
        match self { FmtArg::Err(e) => Some(*e), _ => None }
    }
}

/// Autoref-based specialization so `fmt_arg!($x)` picks `FmtArg::Err`
/// when `$x` is an `error`, and `FmtArg::Disp` otherwise. Not a public API.
#[doc(hidden)]
pub mod __fmt_arg {
    use super::*;
    pub struct Wrap<'a, T: ?Sized>(pub &'a T);

    // Copy so the ref trait impl can consume self by value (sidesteps
    // lifetime-collapse on `&self`).
    impl<'a, T: ?Sized> Copy for Wrap<'a, T> {}
    impl<'a, T: ?Sized> Clone for Wrap<'a, T> { fn clone(&self) -> Self { *self } }

    pub trait ViaError<'a> { fn fmt_arg(self) -> FmtArg<'a>; }
    pub trait ViaDisplay<'a> { fn fmt_arg(self) -> FmtArg<'a>; }

    impl<'a> ViaError<'a> for Wrap<'a, crate::errors::error> {
        fn fmt_arg(self) -> FmtArg<'a> { FmtArg::Err(self.0) }
    }
    impl<'a, T: Display> ViaDisplay<'a> for &'a Wrap<'a, T> {
        fn fmt_arg(self) -> FmtArg<'a> { FmtArg::Disp(self.0) }
    }
}

/// Internal: `errorf_impl(fmt, &[...])` returns (message, optional wrap target).
/// A `%w` verb binds the next arg as the wrap target — its error chain
/// becomes the returned error's source.
pub fn errorf_impl(fmt_str: &str, args: &[FmtArg]) -> crate::errors::error {
    let (msg, wrap) = go_format_errorf(fmt_str, args);
    match wrap {
        Some(w) => crate::errors::New_with_source(&msg, w.clone()),
        None => crate::errors::New(&msg),
    }
}

/// Format scanner that understands `%w`. Substitutes each `%w` with its
/// arg's Error() string; records the first %w's error as the wrap target
/// (Go's semantics for fmt.Errorf).
pub fn go_format_errorf(fmt_str: &str, args: &[FmtArg]) -> (String, Option<crate::errors::error>) {
    let bytes = fmt_str.as_bytes();
    let mut out = String::with_capacity(fmt_str.len() * 2);
    let mut arg_idx = 0usize;
    let mut i = 0usize;
    let mut wrap_target: Option<crate::errors::error> = None;

    while i < bytes.len() {
        if bytes[i] != b'%' {
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }
        i += 1;
        if i >= bytes.len() {
            out.push('%');
            break;
        }
        if bytes[i] == b'%' { out.push('%'); i += 1; continue; }

        // Flags / width / precision parsing (duplicate go_format's logic).
        let mut flags = String::new();
        while i < bytes.len() && matches!(bytes[i], b'-' | b'+' | b' ' | b'0' | b'#') {
            flags.push(bytes[i] as char); i += 1;
        }
        let mut width: Option<usize> = None;
        let ws = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() { i += 1; }
        if i > ws { width = fmt_str[ws..i].parse().ok(); }
        let mut precision: Option<usize> = None;
        if i < bytes.len() && bytes[i] == b'.' {
            i += 1;
            let ps = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() { i += 1; }
            precision = fmt_str[ps..i].parse().ok();
        }
        if i >= bytes.len() { out.push('%'); break; }

        let verb = bytes[i] as char;
        i += 1;

        if arg_idx >= args.len() {
            out.push_str(&format!("%!{}(MISSING)", verb));
            continue;
        }
        let arg = &args[arg_idx];
        arg_idx += 1;

        if verb == 'w' {
            // Substitute with the error's Error() text, record wrap target.
            match arg.as_error() {
                Some(e) => {
                    out.push_str(&format!("{}", e));
                    if wrap_target.is_none() {
                        wrap_target = Some(e.clone());
                    }
                }
                None => {
                    // Non-error arg at %w — Go emits "%!w(type=val)".
                    out.push_str(&format!("%!w(string={})", arg.as_display()));
                }
            }
            continue;
        }

        let raw = format!("{}", arg.as_display());
        out.push_str(&apply_verb(&raw, verb, width, precision, &flags));
    }

    (out, wrap_target)
}

// ── Autoref specialization: Display-or-Debug per arg ─────────────────
//
// The Sprintf!/Printf!/etc. macros add `&&$arg` at the call site. Rust's
// method resolution then prefers the Display-bounded impl (one deref
// needed) over the Debug-only impl (two derefs). This gives:
//   - T: Display  → uses Display (Go-faithful for string verbs)
//   - T: Debug    → falls back to Debug (works for slice<T>, map, Struct!)
//   - Neither     → compile error with missing-trait message

// Autoref specialization via inherent-vs-trait method resolution.
//
// `Wrap(&x).__go_fmt_str()`:
//   - Inherent method on Wrap<T> where T: Display → wins if T has Display
//   - Trait method from FallbackDebug → runs if T only has Debug
//
// Rust prefers inherent methods over trait methods, so Display is
// preferred; Debug is the fallback. Every Go-portable type has at
// least Debug (derivable), so this covers slice<T>, map<K,V>, Vec<T>,
// Struct!-generated types, and any custom user struct with #[derive(Debug)].

pub struct __GoFmtWrap<'a, T: ?Sized>(pub &'a T);

impl<T: Display + ?Sized> __GoFmtWrap<'_, T> {
    pub fn __go_fmt_str(&self) -> String { format!("{}", self.0) }
}

pub trait __GoFmtFallback { fn __go_fmt_str(&self) -> String; }
impl<T: std::fmt::Debug + ?Sized> __GoFmtFallback for __GoFmtWrap<'_, T> {
    fn __go_fmt_str(&self) -> String { format!("{:?}", self.0) }
}

/// Pre-rendered-strings form of go_format. Used by the autoref path.
pub fn go_format_strs(fmt_str: &str, args: &[String]) -> String {
    let bytes = fmt_str.as_bytes();
    let mut out = String::with_capacity(fmt_str.len() * 2);
    let mut arg_idx = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != b'%' { out.push(bytes[i] as char); i += 1; continue; }
        i += 1;
        if i >= bytes.len() { out.push('%'); break; }
        if bytes[i] == b'%' { out.push('%'); i += 1; continue; }
        let mut flags = String::new();
        while i < bytes.len() && matches!(bytes[i], b'-' | b'+' | b' ' | b'0' | b'#') {
            flags.push(bytes[i] as char); i += 1;
        }
        let mut width: Option<usize> = None;
        { let start = i;
          while i < bytes.len() && bytes[i].is_ascii_digit() { i += 1; }
          if i > start { width = fmt_str[start..i].parse().ok(); } }
        let mut precision: Option<usize> = None;
        if i < bytes.len() && bytes[i] == b'.' {
            i += 1; let start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() { i += 1; }
            precision = fmt_str[start..i].parse().ok();
        }
        if i >= bytes.len() { out.push('%'); break; }
        let verb = bytes[i] as char; i += 1;
        if arg_idx >= args.len() {
            out.push_str(&format!("%!{}(MISSING)", verb)); continue;
        }
        let raw = &args[arg_idx];
        arg_idx += 1;
        out.push_str(&apply_verb(raw, verb, width, precision, &flags));
    }
    out
}

pub fn go_format(fmt_str: &str, args: &[&dyn Display]) -> String {
    let bytes = fmt_str.as_bytes();
    let mut out = String::with_capacity(fmt_str.len() * 2);
    let mut arg_idx = 0usize;
    let mut i = 0usize;

    while i < bytes.len() {
        if bytes[i] != b'%' {
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }
        i += 1;
        if i >= bytes.len() {
            out.push('%');
            break;
        }
        if bytes[i] == b'%' {
            out.push('%');
            i += 1;
            continue;
        }

        // [flags][width][.precision]verb
        let mut flags = String::new();
        while i < bytes.len() && matches!(bytes[i], b'-' | b'+' | b' ' | b'0' | b'#') {
            flags.push(bytes[i] as char);
            i += 1;
        }
        let mut width: Option<usize> = None;
        {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i > start {
                width = fmt_str[start..i].parse().ok();
            }
        }
        let mut precision: Option<usize> = None;
        if i < bytes.len() && bytes[i] == b'.' {
            i += 1;
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            precision = fmt_str[start..i].parse().ok();
        }

        if i >= bytes.len() {
            out.push('%');
            break;
        }
        let verb = bytes[i] as char;
        i += 1;

        if arg_idx >= args.len() {
            out.push_str(&format!("%!{}(MISSING)", verb));
            continue;
        }
        let raw = format!("{}", args[arg_idx]);
        arg_idx += 1;

        out.push_str(&apply_verb(&raw, verb, width, precision, &flags));
    }

    out
}

/// Go's `%q` — double-quoted string with Go-escape sequences for
/// non-printable and non-ASCII characters. Matches `strconv.Quote`.
fn go_quote(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 2);
    out.push('"');
    for c in raw.chars() {
        match c {
            '"'  => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\x07' => out.push_str("\\a"),
            '\x08' => out.push_str("\\b"),
            '\x0c' => out.push_str("\\f"),
            '\x0b' => out.push_str("\\v"),
            c if c.is_ascii() && (c as u32) >= 0x20 && (c as u32) < 0x7f => out.push(c),
            c if (c as u32) < 0x80 => {
                out.push_str(&format!("\\x{:02x}", c as u32));
            }
            c if (c as u32) < 0x10000 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => {
                out.push_str(&format!("\\U{:08x}", c as u32));
            }
        }
    }
    out.push('"');
    out
}

fn apply_verb(raw: &str, verb: char, width: Option<usize>, precision: Option<usize>, flags: &str) -> String {
    fn fmt_float_go(f: f64, prec: usize, upper: bool) -> String {
        if f.is_nan()       { return "NaN".into(); }
        if f.is_infinite()  { return if f < 0.0 { "-Inf".into() } else { "+Inf".into() }; }
        if upper { format!("{:.*}", prec, f).to_uppercase() } else { format!("{:.*}", prec, f) }
    }

    let mut value: std::string::String = match verb {
        'q' => go_quote(raw),
        'f' | 'F' => {
            let p = precision.unwrap_or(6);
            raw.parse::<f64>().map(|f| fmt_float_go(f, p, verb == 'F')).unwrap_or_else(|_| raw.into())
        },
        'e' | 'E' => {
            let p = precision.unwrap_or(6);
            raw.parse::<f64>()
                .map(|f| crate::strconv::FormatFloat(f, verb as u8, p as i64, 64).as_str().to_string())
                .unwrap_or_else(|_| raw.into())
        },
        'g' | 'G' => {
            let p = precision.map(|p| p as i64).unwrap_or(-1);
            raw.parse::<f64>()
                .map(|f| crate::strconv::FormatFloat(f, verb as u8, p, 64).as_str().to_string())
                .unwrap_or_else(|_| raw.into())
        },
        'x' => raw.parse::<i128>().map(|n| format!("{:x}", n)).unwrap_or_else(|_| raw.into()),
        'X' => raw.parse::<i128>().map(|n| format!("{:X}", n)).unwrap_or_else(|_| raw.into()),
        'o' => raw.parse::<i128>().map(|n| format!("{:o}", n)).unwrap_or_else(|_| raw.into()),
        'b' => raw.parse::<i128>().map(|n| format!("{:b}", n)).unwrap_or_else(|_| raw.into()),
        's' => match precision {
            Some(p) if raw.len() > p => raw[..p].into(),
            _ => raw.into(),
        },
        // %d, %v, %t, %p — Display already does the right thing
        _ => raw.into(),
    };

    // Sign flag: `%+d` / `%+f` / etc — prepend '+' for non-negative
    // numeric values.
    if flags.contains('+') && matches!(verb, 'd' | 'f' | 'F' | 'e' | 'E' | 'g' | 'G') && !value.starts_with('-') && !value.starts_with('+') {
        // Suppress for NaN/Inf — Go's "%+f" of +Inf is "+Inf" already.
        if value != "NaN" && !value.starts_with("+Inf") && !value.starts_with("-Inf") {
            value = format!("+{}", value);
        }
    }

    if let Some(w) = width {
        if value.chars().count() < w {
            let pad = w - value.chars().count();
            let zero_pad = flags.contains('0') && !flags.contains('-') && matches!(verb, 'd' | 'f' | 'F' | 'e' | 'E' | 'g' | 'G' | 'x' | 'X' | 'o' | 'b');
            let pad_char = if zero_pad { '0' } else { ' ' };
            let padding: String = std::iter::repeat(pad_char).take(pad).collect();
            value = if flags.contains('-') {
                format!("{}{}", value, padding)
            } else if zero_pad && (value.starts_with('-') || value.starts_with('+')) {
                // Pad between the sign and the digits: "-00042" not "00-42".
                let (sign, body) = value.split_at(1);
                format!("{}{}{}", sign, padding, body)
            } else {
                format!("{}{}", padding, value)
            };
        }
    }
    value
}

// ── Stringer ───────────────────────────────────────────────────────────
//
// Go's `fmt.Stringer` interface:
//
//     type Stringer interface { String() string }
//
// In Rust we expose a `Stringer` trait + a `stringer!` macro that takes
// one Go-style impl block and emits all three of: an inherent `String()`
// method, the `Stringer` trait impl, and the matching `Display` impl.
//
// Usage:
//
//     struct Color { r: int, g: int, b: int }
//
//     stringer! {
//         impl Color {
//             fn String(&self) -> string {
//                 Sprintf!("#%02x%02x%02x", self.r, self.g, self.b)
//             }
//         }
//     }
//
//     Println!("color:", Color { r: 255, g: 0, b: 0 });
//     // → color: #ff0000

pub trait Stringer {
    fn String(&self) -> crate::types::string;
}

/// Generate Stringer + Display + an inherent `String()` method from one Go-style block.
///
/// The user's `fn String(&self) -> string { ... }` is captured as a whole item
/// and emitted verbatim inside `impl $ty { ... }`, which preserves `self`
/// hygiene. The macro then layers on the `Stringer` trait impl and the
/// matching `Display` impl.
#[macro_export]
macro_rules! stringer {
    (impl $ty:ty { $($item:item)+ }) => {
        #[allow(non_snake_case)]
        impl $ty {
            $($item)+
        }
        impl $crate::fmt::Stringer for $ty {
            fn String(&self) -> $crate::types::string {
                <$ty>::String(self)
            }
        }
        impl ::std::fmt::Display for $ty {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                ::std::write!(f, "{}", <$ty>::String(self))
            }
        }
    };
}

// ── macros ─────────────────────────────────────────────────────────────
//
// All five live at the crate root via #[macro_export], and are re-exported
// below so users can also call them as fmt::Println!(...), etc.

/// fmt.Println(a, b, c) — space-separated, trailing newline. Returns (int, error).
#[macro_export]
macro_rules! Println {
    ($($arg:expr),* $(,)?) => {{
        let parts: Vec<String> = vec![ $( format!("{}", $arg) ),* ];
        let s = parts.join(" ");
        println!("{}", s);
        ((s.len() + 1) as $crate::int, $crate::errors::nil)
    }};
}

/// fmt.Printf(format, args...) — Go-style verbs. Returns (int, error).
#[macro_export]
macro_rules! Printf {
    ($fmt:expr $(, $arg:expr)* $(,)?) => {{
        use $crate::fmt::__GoFmtFallback as _;
        let out = $crate::fmt::go_format_strs(
            $fmt,
            &[ $( $crate::fmt::__GoFmtWrap(&$arg).__go_fmt_str() ),* ],
        );
        print!("{}", out);
        (out.len() as $crate::int, $crate::errors::nil)
    }};
}

/// fmt.Sprintf(format, args...) — returns the formatted string.
#[macro_export]
macro_rules! Sprintf {
    ($fmt:expr $(, $arg:expr)* $(,)?) => {{
        use $crate::fmt::__GoFmtFallback as _;
        let _s: $crate::types::string = $crate::fmt::go_format_strs(
            $fmt,
            &[ $( $crate::fmt::__GoFmtWrap(&$arg).__go_fmt_str() ),* ],
        ).into();
        _s
    }};
}

/// fmt.Fprintf(w, format, args...) — writes to anything that impls io::Write.
/// Returns (int, error).
#[macro_export]
macro_rules! Fprintf {
    ($w:expr, $fmt:expr $(, $arg:expr)* $(,)?) => {{
        use $crate::fmt::__GoFmtFallback as _;
        let out = $crate::fmt::go_format_strs(
            $fmt,
            &[ $( $crate::fmt::__GoFmtWrap(&$arg).__go_fmt_str() ),* ],
        );
        use ::std::io::Write as _;
        match write!($w, "{}", out) {
            Ok(()) => (out.len() as $crate::int, $crate::errors::nil),
            Err(e) => (0 as $crate::int, $crate::errors::New(&format!("{}", e))),
        }
    }};
}

/// fmt.Errorf(format, args...) — returns an error with the formatted
/// message. Supports `%w` to wrap a single error; its `.Error()` text
/// replaces the verb at format time, and the returned error unwraps to
/// the original.
#[macro_export]
macro_rules! Errorf {
    ($fmt:expr $(, $arg:expr)* $(,)?) => {{
        #[allow(unused_imports)]
        use $crate::fmt::__fmt_arg::{ViaError as _, ViaDisplay as _};
        $crate::fmt::errorf_impl(
            $fmt,
            &[ $( $crate::fmt::__fmt_arg::Wrap(&$arg).fmt_arg() ),* ],
        )
    }};
}

// Re-export so `fmt::Println!(...)` works in addition to `Println!(...)`.
pub use crate::{Errorf, Fprintf, Printf, Println, Sprintf, stringer};

#[cfg(test)]
mod tests {
    #[test]
    fn sprintf_quote_escapes() {
        // Go: fmt.Sprintf("%q", "hello") → "hello"
        assert_eq!(crate::Sprintf!("%q", "hello"), "\"hello\"");
        // Go: newline → \n, tab → \t, quote → \", backslash → \\
        assert_eq!(crate::Sprintf!("%q", "a\nb"), "\"a\\nb\"");
        assert_eq!(crate::Sprintf!("%q", "a\tb"), "\"a\\tb\"");
        assert_eq!(crate::Sprintf!("%q", "a\"b"), "\"a\\\"b\"");
        assert_eq!(crate::Sprintf!("%q", "a\\b"), "\"a\\\\b\"");
        // Non-printable ASCII → \xHH
        assert_eq!(crate::Sprintf!("%q", "\x01x"), "\"\\x01x\"");
    }

    #[test]
    fn sprintf_basic_verbs() {
        let s = crate::Sprintf!("%s is %d", "x", 42);
        assert_eq!(s, "x is 42");
    }

    #[test]
    fn sprintf_precision() {
        let s = crate::Sprintf!("%.2f", 3.14159);
        assert_eq!(s, "3.14");
    }

    #[test]
    fn sprintf_padding() {
        assert_eq!(crate::Sprintf!("%05d", 7), "00007");
        assert_eq!(crate::Sprintf!("%-5s|", "ab"), "ab   |");
        assert_eq!(crate::Sprintf!("%5s|", "ab"), "   ab|");
    }

    #[test]
    fn sprintf_hex_and_bool() {
        assert_eq!(crate::Sprintf!("%x", 255), "ff");
        assert_eq!(crate::Sprintf!("%X", 255), "FF");
        assert_eq!(crate::Sprintf!("%t", true), "true");
    }

    #[test]
    fn sprintf_literal_percent() {
        assert_eq!(crate::Sprintf!("100%%"), "100%");
    }

    #[test]
    fn errorf_returns_goerror() {
        let e = crate::Errorf!("bad: %s", "oops");
        assert_eq!(format!("{}", e), "bad: oops");
    }

    #[test]
    fn fprintf_to_vec() {
        let mut buf: Vec<u8> = Vec::new();
        let _ = crate::Fprintf!(&mut buf, "hello %s", "world");
        assert_eq!(String::from_utf8(buf).unwrap(), "hello world");
    }

    use crate::string;

    struct Point { x: crate::int, y: crate::int }
    crate::stringer! {
        impl Point {
            fn String(&self) -> string {
                crate::Sprintf!("(%d, %d)", self.x, self.y)
            }
        }
    }

    #[test]
    fn stringer_drives_display_and_method() {
        let p = Point { x: 3, y: 4 };
        // inherent String()
        assert_eq!(p.String(), "(3, 4)");
        // Display via {}
        assert_eq!(format!("{}", p), "(3, 4)");
        // Sprintf %s
        assert_eq!(crate::Sprintf!("%s", p), "(3, 4)");
        // Stringer trait via dyn
        let s: &dyn crate::fmt::Stringer = &p;
        assert_eq!(s.String(), "(3, 4)");
    }
}
