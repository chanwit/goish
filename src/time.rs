// time: Go's time package, ported.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   time.Sleep(2 * time.Second)         time::Sleep(time::Second * 2);
//   start := time.Now()                 let start = time::Now();
//   elapsed := time.Since(start)        let elapsed = time::Since(start);
//   d := 500 * time.Millisecond         let d = time::Millisecond * 500i64;
//   d.Seconds()                         d.Seconds()
//   t2.Sub(t1)                          t2.Sub(t1)
//
// Time values are `Instant`-based (monotonic, cannot be formatted as a
// wall-clock date). Wall-clock parsing/formatting is out of scope for v0.1.

use std::ops::{Add, Mul, Sub};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Duration {
    nanos: i128,
}

impl Duration {
    pub const fn from_nanos(n: i128) -> Self {
        Duration { nanos: n }
    }

    pub fn Nanoseconds(&self) -> crate::types::int64 {
        self.nanos as crate::types::int64
    }

    pub fn Microseconds(&self) -> crate::types::int64 {
        (self.nanos / 1_000) as crate::types::int64
    }

    pub fn Milliseconds(&self) -> crate::types::int64 {
        (self.nanos / 1_000_000) as crate::types::int64
    }

    pub fn Seconds(&self) -> crate::types::float64 {
        self.nanos as f64 / 1_000_000_000.0
    }

    pub fn Minutes(&self) -> crate::types::float64 {
        self.Seconds() / 60.0
    }

    pub fn Hours(&self) -> crate::types::float64 {
        self.Seconds() / 3600.0
    }

    /// Internal conversion to std::time::Duration for sleep/arithmetic.
    /// Saturates at zero for negative values (matches std::thread::sleep).
    pub fn to_std(&self) -> std::time::Duration {
        if self.nanos <= 0 {
            std::time::Duration::ZERO
        } else {
            std::time::Duration::from_nanos(self.nanos as u64)
        }
    }

    /// Go-style string: "1h2m3s", "500ms", "1.5s", etc.
    pub fn String(&self) -> crate::types::string {
        if self.nanos == 0 {
            return "0s".to_string();
        }
        let mut n = self.nanos;
        let neg = n < 0;
        if neg {
            n = -n;
        }

        // Sub-second
        if n < 1_000_000_000 {
            let mut prefix = String::new();
            if neg { prefix.push('-'); }
            if n < 1_000 {
                return format!("{}{}ns", prefix, n);
            }
            if n < 1_000_000 {
                return format!("{}{}µs", prefix, n as f64 / 1_000.0);
            }
            return format!("{}{}ms", prefix, n as f64 / 1_000_000.0);
        }

        // >= 1 second — break into h/m/s
        let mut s = String::new();
        if neg { s.push('-'); }
        let total_secs = n / 1_000_000_000;
        let rem_nanos = n % 1_000_000_000;
        let hours = total_secs / 3600;
        let mins = (total_secs / 60) % 60;
        let secs = total_secs % 60;

        if hours > 0 {
            s.push_str(&format!("{}h", hours));
        }
        if mins > 0 || hours > 0 {
            s.push_str(&format!("{}m", mins));
        }
        if rem_nanos == 0 {
            s.push_str(&format!("{}s", secs));
        } else {
            let f = secs as f64 + rem_nanos as f64 / 1_000_000_000.0;
            s.push_str(&format!("{}s", f));
        }
        s
    }
}

impl std::fmt::Display for Duration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.String())
    }
}

// Duration arithmetic: Duration + Duration, Duration - Duration,
// Duration * n, n * Duration (n: int or float).
impl Add for Duration {
    type Output = Duration;
    fn add(self, other: Duration) -> Duration {
        Duration::from_nanos(self.nanos + other.nanos)
    }
}

impl Sub for Duration {
    type Output = Duration;
    fn sub(self, other: Duration) -> Duration {
        Duration::from_nanos(self.nanos - other.nanos)
    }
}

macro_rules! impl_dur_mul {
    ($($t:ty),+) => {
        $(
            impl Mul<$t> for Duration {
                type Output = Duration;
                fn mul(self, rhs: $t) -> Duration {
                    Duration::from_nanos(self.nanos * rhs as i128)
                }
            }
            impl Mul<Duration> for $t {
                type Output = Duration;
                fn mul(self, rhs: Duration) -> Duration {
                    Duration::from_nanos(rhs.nanos * self as i128)
                }
            }
        )+
    };
}
impl_dur_mul!(i32, i64, u32, u64, usize);

impl Mul<f64> for Duration {
    type Output = Duration;
    fn mul(self, rhs: f64) -> Duration {
        Duration::from_nanos((self.nanos as f64 * rhs) as i128)
    }
}
impl Mul<Duration> for f64 {
    type Output = Duration;
    fn mul(self, rhs: Duration) -> Duration {
        Duration::from_nanos((rhs.nanos as f64 * self) as i128)
    }
}

// Standard Go constants — all of type Duration, in nanoseconds.
#[allow(non_upper_case_globals)]
pub const Nanosecond: Duration = Duration::from_nanos(1);
#[allow(non_upper_case_globals)]
pub const Microsecond: Duration = Duration::from_nanos(1_000);
#[allow(non_upper_case_globals)]
pub const Millisecond: Duration = Duration::from_nanos(1_000_000);
#[allow(non_upper_case_globals)]
pub const Second: Duration = Duration::from_nanos(1_000_000_000);
#[allow(non_upper_case_globals)]
pub const Minute: Duration = Duration::from_nanos(60 * 1_000_000_000);
#[allow(non_upper_case_globals)]
pub const Hour: Duration = Duration::from_nanos(3600 * 1_000_000_000);

// ─── Time ────────────────────────────────────────────────────────────
//
// Time carries both a monotonic `Instant` (for Sub/Since/Until arithmetic
// that should ignore clock jumps) and a wall-clock nanosecond count since
// the Unix epoch (UTC) for formatting and parsing.

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Time {
    instant: std::time::Instant,
    /// Nanoseconds since Unix epoch, UTC. None if this Time was produced
    /// from a wall-clock-less source (e.g. `Instant::now()` in isolation).
    unix_nanos: i128,
}

impl Default for Time {
    fn default() -> Self {
        Time { instant: std::time::Instant::now(), unix_nanos: 0 }
    }
}

impl Time {
    /// t2.Sub(t1) — returns the duration that has elapsed between t1 and t2.
    pub fn Sub(self, earlier: Time) -> Duration {
        match self.instant.checked_duration_since(earlier.instant) {
            Some(d) => Duration::from_nanos(d.as_nanos() as i128),
            None => {
                let d = earlier.instant.duration_since(self.instant);
                Duration::from_nanos(-(d.as_nanos() as i128))
            }
        }
    }

    /// t.Add(d) — returns a new Time advanced by d.
    pub fn Add(self, d: Duration) -> Time {
        Time {
            instant: self.instant + d.to_std(),
            unix_nanos: self.unix_nanos.saturating_add(d.nanos),
        }
    }

    pub fn After(self, other: Time) -> bool { self.instant > other.instant }
    pub fn Before(self, other: Time) -> bool { self.instant < other.instant }

    /// t.Unix() — seconds since epoch.
    pub fn Unix(&self) -> crate::types::int64 {
        (self.unix_nanos / 1_000_000_000) as crate::types::int64
    }

    /// t.UnixNano() — nanoseconds since epoch.
    pub fn UnixNano(&self) -> crate::types::int64 {
        self.unix_nanos as crate::types::int64
    }

    /// t.Format(layout) — Go's reference layout. The layout uses specific
    /// numeric magic: "2006" for year, "01" month, "02" day, "15" hour,
    /// "04" minute, "05" second.
    pub fn Format(&self, layout: impl AsRef<str>) -> crate::types::string {
        let (y, mo, d, h, mi, s) = self.civil();
        let mut out = String::new();
        let mut rest = layout.as_ref();
        loop {
            if rest.starts_with("2006") {
                out.push_str(&format!("{:04}", y)); rest = &rest[4..];
            } else if rest.starts_with("01") {
                out.push_str(&format!("{:02}", mo)); rest = &rest[2..];
            } else if rest.starts_with("02") {
                out.push_str(&format!("{:02}", d)); rest = &rest[2..];
            } else if rest.starts_with("15") {
                out.push_str(&format!("{:02}", h)); rest = &rest[2..];
            } else if rest.starts_with("04") {
                out.push_str(&format!("{:02}", mi)); rest = &rest[2..];
            } else if rest.starts_with("05") {
                out.push_str(&format!("{:02}", s)); rest = &rest[2..];
            } else {
                match rest.chars().next() {
                    Some(c) => { out.push(c); rest = &rest[c.len_utf8()..]; }
                    None => break,
                }
            }
        }
        out
    }

    /// Break the stored wall-clock into (year, month, day, hour, min, sec).
    fn civil(&self) -> (i64, u32, u32, u32, u32, u32) {
        let total_secs = (self.unix_nanos / 1_000_000_000) as i64;
        let mut tod = total_secs.rem_euclid(86_400);
        let days = total_secs.div_euclid(86_400);
        let h = (tod / 3600) as u32; tod %= 3600;
        let mi = (tod / 60) as u32;
        let s = (tod % 60) as u32;

        // Howard Hinnant's civil-from-days.
        let z = days + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = z - era * 146_097;
        let yoe = (doe - doe/1460 + doe/36524 - doe/146096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365*yoe + yoe/4 - yoe/100);
        let mp = (5*doy + 2) / 153;
        let d = (doy - (153*mp + 2)/5 + 1) as u32;
        let mo = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
        let year = if mo <= 2 { y + 1 } else { y };
        (year, mo, d, h, mi, s)
    }
}

/// time.Date(year, month, day, hour, min, sec, nsec, loc) — construct a
/// wall-clock Time. `_loc` is accepted for signature compatibility but
/// currently only UTC is supported.
#[allow(non_snake_case, clippy::too_many_arguments)]
pub fn Date(year: crate::types::int, month: crate::types::int, day: crate::types::int,
            hour: crate::types::int, min: crate::types::int, sec: crate::types::int,
            nsec: crate::types::int, _loc: Location) -> Time {
    let days = days_from_civil(year, month as u32, day as u32);
    let secs = days * 86_400 + hour * 3600 + min * 60 + sec;
    let nanos = secs as i128 * 1_000_000_000 + nsec as i128;
    Time { instant: std::time::Instant::now(), unix_nanos: nanos }
}

fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as i64;
    let mp = if m > 2 { m as i64 - 3 } else { m as i64 + 9 };
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe/4 - yoe/100 + doy;
    era * 146_097 + doe - 719_468
}

/// Minimal `Location` placeholder — only `UTC` is supported today.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Location;

#[allow(non_upper_case_globals)]
pub const UTC: Location = Location;
#[allow(non_upper_case_globals)]
pub const Local: Location = Location;

/// time.Parse(layout, value) — parse the string by the reference layout.
/// Returns (time, error).
#[allow(non_snake_case)]
pub fn Parse(layout: impl AsRef<str>, value: impl AsRef<str>) -> (Time, crate::errors::error) {
    let mut year = 1i64;
    let mut mo = 1u32;
    let mut day = 1u32;
    let mut hour = 0u32;
    let mut min = 0u32;
    let mut sec = 0u32;

    let layout = layout.as_ref();
    let value = value.as_ref();
    let mut li = 0;
    let mut vi = 0;
    let lb = layout.as_bytes();
    let vb = value.as_bytes();

    macro_rules! parse_n {
        ($width:expr, $field:ident, $ty:ty) => {{
            if vi + $width > vb.len() { return bad(value); }
            let s = match std::str::from_utf8(&vb[vi..vi + $width]) {
                Ok(s) => s,
                Err(_) => return bad(value),
            };
            match s.parse::<$ty>() {
                Ok(n) => $field = n,
                Err(_) => return bad(value),
            }
            vi += $width;
        }};
    }

    while li < lb.len() {
        let rest = &layout[li..];
        if rest.starts_with("2006") { parse_n!(4, year, i64); li += 4; }
        else if rest.starts_with("01") { parse_n!(2, mo, u32); li += 2; }
        else if rest.starts_with("02") { parse_n!(2, day, u32); li += 2; }
        else if rest.starts_with("15") { parse_n!(2, hour, u32); li += 2; }
        else if rest.starts_with("04") { parse_n!(2, min, u32); li += 2; }
        else if rest.starts_with("05") { parse_n!(2, sec, u32); li += 2; }
        else {
            if vi >= vb.len() || vb[vi] != lb[li] { return bad(value); }
            li += 1; vi += 1;
        }
    }
    if vi != vb.len() { return bad(value); }

    let t = Date(year, mo as crate::types::int, day as crate::types::int,
                 hour as crate::types::int, min as crate::types::int, sec as crate::types::int,
                 0, UTC);
    (t, crate::errors::nil)
}

fn bad(s: &str) -> (Time, crate::errors::error) {
    (Time { instant: std::time::Instant::now(), unix_nanos: 0 },
     crate::errors::New(&format!("time.Parse: cannot parse {:?}", s)))
}

/// time.Now() — wall clock + monotonic "now".
#[allow(non_snake_case)]
pub fn Now() -> Time {
    let wall = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as i128)
        .unwrap_or(0);
    Time { instant: std::time::Instant::now(), unix_nanos: wall }
}

#[allow(non_snake_case)]
pub fn Since(t: Time) -> Duration { Now().Sub(t) }

#[allow(non_snake_case)]
pub fn Until(t: Time) -> Duration { t.Sub(Now()) }

/// time.Sleep(d) — block the current thread for d.
#[allow(non_snake_case)]
pub fn Sleep(d: Duration) {
    std::thread::sleep(d.to_std());
}

// ── Ticker / Timer / AfterFunc ─────────────────────────────────────────

/// A Ticker sends the current Time on its channel `C` every `d`. Drop it
/// or call `Stop()` to release the worker thread.
pub struct Ticker {
    pub C: crate::chan::Chan<Time>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Ticker {
    pub fn Stop(self) {
        self.stop.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

#[allow(non_snake_case)]
pub fn NewTicker(d: Duration) -> Ticker {
    let ch = crate::chan::Chan::<Time>::new(1);
    let producer = ch.clone();
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stop_clone = stop.clone();
    std::thread::spawn(move || {
        let d_std = d.to_std();
        while !stop_clone.load(std::sync::atomic::Ordering::SeqCst) {
            std::thread::sleep(d_std);
            if stop_clone.load(std::sync::atomic::Ordering::SeqCst) { break; }
            // Best-effort, non-blocking send (buffer 1).
            let _ = producer.Send(Now());
        }
    });
    Ticker { C: ch, stop }
}

/// A Timer fires once after `d` and delivers the Time on its channel `C`.
pub struct Timer {
    pub C: crate::chan::Chan<Time>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Timer {
    /// Stop() returns true if the call stops the timer before it fires.
    pub fn Stop(&self) -> bool {
        let was = self.stop.swap(true, std::sync::atomic::Ordering::SeqCst);
        !was
    }
}

#[allow(non_snake_case)]
pub fn NewTimer(d: Duration) -> Timer {
    let ch = crate::chan::Chan::<Time>::new(1);
    let producer = ch.clone();
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stop_clone = stop.clone();
    std::thread::spawn(move || {
        std::thread::sleep(d.to_std());
        if !stop_clone.load(std::sync::atomic::Ordering::SeqCst) {
            let _ = producer.Send(Now());
        }
    });
    Timer { C: ch, stop }
}

/// time.After(d) — shortcut for NewTimer(d).C.
#[allow(non_snake_case)]
pub fn After(d: Duration) -> crate::chan::Chan<Time> {
    NewTimer(d).C
}

/// time.AfterFunc(d, f) — run `f` on a separate thread after `d`. Returns
/// a Timer you can Stop() to cancel before it fires.
#[allow(non_snake_case)]
pub fn AfterFunc<F: FnOnce() + Send + 'static>(d: Duration, f: F) -> Timer {
    let ch = crate::chan::Chan::<Time>::new(1);
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stop_clone = stop.clone();
    std::thread::spawn(move || {
        std::thread::sleep(d.to_std());
        if !stop_clone.load(std::sync::atomic::Ordering::SeqCst) {
            f();
        }
    });
    Timer { C: ch, stop }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_arithmetic() {
        let one_sec = Second;
        assert_eq!(one_sec.Seconds(), 1.0);
        assert_eq!(one_sec.Milliseconds(), 1000);
        assert_eq!(one_sec.Nanoseconds(), 1_000_000_000);

        let two_sec = Second * 2i64;
        assert_eq!(two_sec.Seconds(), 2.0);

        let three_sec = Second + Second * 2i64;
        assert_eq!(three_sec.Seconds(), 3.0);

        let half = Millisecond * 500i64;
        assert_eq!(half.Milliseconds(), 500);
    }

    #[test]
    fn duration_mul_commutative() {
        assert_eq!(Second * 2i64, 2i64 * Second);
        assert_eq!((Millisecond * 500i64).Milliseconds(), 500);
    }

    #[test]
    fn duration_string_formatting() {
        assert_eq!(Duration::from_nanos(0).String(), "0s");
        assert_eq!((Nanosecond * 500i64).String(), "500ns");
        assert_eq!((Millisecond * 1500i64).String(), "1.5s");
        assert_eq!((Second * 65i64).String(), "1m5s");
        assert_eq!((Hour + Minute * 30i64 + Second * 15i64).String(), "1h30m15s");
    }

    #[test]
    fn now_and_since_monotonic() {
        let t = Now();
        Sleep(Millisecond * 10i64);
        let elapsed = Since(t);
        // Allow generous slack for CI/VM noise
        assert!(elapsed.Milliseconds() >= 5, "elapsed = {}ms", elapsed.Milliseconds());
        assert!(elapsed.Milliseconds() < 2000, "elapsed = {}ms", elapsed.Milliseconds());
    }

    #[test]
    fn time_sub_returns_duration() {
        let t1 = Now();
        Sleep(Millisecond * 2i64);
        let t2 = Now();
        let d = t2.Sub(t1);
        assert!(d.Nanoseconds() > 0);
    }

    #[test]
    fn time_add_advances() {
        let t1 = Now();
        let t2 = t1.Add(Second * 10i64);
        assert!(t2.After(t1));
        assert!(t1.Before(t2));
    }

    #[test]
    fn format_and_parse_round_trip() {
        let t = Date(2026, 4, 15, 10, 30, 45, 0, UTC);
        let s = t.Format("2006-01-02 15:04:05");
        assert_eq!(s, "2026-04-15 10:30:45");
        let (t2, err) = Parse("2006-01-02 15:04:05", &s);
        assert!(err == crate::errors::nil);
        assert_eq!(t2.Format("2006-01-02 15:04:05"), s);
        assert_eq!(t2.Unix(), t.Unix());
    }

    #[test]
    fn format_other_layouts() {
        let t = Date(2026, 1, 2, 3, 4, 5, 0, UTC);
        assert_eq!(t.Format("2006/01/02"), "2026/01/02");
        assert_eq!(t.Format("15:04"), "03:04");
    }

    #[test]
    fn parse_rejects_invalid() {
        let (_, err) = Parse("2006-01-02", "bad-in-put");
        assert!(err != crate::errors::nil);
    }

    #[test]
    fn ticker_fires_and_stops() {
        let t = NewTicker(Millisecond * 10i64);
        // Read two ticks, then stop.
        let (_, ok1) = t.C.Recv();
        let (_, ok2) = t.C.Recv();
        assert!(ok1 && ok2);
        t.Stop();
    }

    #[test]
    fn timer_fires_once() {
        let t = NewTimer(Millisecond * 20i64);
        let (tm, ok) = t.C.Recv();
        assert!(ok);
        assert!(tm.Unix() > 0);
    }

    #[test]
    fn after_func_runs() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        let ran = Arc::new(AtomicBool::new(false));
        let r = ran.clone();
        let _timer = AfterFunc(Millisecond * 20i64, move || {
            r.store(true, Ordering::SeqCst);
        });
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert!(ran.load(Ordering::SeqCst));
    }

    #[test]
    fn timer_stop_before_fire() {
        let t = NewTimer(Millisecond * 50i64);
        assert!(t.Stop());
        // No assertion on C — nothing delivered.
    }
}
