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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Time {
    instant: std::time::Instant,
}

impl Time {
    /// t2.Sub(t1) — returns the duration that has elapsed between t1 and t2.
    pub fn Sub(self, earlier: Time) -> Duration {
        match self.instant.checked_duration_since(earlier.instant) {
            Some(d) => Duration::from_nanos(d.as_nanos() as i128),
            None => {
                // earlier is actually later
                let d = earlier.instant.duration_since(self.instant);
                Duration::from_nanos(-(d.as_nanos() as i128))
            }
        }
    }

    /// t.Add(d) — returns a new Time advanced by d.
    pub fn Add(self, d: Duration) -> Time {
        Time {
            instant: self.instant + d.to_std(),
        }
    }

    /// t.After(other) — true if self is strictly later than other.
    pub fn After(self, other: Time) -> bool {
        self.instant > other.instant
    }

    /// t.Before(other) — true if self is strictly earlier than other.
    pub fn Before(self, other: Time) -> bool {
        self.instant < other.instant
    }
}

/// time.Now() — monotonic "now" instant.
#[allow(non_snake_case)]
pub fn Now() -> Time {
    Time { instant: std::time::Instant::now() }
}

/// time.Since(t) — equivalent to time.Now().Sub(t).
#[allow(non_snake_case)]
pub fn Since(t: Time) -> Duration {
    Now().Sub(t)
}

/// time.Until(t) — equivalent to t.Sub(time.Now()).
#[allow(non_snake_case)]
pub fn Until(t: Time) -> Duration {
    t.Sub(Now())
}

/// time.Sleep(d) — block the current thread for d.
#[allow(non_snake_case)]
pub fn Sleep(d: Duration) {
    std::thread::sleep(d.to_std());
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
}
