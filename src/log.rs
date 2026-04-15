// log: Go's log package.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   log.Println("msg", x)               log::Println!("msg", x)
//   log.Printf("n=%d", n)               log::Printf!("n=%d", n)
//   log.Fatalf("fatal: %s", err)        log::Fatalf!("fatal: %s", err)
//   log.Panic("boom")                   log::Panic!("boom")
//
// Writes to stderr, prepending a timestamp like Go's default logger
// (`YYYY/MM/DD HH:MM:SS `). `Fatalf` exits with code 1; `Panic` panics.
//
// Timestamp uses `std::time::SystemTime` + a simple epoch-seconds formatter
// — no `chrono` dep. Format matches Go exactly: "2026/04/15 16:02:11 ".

use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

/// Format `epoch_secs` as "YYYY/MM/DD HH:MM:SS".
/// Zeller-style civil date conversion; no time zone (UTC-local-matching).
pub fn timestamp_prefix() -> String {
    let d = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => d,
        Err(_) => return String::new(),
    };
    let total_secs = d.as_secs() as i64;
    let secs_in_day = 86_400i64;
    let days = total_secs / secs_in_day;
    let mut tod = total_secs % secs_in_day;
    if tod < 0 { tod += secs_in_day; }
    let hour = tod / 3600;
    let min = (tod % 3600) / 60;
    let sec = tod % 60;

    // Days since 1970-01-01 → Y/M/D via Howard Hinnant's algorithm.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as i64;
    let yoe = (doe - doe/1460 + doe/36524 - doe/146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365*yoe + yoe/4 - yoe/100);
    let mp = (5*doy + 2) / 153;
    let d_ = doy - (153*mp + 2)/5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };

    format!("{:04}/{:02}/{:02} {:02}:{:02}:{:02} ", year, m, d_, hour, min, sec)
}

/// Internal helper invoked by the log macros. Writes `msg` to stderr with
/// the standard Go timestamp prefix and a trailing newline if missing.
pub fn _log(msg: &str) {
    let stamp = timestamp_prefix();
    let mut out = std::io::stderr();
    let _ = out.write_all(stamp.as_bytes());
    let _ = out.write_all(msg.as_bytes());
    if !msg.ends_with('\n') {
        let _ = out.write_all(b"\n");
    }
    let _ = out.flush();
}

/// log.Println(a, b, c) — space-separated, newline-terminated, stderr.
#[macro_export]
macro_rules! log_Println {
    ($($arg:expr),* $(,)?) => {{
        let parts: Vec<::std::string::String> = vec![ $( format!("{}", $arg) ),* ];
        $crate::log::_log(&parts.join(" "));
    }};
}

/// log.Printf(fmt, ...) — Go-style verbs, written to stderr with timestamp.
#[macro_export]
macro_rules! log_Printf {
    ($fmt:expr $(, $arg:expr)* $(,)?) => {{
        let out = $crate::fmt::go_format($fmt, &[ $( &$arg as &dyn ::std::fmt::Display ),* ]);
        $crate::log::_log(&out);
    }};
}

/// log.Fatalf — like Printf, but then os.Exit(1).
#[macro_export]
macro_rules! log_Fatalf {
    ($fmt:expr $(, $arg:expr)* $(,)?) => {{
        let out = $crate::fmt::go_format($fmt, &[ $( &$arg as &dyn ::std::fmt::Display ),* ]);
        $crate::log::_log(&out);
        $crate::os::Exit(1);
    }};
}

/// log.Panic — like Printf, but then panic!.
#[macro_export]
macro_rules! log_Panic {
    ($fmt:expr $(, $arg:expr)* $(,)?) => {{
        let out = $crate::fmt::go_format($fmt, &[ $( &$arg as &dyn ::std::fmt::Display ),* ]);
        $crate::log::_log(&out);
        panic!("{}", out);
    }};
}

// Re-export so users write `log::Println!(...)` etc.
pub use crate::log_Fatalf as Fatalf;
pub use crate::log_Panic as Panic;
pub use crate::log_Printf as Printf;
pub use crate::log_Println as Println;

#[cfg(test)]
mod tests {
    #[test]
    fn timestamp_has_expected_shape() {
        let ts = super::timestamp_prefix();
        // "YYYY/MM/DD HH:MM:SS " = 20 chars
        assert_eq!(ts.len(), 20);
        assert_eq!(&ts[4..5], "/");
        assert_eq!(&ts[7..8], "/");
        assert_eq!(&ts[10..11], " ");
        assert_eq!(&ts[13..14], ":");
        assert_eq!(&ts[16..17], ":");
    }

    #[test]
    fn println_and_printf_do_not_panic() {
        crate::log::Println!("hello", 42);
        crate::log::Printf!("n=%d", 7);
    }
}
