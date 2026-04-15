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

fn apply_verb(raw: &str, verb: char, width: Option<usize>, precision: Option<usize>, flags: &str) -> String {
    let mut value = match verb {
        'q' => format!("\"{}\"", raw),
        'f' => match precision {
            Some(p) => raw.parse::<f64>().map(|f| format!("{:.*}", p, f)).unwrap_or_else(|_| raw.to_string()),
            None => raw.parse::<f64>().map(|f| format!("{}", f)).unwrap_or_else(|_| raw.to_string()),
        },
        'x' => raw.parse::<i128>().map(|n| format!("{:x}", n)).unwrap_or_else(|_| raw.to_string()),
        'X' => raw.parse::<i128>().map(|n| format!("{:X}", n)).unwrap_or_else(|_| raw.to_string()),
        'o' => raw.parse::<i128>().map(|n| format!("{:o}", n)).unwrap_or_else(|_| raw.to_string()),
        'b' => raw.parse::<i128>().map(|n| format!("{:b}", n)).unwrap_or_else(|_| raw.to_string()),
        's' => match precision {
            Some(p) if raw.len() > p => raw[..p].to_string(),
            _ => raw.to_string(),
        },
        // %d, %v, %t, %p — Display already does the right thing
        _ => raw.to_string(),
    };

    if let Some(w) = width {
        if value.chars().count() < w {
            let pad = w - value.chars().count();
            let zero_pad = flags.contains('0') && !flags.contains('-') && matches!(verb, 'd' | 'f' | 'x' | 'X' | 'o' | 'b');
            let pad_char = if zero_pad { '0' } else { ' ' };
            let padding: String = std::iter::repeat(pad_char).take(pad).collect();
            value = if flags.contains('-') {
                format!("{}{}", value, padding)
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
        let out = $crate::fmt::go_format($fmt, &[ $( &$arg as &dyn ::std::fmt::Display ),* ]);
        print!("{}", out);
        (out.len() as $crate::int, $crate::errors::nil)
    }};
}

/// fmt.Sprintf(format, args...) — returns the formatted string.
#[macro_export]
macro_rules! Sprintf {
    ($fmt:expr $(, $arg:expr)* $(,)?) => {{
        $crate::fmt::go_format($fmt, &[ $( &$arg as &dyn ::std::fmt::Display ),* ])
    }};
}

/// fmt.Fprintf(w, format, args...) — writes to anything that impls io::Write.
/// Returns (int, error).
#[macro_export]
macro_rules! Fprintf {
    ($w:expr, $fmt:expr $(, $arg:expr)* $(,)?) => {{
        let out = $crate::fmt::go_format($fmt, &[ $( &$arg as &dyn ::std::fmt::Display ),* ]);
        use ::std::io::Write as _;
        match write!($w, "{}", out) {
            Ok(()) => (out.len() as $crate::int, $crate::errors::nil),
            Err(e) => (0 as $crate::int, $crate::errors::New(&format!("{}", e))),
        }
    }};
}

/// fmt.Errorf(format, args...) — returns an error with the formatted message.
#[macro_export]
macro_rules! Errorf {
    ($fmt:expr $(, $arg:expr)* $(,)?) => {{
        $crate::errors::New(
            &$crate::fmt::go_format($fmt, &[ $( &$arg as &dyn ::std::fmt::Display ),* ])
        )
    }};
}

// Re-export so `fmt::Println!(...)` works in addition to `Println!(...)`.
pub use crate::{Errorf, Fprintf, Printf, Println, Sprintf, stringer};

#[cfg(test)]
mod tests {
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
