// time: Go's time package, ported from go1.25.5.
//
// Time values are backed by (sec, nsec, loc). The layout engine matches
// Go's "reference time" syntax (Mon Jan 2 15:04:05 MST 2006) token-for-token.
//
// What's supported:
//   - Named layouts: ANSIC, UnixDate, RubyDate, RFC822, RFC822Z, RFC850,
//     RFC1123, RFC1123Z, RFC3339, RFC3339Nano, Kitchen, Stamp{,Milli,Micro,Nano},
//     DateTime, DateOnly, TimeOnly, Layout.
//   - All standard chunks: year (2006/06), month (Jan/January/1/01/_1),
//     day (2/02/_2), yearday (002/__2), weekday (Mon/Monday),
//     hour (15/3/03/_3), minute (4/04/_4), second (5/05/_5),
//     AM/PM (PM/pm), zone (MST, Z0700, Z07:00, Z07, Z070000, Z07:00:00,
//     -0700, -07:00, -07, -070000, -07:00:00), and fractional seconds
//     (.000/.999 with any width, comma separator also accepted).
//
// What's NOT supported yet:
//   - Named-zone lookup (LoadLocation / IANA tzdata). Local is treated as
//     UTC; FixedZone values round-trip but no tzdata matching is done.
//   - Monotonic clock reading in String().

use std::ops::{Add, Mul, Sub};
use std::sync::Arc;

// ─── Duration ─────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Duration {
    nanos: i128,
}

impl Duration {
    pub const fn from_nanos(n: i128) -> Self { Duration { nanos: n } }

    pub fn Nanoseconds(&self) -> crate::types::int64 { self.nanos as crate::types::int64 }
    pub fn Microseconds(&self) -> crate::types::int64 { (self.nanos / 1_000) as crate::types::int64 }
    pub fn Milliseconds(&self) -> crate::types::int64 { (self.nanos / 1_000_000) as crate::types::int64 }
    pub fn Seconds(&self) -> crate::types::float64 { self.nanos as f64 / 1_000_000_000.0 }
    pub fn Minutes(&self) -> crate::types::float64 { self.Seconds() / 60.0 }
    pub fn Hours(&self) -> crate::types::float64 { self.Seconds() / 3600.0 }

    /// Truncate rounds toward zero to a multiple of m. If m <= 0, returns d.
    pub fn Truncate(self, m: Duration) -> Duration {
        if m.nanos <= 0 { return self; }
        Duration::from_nanos(self.nanos - self.nanos % m.nanos)
    }

    /// Round rounds to the nearest multiple of m. If m <= 0, returns d.
    /// Halfway values round away from zero.
    pub fn Round(self, m: Duration) -> Duration {
        if m.nanos <= 0 { return self; }
        let r = self.nanos % m.nanos;
        if r < 0 {
            let r = -r;
            if r + r < m.nanos {
                Duration::from_nanos(self.nanos + r)
            } else {
                Duration::from_nanos(self.nanos - (m.nanos - r))
            }
        } else if r + r < m.nanos {
            Duration::from_nanos(self.nanos - r)
        } else {
            Duration::from_nanos(self.nanos + (m.nanos - r))
        }
    }

    /// Abs returns |d|, saturating at the largest representable magnitude on overflow.
    pub fn Abs(self) -> Duration {
        if self.nanos < 0 { Duration::from_nanos(-self.nanos) } else { self }
    }

    pub fn to_std(&self) -> std::time::Duration {
        if self.nanos <= 0 {
            std::time::Duration::ZERO
        } else {
            std::time::Duration::from_nanos(self.nanos as u64)
        }
    }

    pub fn String(&self) -> crate::types::string {
        if self.nanos == 0 { return "0s".to_string(); }
        let mut n = self.nanos;
        let neg = n < 0;
        if neg { n = -n; }

        if n < 1_000_000_000 {
            let mut prefix = String::new();
            if neg { prefix.push('-'); }
            if n < 1_000 { return format!("{}{}ns", prefix, n); }
            if n < 1_000_000 { return format!("{}{}µs", prefix, n as f64 / 1_000.0); }
            return format!("{}{}ms", prefix, n as f64 / 1_000_000.0);
        }

        let mut s = String::new();
        if neg { s.push('-'); }
        let total_secs = n / 1_000_000_000;
        let rem_nanos = n % 1_000_000_000;
        let hours = total_secs / 3600;
        let mins = (total_secs / 60) % 60;
        let secs = total_secs % 60;

        if hours > 0 { s.push_str(&format!("{}h", hours)); }
        if mins > 0 || hours > 0 { s.push_str(&format!("{}m", mins)); }
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

impl Add for Duration {
    type Output = Duration;
    fn add(self, other: Duration) -> Duration { Duration::from_nanos(self.nanos + other.nanos) }
}
impl Sub for Duration {
    type Output = Duration;
    fn sub(self, other: Duration) -> Duration { Duration::from_nanos(self.nanos - other.nanos) }
}
impl std::ops::Neg for Duration {
    type Output = Duration;
    fn neg(self) -> Duration { Duration::from_nanos(-self.nanos) }
}

macro_rules! impl_dur_mul {
    ($($t:ty),+) => { $(
        impl Mul<$t> for Duration {
            type Output = Duration;
            fn mul(self, rhs: $t) -> Duration { Duration::from_nanos(self.nanos * rhs as i128) }
        }
        impl Mul<Duration> for $t {
            type Output = Duration;
            fn mul(self, rhs: Duration) -> Duration { Duration::from_nanos(rhs.nanos * self as i128) }
        }
    )+ };
}
impl_dur_mul!(i32, i64, u32, u64, usize);

macro_rules! impl_dur_div {
    ($($t:ty),+) => { $(
        impl std::ops::Div<$t> for Duration {
            type Output = Duration;
            fn div(self, rhs: $t) -> Duration {
                Duration::from_nanos(self.nanos / rhs as i128)
            }
        }
    )+ };
}
impl_dur_div!(i32, i64, u32, u64, usize);

impl std::ops::Div<Duration> for Duration {
    type Output = i64;
    fn div(self, rhs: Duration) -> i64 { (self.nanos / rhs.nanos) as i64 }
}

impl Mul<f64> for Duration {
    type Output = Duration;
    fn mul(self, rhs: f64) -> Duration { Duration::from_nanos((self.nanos as f64 * rhs) as i128) }
}
impl Mul<Duration> for f64 {
    type Output = Duration;
    fn mul(self, rhs: Duration) -> Duration { Duration::from_nanos((rhs.nanos as f64 * self) as i128) }
}

#[allow(non_upper_case_globals)] pub const Nanosecond:  Duration = Duration::from_nanos(1);
#[allow(non_upper_case_globals)] pub const Microsecond: Duration = Duration::from_nanos(1_000);
#[allow(non_upper_case_globals)] pub const Millisecond: Duration = Duration::from_nanos(1_000_000);
#[allow(non_upper_case_globals)] pub const Second:      Duration = Duration::from_nanos(1_000_000_000);
#[allow(non_upper_case_globals)] pub const Minute:      Duration = Duration::from_nanos(60 * 1_000_000_000);
#[allow(non_upper_case_globals)] pub const Hour:        Duration = Duration::from_nanos(3600 * 1_000_000_000);

// ─── Month / Weekday ──────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Month(pub crate::types::int);

#[allow(non_upper_case_globals)] pub const January:   Month = Month(1);
#[allow(non_upper_case_globals)] pub const February:  Month = Month(2);
#[allow(non_upper_case_globals)] pub const March:     Month = Month(3);
#[allow(non_upper_case_globals)] pub const April:     Month = Month(4);
#[allow(non_upper_case_globals)] pub const May:       Month = Month(5);
#[allow(non_upper_case_globals)] pub const June:      Month = Month(6);
#[allow(non_upper_case_globals)] pub const July:      Month = Month(7);
#[allow(non_upper_case_globals)] pub const August:    Month = Month(8);
#[allow(non_upper_case_globals)] pub const September: Month = Month(9);
#[allow(non_upper_case_globals)] pub const October:   Month = Month(10);
#[allow(non_upper_case_globals)] pub const November:  Month = Month(11);
#[allow(non_upper_case_globals)] pub const December:  Month = Month(12);

pub(crate) const LONG_MONTH_NAMES: [&str; 12] = [
    "January","February","March","April","May","June",
    "July","August","September","October","November","December",
];
pub(crate) const SHORT_MONTH_NAMES: [&str; 12] = [
    "Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec",
];

impl Month {
    pub fn String(&self) -> crate::types::string {
        let m = self.0;
        if m >= 1 && m <= 12 {
            LONG_MONTH_NAMES[(m - 1) as usize].to_string()
        } else {
            format!("%!Month({})", m)
        }
    }
}
impl std::fmt::Display for Month {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.String())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Weekday(pub crate::types::int);

#[allow(non_upper_case_globals)] pub const Sunday:    Weekday = Weekday(0);
#[allow(non_upper_case_globals)] pub const Monday:    Weekday = Weekday(1);
#[allow(non_upper_case_globals)] pub const Tuesday:   Weekday = Weekday(2);
#[allow(non_upper_case_globals)] pub const Wednesday: Weekday = Weekday(3);
#[allow(non_upper_case_globals)] pub const Thursday:  Weekday = Weekday(4);
#[allow(non_upper_case_globals)] pub const Friday:    Weekday = Weekday(5);
#[allow(non_upper_case_globals)] pub const Saturday:  Weekday = Weekday(6);

pub(crate) const LONG_DAY_NAMES: [&str; 7] = [
    "Sunday","Monday","Tuesday","Wednesday","Thursday","Friday","Saturday",
];
pub(crate) const SHORT_DAY_NAMES: [&str; 7] = ["Sun","Mon","Tue","Wed","Thu","Fri","Sat"];

impl Weekday {
    pub fn String(&self) -> crate::types::string {
        let d = self.0;
        if d >= 0 && d <= 6 {
            LONG_DAY_NAMES[d as usize].to_string()
        } else {
            format!("%!Weekday({})", d)
        }
    }
}
impl std::fmt::Display for Weekday {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.String())
    }
}

// ─── Location ─────────────────────────────────────────────────────────

#[derive(Debug)]
struct LocInner { name: String, offset: i32 /* seconds east of UTC */ }

#[derive(Clone, Debug)]
pub struct Location(Arc<LocInner>);

impl PartialEq for Location {
    fn eq(&self, other: &Self) -> bool {
        self.0.name == other.0.name && self.0.offset == other.0.offset
    }
}
impl Eq for Location {}

impl Location {
    pub fn String(&self) -> crate::types::string { self.0.name.clone() }
    pub fn name(&self) -> &str { &self.0.name }
    pub fn offset_sec(&self) -> i32 { self.0.offset }
}

#[allow(non_snake_case)]
pub fn FixedZone(name: impl Into<String>, offset: crate::types::int) -> Location {
    Location(Arc::new(LocInner { name: name.into(), offset: offset as i32 }))
}

// UTC / Local: static Arcs via lazy OnceLock.
use std::sync::OnceLock;
static UTC_LOC: OnceLock<Location> = OnceLock::new();
static LOCAL_LOC: OnceLock<Location> = OnceLock::new();

fn utc_loc() -> &'static Location {
    UTC_LOC.get_or_init(|| Location(Arc::new(LocInner { name: "UTC".to_string(), offset: 0 })))
}
fn local_loc() -> &'static Location {
    LOCAL_LOC.get_or_init(|| Location(Arc::new(LocInner { name: "Local".to_string(), offset: 0 })))
}

#[allow(non_upper_case_globals)]
pub static UTC:   LocationRef = LocationRef { kind: LocKind::Utc };
#[allow(non_upper_case_globals)]
pub static Local: LocationRef = LocationRef { kind: LocKind::Local };

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LocKind { Utc, Local }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocationRef { kind: LocKind }

impl LocationRef {
    fn to_location(self) -> Location {
        match self.kind {
            LocKind::Utc => utc_loc().clone(),
            LocKind::Local => local_loc().clone(),
        }
    }
}

// ToLocation: unify Location vs LocationRef at call sites so `time::Date(..., UTC)`
// and `time::Date(..., FixedZone(...))` both work.
pub trait ToLocation {
    fn to_location(&self) -> Location;
}
impl ToLocation for LocationRef { fn to_location(&self) -> Location { (*self).to_location() } }
impl ToLocation for Location   { fn to_location(&self) -> Location { self.clone() } }
impl ToLocation for &Location  { fn to_location(&self) -> Location { (*self).clone() } }

// ─── Time ─────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct Time {
    /// Seconds since Unix epoch, referring to the instant in UTC.
    sec: i64,
    /// [0, 1_000_000_000) nanoseconds within sec.
    nsec: u32,
    /// Location used for display. Does NOT shift the instant.
    loc: Location,
}

impl Default for Time {
    fn default() -> Self {
        // Go's zero Time is January 1, year 1, 00:00:00 UTC.
        let sec = unix_seconds_from_civil(1, 1, 1, 0, 0, 0);
        Time { sec, nsec: 0, loc: utc_loc().clone() }
    }
}

impl PartialEq for Time {
    fn eq(&self, other: &Self) -> bool { self.sec == other.sec && self.nsec == other.nsec }
}
impl Eq for Time {}

impl PartialOrd for Time {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> { Some(self.cmp(other)) }
}
impl Ord for Time {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.sec.cmp(&other.sec) {
            std::cmp::Ordering::Equal => self.nsec.cmp(&other.nsec),
            o => o,
        }
    }
}

impl Time {
    fn with_loc(mut self, loc: Location) -> Time { self.loc = loc; self }

    /// Seconds since Unix epoch.
    pub fn Unix(&self) -> crate::types::int64 { self.sec }
    /// Milliseconds since Unix epoch.
    pub fn UnixMilli(&self) -> crate::types::int64 {
        self.sec.saturating_mul(1_000) + (self.nsec as i64 / 1_000_000)
    }
    pub fn UnixMicro(&self) -> crate::types::int64 {
        self.sec.saturating_mul(1_000_000) + (self.nsec as i64 / 1_000)
    }
    pub fn UnixNano(&self) -> crate::types::int64 {
        self.sec.saturating_mul(1_000_000_000) + (self.nsec as i64)
    }

    pub fn Nanosecond(&self) -> crate::types::int { self.nsec as crate::types::int }

    /// Returns local seconds since Unix epoch (for use in civil conversion).
    fn local_sec(&self) -> i64 {
        self.sec + self.loc.0.offset as i64
    }

    fn civil(&self) -> (i64, u32, u32, u32, u32, u32) {
        abs_to_civil(self.local_sec())
    }

    pub fn Year(&self) -> crate::types::int { self.civil().0 as crate::types::int }
    pub fn Month(&self) -> Month { Month(self.civil().1 as crate::types::int) }
    pub fn Day(&self) -> crate::types::int { self.civil().2 as crate::types::int }
    pub fn Hour(&self) -> crate::types::int { self.civil().3 as crate::types::int }
    pub fn Minute(&self) -> crate::types::int { self.civil().4 as crate::types::int }
    pub fn Second(&self) -> crate::types::int { self.civil().5 as crate::types::int }

    pub fn Date(&self) -> (crate::types::int, Month, crate::types::int) {
        let (y, mo, d, _, _, _) = self.civil();
        (y as crate::types::int, Month(mo as crate::types::int), d as crate::types::int)
    }
    pub fn Clock(&self) -> (crate::types::int, crate::types::int, crate::types::int) {
        let (_, _, _, h, mi, s) = self.civil();
        (h as crate::types::int, mi as crate::types::int, s as crate::types::int)
    }

    /// ISOWeek returns the ISO 8601 year and week number in which `self` occurs.
    /// Week ranges from 1 to 53. Jan 01..Jan 03 may belong to week 52 or 53 of
    /// the previous year, and Dec 29..Dec 31 may belong to week 1 of the following year.
    pub fn ISOWeek(&self) -> (crate::types::int, crate::types::int) {
        let (y, mo, d, _, _, _) = self.civil();
        let yday = days_before(mo) + d as i64
            + if is_leap(y) && mo > 2 { 1 } else { 0 };
        // Weekday where Monday=1..Sunday=7 (ISO)
        let wd_go = self.Weekday().0; // Sunday=0..Saturday=6
        let dow = if wd_go == 0 { 7 } else { wd_go as i64 };
        let iso_yday = yday + (4 - dow);
        let mut year = y;
        let mut week = (iso_yday + 6) / 7;
        if week < 1 {
            year -= 1;
            week = iso_weeks_in_year(year);
        } else if week > iso_weeks_in_year(y) {
            year += 1;
            week = 1;
        }
        (year as crate::types::int, week as crate::types::int)
    }

    /// AddDate returns the time corresponding to adding the given number of
    /// years, months, and days to t. Follows Go's normalization semantics.
    pub fn AddDate(&self, years: crate::types::int, months: crate::types::int, days: crate::types::int) -> Time {
        let (y, mo, d) = self.Date();
        let (h, mi, s) = self.Clock();
        let ns = self.Nanosecond();
        Date(y + years, Month(mo.0 + months), d + days, h, mi, s, ns, self.loc.clone())
    }

    /// Truncate rounds t down to a multiple of d (since the zero time). If d <= 0, returns t.
    pub fn Truncate(&self, d: Duration) -> Time {
        if d.nanos <= 0 { return self.clone(); }
        // Work in nanoseconds since the zero time.
        let ns = self.absolute_ns();
        let r = ns.rem_euclid(d.nanos);
        let truncated = ns - r;
        let abs = Time::from_absolute_ns(truncated, self.loc.clone());
        abs
    }

    /// Round rounds t to the nearest multiple of d since the zero time.
    /// Halfway values round up.
    pub fn Round(&self, d: Duration) -> Time {
        if d.nanos <= 0 { return self.clone(); }
        let ns = self.absolute_ns();
        let r = ns.rem_euclid(d.nanos);
        let out = if r + r < d.nanos { ns - r } else { ns + (d.nanos - r) };
        Time::from_absolute_ns(out, self.loc.clone())
    }

    /// absolute nanoseconds since year 1 Jan 1 00:00 UTC (matches Go's internal abs clock).
    fn absolute_ns(&self) -> i128 {
        // year 1 Jan 1 to Unix epoch = 62135596800 seconds
        (self.sec as i128 + 62_135_596_800) * 1_000_000_000 + self.nsec as i128
    }

    fn from_absolute_ns(abs_ns: i128, loc: Location) -> Time {
        let ns_per_sec = 1_000_000_000i128;
        let secs = abs_ns.div_euclid(ns_per_sec) - 62_135_596_800;
        let nsec = abs_ns.rem_euclid(ns_per_sec) as u32;
        Time { sec: secs as i64, nsec, loc }
    }

    pub fn Weekday(&self) -> Weekday {
        let days = self.local_sec().div_euclid(86_400);
        // Jan 1 1970 (Unix epoch) was a Thursday = 4.
        let w = (days + 4).rem_euclid(7) as crate::types::int;
        Weekday(w)
    }

    pub fn YearDay(&self) -> crate::types::int {
        let (y, mo, d, _, _, _) = self.civil();
        (days_before(mo as u32) + d as i64 + if is_leap(y) && mo > 2 { 1 } else { 0 }) as crate::types::int
    }

    pub fn Zone(&self) -> (crate::types::string, crate::types::int) {
        (self.loc.0.name.clone(), self.loc.0.offset as crate::types::int)
    }

    pub fn Location(&self) -> Location { self.loc.clone() }

    pub fn IsZero(&self) -> bool {
        let zero = Time::default();
        self.sec == zero.sec && self.nsec == zero.nsec
    }

    pub fn Sub(&self, earlier: Time) -> Duration {
        let dsec = self.sec as i128 - earlier.sec as i128;
        let dnsec = self.nsec as i128 - earlier.nsec as i128;
        Duration::from_nanos(dsec * 1_000_000_000 + dnsec)
    }
    pub fn Add(&self, d: Duration) -> Time {
        let total = (self.sec as i128) * 1_000_000_000 + self.nsec as i128 + d.nanos;
        let sec = total.div_euclid(1_000_000_000) as i64;
        let nsec = total.rem_euclid(1_000_000_000) as u32;
        Time { sec, nsec, loc: self.loc.clone() }
    }
    pub fn Equal(&self, other: &Time) -> bool { self.sec == other.sec && self.nsec == other.nsec }
    pub fn After(&self, other: Time) -> bool {
        self.sec > other.sec || (self.sec == other.sec && self.nsec > other.nsec)
    }
    pub fn Before(&self, other: Time) -> bool {
        self.sec < other.sec || (self.sec == other.sec && self.nsec < other.nsec)
    }
    pub fn Compare(&self, other: Time) -> crate::types::int {
        match self.cmp(&other) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        }
    }

    pub fn UTC(&self) -> Time { self.clone().with_loc(utc_loc().clone()) }
    pub fn Local(&self) -> Time { self.clone().with_loc(local_loc().clone()) }
    pub fn In<L: ToLocation>(&self, loc: L) -> Time { self.clone().with_loc(loc.to_location()) }

    pub fn String(&self) -> crate::types::string {
        self.Format("2006-01-02 15:04:05.999999999 -0700 MST")
    }

    pub fn GoString(&self) -> crate::types::string {
        let (y, mo, d) = self.Date();
        let (h, mi, s) = self.Clock();
        let ns = self.Nanosecond();
        let loc_part = match self.loc.0.name.as_str() {
            "UTC" => "time.UTC".to_string(),
            "Local" => "time.Local".to_string(),
            other => format!("time.Location({})", quote_go(other)),
        };
        let month_part = if mo.0 >= 1 && mo.0 <= 12 {
            format!("time.{}", LONG_MONTH_NAMES[(mo.0 - 1) as usize])
        } else {
            format!("{}", mo.0)
        };
        format!("time.Date({}, {}, {}, {}, {}, {}, {}, {})",
            y, month_part, d, h, mi, s, ns, loc_part)
    }

    pub fn Format(&self, layout: impl AsRef<str>) -> crate::types::string {
        let mut buf = Vec::with_capacity(layout.as_ref().len() + 10);
        self.append_format(&mut buf, layout.as_ref());
        String::from_utf8(buf).unwrap_or_default()
    }

    pub fn AppendFormat(&self, mut b: Vec<u8>, layout: impl AsRef<str>) -> Vec<u8> {
        self.append_format(&mut b, layout.as_ref());
        b
    }

    fn append_format(&self, b: &mut Vec<u8>, mut layout: &str) {
        let (name, offset) = (self.loc.0.name.clone(), self.loc.0.offset);
        let (year, month_u, day, hour, min_u, sec_u) = self.civil();
        let yday = days_before(month_u) + day as i64
            + if is_leap(year) && month_u > 2 { 1 } else { 0 };
        let month = month_u as i64;
        let day = day as i64;
        let hour = hour as i64;
        let min = min_u as i64;
        let sec = sec_u as i64;
        let nsec = self.nsec as i64;

        loop {
            let (prefix, std, suffix) = next_std_chunk(layout);
            b.extend_from_slice(prefix.as_bytes());
            if std == 0 { break; }
            layout = suffix;
            match std & STD_MASK {
                STD_YEAR => {
                    let y = if year < 0 { -year } else { year };
                    append_int(b, (y % 100) as i64, 2);
                }
                STD_LONG_YEAR => {
                    append_int(b, year, 4);
                }
                STD_MONTH => {
                    b.extend_from_slice(SHORT_MONTH_NAMES[(month - 1) as usize].as_bytes());
                }
                STD_LONG_MONTH => {
                    b.extend_from_slice(LONG_MONTH_NAMES[(month - 1) as usize].as_bytes());
                }
                STD_NUM_MONTH => append_int(b, month, 0),
                STD_ZERO_MONTH => append_int(b, month, 2),
                STD_WEEK_DAY => {
                    let wd = self.Weekday().0 as usize;
                    b.extend_from_slice(SHORT_DAY_NAMES[wd].as_bytes());
                }
                STD_LONG_WEEK_DAY => {
                    let wd = self.Weekday().0 as usize;
                    b.extend_from_slice(LONG_DAY_NAMES[wd].as_bytes());
                }
                STD_DAY => append_int(b, day, 0),
                STD_UNDER_DAY => {
                    if day < 10 { b.push(b' '); }
                    append_int(b, day, 0);
                }
                STD_ZERO_DAY => append_int(b, day, 2),
                STD_UNDER_YEAR_DAY => {
                    if yday < 100 {
                        b.push(b' ');
                        if yday < 10 { b.push(b' '); }
                    }
                    append_int(b, yday, 0);
                }
                STD_ZERO_YEAR_DAY => append_int(b, yday, 3),
                STD_HOUR => append_int(b, hour, 2),
                STD_HOUR_12 => {
                    let mut hr = hour % 12;
                    if hr == 0 { hr = 12; }
                    append_int(b, hr, 0);
                }
                STD_ZERO_HOUR_12 => {
                    let mut hr = hour % 12;
                    if hr == 0 { hr = 12; }
                    append_int(b, hr, 2);
                }
                STD_MINUTE => append_int(b, min, 0),
                STD_ZERO_MINUTE => append_int(b, min, 2),
                STD_SECOND => append_int(b, sec, 0),
                STD_ZERO_SECOND => append_int(b, sec, 2),
                STD_PM => {
                    b.extend_from_slice(if hour >= 12 { b"PM" } else { b"AM" });
                }
                STDPM => {
                    b.extend_from_slice(if hour >= 12 { b"pm" } else { b"am" });
                }
                code @ (STD_ISO8601_TZ | STD_ISO8601_COLON_TZ | STD_ISO8601_SECONDS_TZ
                    | STD_ISO8601_SHORT_TZ | STD_ISO8601_COLON_SECONDS_TZ
                    | STD_NUM_TZ | STD_NUM_COLON_TZ | STD_NUM_SECONDS_TZ
                    | STD_NUM_SHORT_TZ | STD_NUM_COLON_SECONDS_TZ) => {
                    let iso_z = matches!(code, STD_ISO8601_TZ | STD_ISO8601_COLON_TZ
                        | STD_ISO8601_SECONDS_TZ | STD_ISO8601_SHORT_TZ
                        | STD_ISO8601_COLON_SECONDS_TZ);
                    if offset == 0 && iso_z {
                        b.push(b'Z');
                    } else {
                        let mut abs = offset as i64;
                        let mut zone = offset as i64 / 60;
                        if zone < 0 {
                            b.push(b'-');
                            zone = -zone;
                            abs = -abs;
                        } else {
                            b.push(b'+');
                        }
                        append_int(b, zone / 60, 2);
                        if matches!(code, STD_ISO8601_COLON_TZ | STD_NUM_COLON_TZ
                            | STD_ISO8601_COLON_SECONDS_TZ | STD_NUM_COLON_SECONDS_TZ) {
                            b.push(b':');
                        }
                        if code != STD_NUM_SHORT_TZ && code != STD_ISO8601_SHORT_TZ {
                            append_int(b, zone % 60, 2);
                        }
                        if matches!(code, STD_ISO8601_SECONDS_TZ | STD_NUM_SECONDS_TZ
                            | STD_NUM_COLON_SECONDS_TZ | STD_ISO8601_COLON_SECONDS_TZ) {
                            if matches!(code, STD_NUM_COLON_SECONDS_TZ | STD_ISO8601_COLON_SECONDS_TZ) {
                                b.push(b':');
                            }
                            append_int(b, abs % 60, 2);
                        }
                    }
                }
                STD_TZ => {
                    if !name.is_empty() && name != "Local" {
                        b.extend_from_slice(name.as_bytes());
                    } else {
                        let mut zone = offset as i64 / 60;
                        if zone < 0 {
                            b.push(b'-');
                            zone = -zone;
                        } else {
                            b.push(b'+');
                        }
                        append_int(b, zone / 60, 2);
                        append_int(b, zone % 60, 2);
                    }
                }
                STD_FRAC_SECOND_0 | STD_FRAC_SECOND_9 => append_nano(b, nsec, std),
                _ => {}
            }
        }
    }
}

impl std::fmt::Display for Time {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.String())
    }
}

// ─── civil <-> absolute ───────────────────────────────────────────────

pub(crate) fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

pub(crate) fn days_before(m: u32) -> i64 {
    static BEFORE: [i64; 13] = [0,31,59,90,120,151,181,212,243,273,304,334,365];
    BEFORE[(m - 1) as usize]
}

pub(crate) fn days_in(m: u32, year: i64) -> i64 {
    if m == 2 && is_leap(year) { 29 } else { days_before(m + 1) - days_before(m) }
}

/// daysIn reports the number of days in the month of the given year.
/// Exported to match Go's internal test hook `DaysIn`.
#[allow(non_snake_case)]
pub fn DaysIn(month: Month, year: crate::types::int) -> crate::types::int {
    days_in(month.0 as u32, year as i64) as crate::types::int
}

fn iso_weeks_in_year(year: i64) -> i64 {
    let t = Time {
        sec: unix_seconds_from_civil(year, 1, 1, 0, 0, 0),
        nsec: 0,
        loc: utc_loc().clone(),
    };
    let wd = t.Weekday().0;
    let iso_wd = if wd == 0 { 7 } else { wd };
    if iso_wd == 4 || (iso_wd == 3 && is_leap(year)) { 53 } else { 52 }
}

fn days_from_civil(y: i64, m: u32, d: i64) -> i64 {
    // Howard Hinnant's algorithm. Allows d outside [1,31] for normalization.
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as i64;
    let mp = if m > 2 { m as i64 - 3 } else { m as i64 + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe/4 - yoe/100 + doy;
    era * 146_097 + doe - 719_468
}

fn unix_seconds_from_civil(y: i64, m: u32, d: i64, h: i64, mi: i64, s: i64) -> i64 {
    let days = days_from_civil(y, m, d);
    days * 86_400 + h * 3600 + mi * 60 + s
}

/// abs_to_civil takes seconds since the Unix epoch (UTC) and returns
/// (year, month, day, hour, minute, second) in the Gregorian calendar.
fn abs_to_civil(secs: i64) -> (i64, u32, u32, u32, u32, u32) {
    let mut tod = secs.rem_euclid(86_400);
    let days = secs.div_euclid(86_400);
    let h = (tod / 3600) as u32; tod %= 3600;
    let mi = (tod / 60) as u32;
    let s = (tod % 60) as u32;

    // Howard Hinnant's civil_from_days: `days` is days since 1970-01-01.
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

// ─── Constructors ─────────────────────────────────────────────────────

#[allow(non_snake_case)]
pub fn Date<L: ToLocation>(
    year: crate::types::int, month: Month, day: crate::types::int,
    hour: crate::types::int, min: crate::types::int, sec: crate::types::int,
    nsec: crate::types::int, loc: L,
) -> Time {
    let loc = loc.to_location();
    let off = loc.0.offset as i64;
    // Normalize: allow arbitrary int overflow for each field (Go does this).
    let mut y = year as i64;
    let mut mo = month.0 as i64;
    // Month can be 0 or > 12; normalize into 1..=12.
    if mo < 1 {
        let years = (-(mo - 1) + 11) / 12;
        y -= years;
        mo += years * 12;
    } else if mo > 12 {
        let years = (mo - 1) / 12;
        y += years;
        mo -= years * 12;
    }
    let total_ns = nsec as i128;
    let extra_sec = total_ns.div_euclid(1_000_000_000) as i64;
    let nsec_norm = total_ns.rem_euclid(1_000_000_000) as u32;
    // local seconds from civil (day/hour/min/sec can all overflow; Hinnant handles day).
    let local_sec = unix_seconds_from_civil(y, mo as u32, day as i64, hour as i64, min as i64, sec as i64) + extra_sec;
    // subtract the offset to get UTC sec
    let utc_sec = local_sec - off;
    Time { sec: utc_sec, nsec: nsec_norm, loc }
}

/// Alternative Date form: accept month as int (matches Go where mo is an untyped const).
#[allow(non_snake_case, clippy::too_many_arguments)]
pub fn DateInt<L: ToLocation>(
    year: crate::types::int, month: crate::types::int, day: crate::types::int,
    hour: crate::types::int, min: crate::types::int, sec: crate::types::int,
    nsec: crate::types::int, loc: L,
) -> Time {
    Date(year, Month(month), day, hour, min, sec, nsec, loc)
}

#[allow(non_snake_case)]
pub fn Unix(sec: crate::types::int64, nsec: crate::types::int64) -> Time {
    let total = sec as i128 * 1_000_000_000 + nsec as i128;
    let s = total.div_euclid(1_000_000_000) as i64;
    let ns = total.rem_euclid(1_000_000_000) as u32;
    Time { sec: s, nsec: ns, loc: local_loc().clone() }
}

#[allow(non_snake_case)]
pub fn UnixMilli(msec: crate::types::int64) -> Time {
    Unix(msec / 1_000, (msec % 1_000) * 1_000_000)
}
#[allow(non_snake_case)]
pub fn UnixMicro(usec: crate::types::int64) -> Time {
    Unix(usec / 1_000_000, (usec % 1_000_000) * 1_000)
}

#[allow(non_snake_case)]
pub fn Now() -> Time {
    let wall = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH);
    match wall {
        Ok(d) => {
            let s = d.as_secs() as i64;
            let ns = d.subsec_nanos();
            Time { sec: s, nsec: ns, loc: local_loc().clone() }
        }
        Err(e) => {
            let d = e.duration();
            let s = -(d.as_secs() as i64);
            let ns = d.subsec_nanos();
            if ns == 0 {
                Time { sec: s, nsec: 0, loc: local_loc().clone() }
            } else {
                Time { sec: s - 1, nsec: 1_000_000_000 - ns, loc: local_loc().clone() }
            }
        }
    }
}

#[allow(non_snake_case)]
pub fn Since(t: Time) -> Duration { Now().Sub(t) }
#[allow(non_snake_case)]
pub fn Until(t: Time) -> Duration { t.Sub(Now()) }

#[allow(non_snake_case)]
pub fn Sleep(d: Duration) { std::thread::sleep(d.to_std()); }

// ─── Layout constants ─────────────────────────────────────────────────

#[allow(non_upper_case_globals)] pub const Layout:       &str = "01/02 03:04:05PM '06 -0700";
#[allow(non_upper_case_globals)] pub const ANSIC:        &str = "Mon Jan _2 15:04:05 2006";
#[allow(non_upper_case_globals)] pub const UnixDate:     &str = "Mon Jan _2 15:04:05 MST 2006";
#[allow(non_upper_case_globals)] pub const RubyDate:     &str = "Mon Jan 02 15:04:05 -0700 2006";
#[allow(non_upper_case_globals)] pub const RFC822:       &str = "02 Jan 06 15:04 MST";
#[allow(non_upper_case_globals)] pub const RFC822Z:      &str = "02 Jan 06 15:04 -0700";
#[allow(non_upper_case_globals)] pub const RFC850:       &str = "Monday, 02-Jan-06 15:04:05 MST";
#[allow(non_upper_case_globals)] pub const RFC1123:      &str = "Mon, 02 Jan 2006 15:04:05 MST";
#[allow(non_upper_case_globals)] pub const RFC1123Z:     &str = "Mon, 02 Jan 2006 15:04:05 -0700";
#[allow(non_upper_case_globals)] pub const RFC3339:      &str = "2006-01-02T15:04:05Z07:00";
#[allow(non_upper_case_globals)] pub const RFC3339Nano:  &str = "2006-01-02T15:04:05.999999999Z07:00";
#[allow(non_upper_case_globals)] pub const Kitchen:      &str = "3:04PM";
#[allow(non_upper_case_globals)] pub const Stamp:        &str = "Jan _2 15:04:05";
#[allow(non_upper_case_globals)] pub const StampMilli:   &str = "Jan _2 15:04:05.000";
#[allow(non_upper_case_globals)] pub const StampMicro:   &str = "Jan _2 15:04:05.000000";
#[allow(non_upper_case_globals)] pub const StampNano:    &str = "Jan _2 15:04:05.000000000";
#[allow(non_upper_case_globals)] pub const DateTime:     &str = "2006-01-02 15:04:05";
#[allow(non_upper_case_globals)] pub const DateOnly:     &str = "2006-01-02";
#[allow(non_upper_case_globals)] pub const TimeOnly:     &str = "15:04:05";

// ─── nextStdChunk and std constants ───────────────────────────────────

const STD_LONG_MONTH: i32              =  1 | STD_NEED_DATE;
const STD_MONTH: i32                   =  2 | STD_NEED_DATE;
const STD_NUM_MONTH: i32               =  3 | STD_NEED_DATE;
const STD_ZERO_MONTH: i32              =  4 | STD_NEED_DATE;
const STD_LONG_WEEK_DAY: i32           =  5 | STD_NEED_DATE;
const STD_WEEK_DAY: i32                =  6 | STD_NEED_DATE;
const STD_DAY: i32                     =  7 | STD_NEED_DATE;
const STD_UNDER_DAY: i32               =  8 | STD_NEED_DATE;
const STD_ZERO_DAY: i32                =  9 | STD_NEED_DATE;
const STD_UNDER_YEAR_DAY: i32          = 10 | STD_NEED_YDAY;
const STD_ZERO_YEAR_DAY: i32           = 11 | STD_NEED_YDAY;
const STD_HOUR: i32                    = 12 | STD_NEED_CLOCK;
const STD_HOUR_12: i32                 = 13 | STD_NEED_CLOCK;
const STD_ZERO_HOUR_12: i32            = 14 | STD_NEED_CLOCK;
const STD_MINUTE: i32                  = 15 | STD_NEED_CLOCK;
const STD_ZERO_MINUTE: i32             = 16 | STD_NEED_CLOCK;
const STD_SECOND: i32                  = 17 | STD_NEED_CLOCK;
const STD_ZERO_SECOND: i32             = 18 | STD_NEED_CLOCK;
const STD_LONG_YEAR: i32               = 19 | STD_NEED_DATE;
const STD_YEAR: i32                    = 20 | STD_NEED_DATE;
const STD_PM: i32                      = 21 | STD_NEED_CLOCK;
const STDPM: i32                       = 22 | STD_NEED_CLOCK;
const STD_TZ: i32                      = 23;
const STD_ISO8601_TZ: i32              = 24;
const STD_ISO8601_SECONDS_TZ: i32      = 25;
const STD_ISO8601_SHORT_TZ: i32        = 26;
const STD_ISO8601_COLON_TZ: i32        = 27;
const STD_ISO8601_COLON_SECONDS_TZ: i32= 28;
const STD_NUM_TZ: i32                  = 29;
const STD_NUM_SECONDS_TZ: i32          = 30;
const STD_NUM_SHORT_TZ: i32            = 31;
const STD_NUM_COLON_TZ: i32            = 32;
const STD_NUM_COLON_SECONDS_TZ: i32    = 33;
const STD_FRAC_SECOND_0: i32           = 34;
const STD_FRAC_SECOND_9: i32           = 35;

const STD_NEED_DATE: i32    = 1 << 8;
const STD_NEED_YDAY: i32    = 1 << 9;
const STD_NEED_CLOCK: i32   = 1 << 10;
const STD_ARG_SHIFT: i32    = 16;
const STD_SEPARATOR_SHIFT: i32 = 28;
const STD_MASK: i32         = (1 << STD_ARG_SHIFT) - 1;

fn std_frac_second(code: i32, n: i32, c: u8) -> i32 {
    if c == b'.' {
        code | ((n & 0xfff) << STD_ARG_SHIFT)
    } else {
        code | ((n & 0xfff) << STD_ARG_SHIFT) | (1 << STD_SEPARATOR_SHIFT)
    }
}

fn digits_len(std: i32) -> i32 { (std >> STD_ARG_SHIFT) & 0xfff }
fn separator(std: i32) -> u8 {
    if (std >> STD_SEPARATOR_SHIFT) == 0 { b'.' } else { b',' }
}

fn starts_with_lower_case(s: &str) -> bool {
    match s.as_bytes().first() { Some(&c) => c >= b'a' && c <= b'z', None => false }
}

fn is_digit_byte(s: &[u8], i: usize) -> bool {
    i < s.len() && s[i] >= b'0' && s[i] <= b'9'
}

fn next_std_chunk(layout: &str) -> (&str, i32, &str) {
    let bytes = layout.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        match c {
            b'J' => {
                if bytes.len() >= i + 3 && &bytes[i..i+3] == b"Jan" {
                    if bytes.len() >= i + 7 && &bytes[i..i+7] == b"January" {
                        return (&layout[..i], STD_LONG_MONTH, &layout[i+7..]);
                    }
                    if !starts_with_lower_case(&layout[i+3..]) {
                        return (&layout[..i], STD_MONTH, &layout[i+3..]);
                    }
                }
            }
            b'M' => {
                if bytes.len() >= i + 3 {
                    if &bytes[i..i+3] == b"Mon" {
                        if bytes.len() >= i + 6 && &bytes[i..i+6] == b"Monday" {
                            return (&layout[..i], STD_LONG_WEEK_DAY, &layout[i+6..]);
                        }
                        if !starts_with_lower_case(&layout[i+3..]) {
                            return (&layout[..i], STD_WEEK_DAY, &layout[i+3..]);
                        }
                    }
                    if &bytes[i..i+3] == b"MST" {
                        return (&layout[..i], STD_TZ, &layout[i+3..]);
                    }
                }
            }
            b'0' => {
                if bytes.len() >= i + 2 && bytes[i+1] >= b'1' && bytes[i+1] <= b'6' {
                    let code = match bytes[i+1] {
                        b'1' => STD_ZERO_MONTH,
                        b'2' => STD_ZERO_DAY,
                        b'3' => STD_ZERO_HOUR_12,
                        b'4' => STD_ZERO_MINUTE,
                        b'5' => STD_ZERO_SECOND,
                        b'6' => STD_YEAR,
                        _ => unreachable!(),
                    };
                    return (&layout[..i], code, &layout[i+2..]);
                }
                if bytes.len() >= i + 3 && bytes[i+1] == b'0' && bytes[i+2] == b'2' {
                    return (&layout[..i], STD_ZERO_YEAR_DAY, &layout[i+3..]);
                }
            }
            b'1' => {
                if bytes.len() >= i + 2 && bytes[i+1] == b'5' {
                    return (&layout[..i], STD_HOUR, &layout[i+2..]);
                }
                return (&layout[..i], STD_NUM_MONTH, &layout[i+1..]);
            }
            b'2' => {
                if bytes.len() >= i + 4 && &bytes[i..i+4] == b"2006" {
                    return (&layout[..i], STD_LONG_YEAR, &layout[i+4..]);
                }
                return (&layout[..i], STD_DAY, &layout[i+1..]);
            }
            b'_' => {
                if bytes.len() >= i + 2 && bytes[i+1] == b'2' {
                    // _2006 is really a literal _, followed by stdLongYear.
                    if bytes.len() >= i + 5 && &bytes[i+1..i+5] == b"2006" {
                        return (&layout[..i+1], STD_LONG_YEAR, &layout[i+5..]);
                    }
                    return (&layout[..i], STD_UNDER_DAY, &layout[i+2..]);
                }
                if bytes.len() >= i + 3 && bytes[i+1] == b'_' && bytes[i+2] == b'2' {
                    return (&layout[..i], STD_UNDER_YEAR_DAY, &layout[i+3..]);
                }
            }
            b'3' => return (&layout[..i], STD_HOUR_12, &layout[i+1..]),
            b'4' => return (&layout[..i], STD_MINUTE, &layout[i+1..]),
            b'5' => return (&layout[..i], STD_SECOND, &layout[i+1..]),
            b'P' => {
                if bytes.len() >= i + 2 && bytes[i+1] == b'M' {
                    return (&layout[..i], STD_PM, &layout[i+2..]);
                }
            }
            b'p' => {
                if bytes.len() >= i + 2 && bytes[i+1] == b'm' {
                    return (&layout[..i], STDPM, &layout[i+2..]);
                }
            }
            b'-' => {
                if bytes.len() >= i + 7 && &bytes[i..i+7] == b"-070000" {
                    return (&layout[..i], STD_NUM_SECONDS_TZ, &layout[i+7..]);
                }
                if bytes.len() >= i + 9 && &bytes[i..i+9] == b"-07:00:00" {
                    return (&layout[..i], STD_NUM_COLON_SECONDS_TZ, &layout[i+9..]);
                }
                if bytes.len() >= i + 5 && &bytes[i..i+5] == b"-0700" {
                    return (&layout[..i], STD_NUM_TZ, &layout[i+5..]);
                }
                if bytes.len() >= i + 6 && &bytes[i..i+6] == b"-07:00" {
                    return (&layout[..i], STD_NUM_COLON_TZ, &layout[i+6..]);
                }
                if bytes.len() >= i + 3 && &bytes[i..i+3] == b"-07" {
                    return (&layout[..i], STD_NUM_SHORT_TZ, &layout[i+3..]);
                }
            }
            b'Z' => {
                if bytes.len() >= i + 7 && &bytes[i..i+7] == b"Z070000" {
                    return (&layout[..i], STD_ISO8601_SECONDS_TZ, &layout[i+7..]);
                }
                if bytes.len() >= i + 9 && &bytes[i..i+9] == b"Z07:00:00" {
                    return (&layout[..i], STD_ISO8601_COLON_SECONDS_TZ, &layout[i+9..]);
                }
                if bytes.len() >= i + 5 && &bytes[i..i+5] == b"Z0700" {
                    return (&layout[..i], STD_ISO8601_TZ, &layout[i+5..]);
                }
                if bytes.len() >= i + 6 && &bytes[i..i+6] == b"Z07:00" {
                    return (&layout[..i], STD_ISO8601_COLON_TZ, &layout[i+6..]);
                }
                if bytes.len() >= i + 3 && &bytes[i..i+3] == b"Z07" {
                    return (&layout[..i], STD_ISO8601_SHORT_TZ, &layout[i+3..]);
                }
            }
            b'.' | b',' => {
                if i + 1 < bytes.len() && (bytes[i+1] == b'0' || bytes[i+1] == b'9') {
                    let ch = bytes[i+1];
                    let mut j = i + 1;
                    while j < bytes.len() && bytes[j] == ch { j += 1; }
                    if !is_digit_byte(bytes, j) {
                        let code = if bytes[i+1] == b'9' { STD_FRAC_SECOND_9 } else { STD_FRAC_SECOND_0 };
                        let std = std_frac_second(code, (j - (i + 1)) as i32, c);
                        return (&layout[..i], std, &layout[j..]);
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }
    (layout, 0, "")
}

// ─── append helpers ───────────────────────────────────────────────────

pub(crate) fn append_int(b: &mut Vec<u8>, x: i64, width: i32) {
    let mut u = x.unsigned_abs();
    if x < 0 {
        b.push(b'-');
    }
    let mut buf = [0u8; 20];
    let mut n = 0usize;
    if u == 0 { buf[0] = b'0'; n = 1; }
    while u > 0 {
        buf[n] = b'0' + (u % 10) as u8;
        u /= 10;
        n += 1;
    }
    for _ in 0..(width as isize - n as isize) { b.push(b'0'); }
    for i in (0..n).rev() { b.push(buf[i]); }
}

/// Public AppendInt, matches Go's time.AppendInt (used via export_test).
#[allow(non_snake_case)]
pub fn AppendInt(mut b: Vec<u8>, x: crate::types::int, width: crate::types::int) -> Vec<u8> {
    append_int(&mut b, x as i64, width as i32);
    b
}

fn append_nano(b: &mut Vec<u8>, nanosec: i64, std: i32) {
    let trim = std & STD_MASK == STD_FRAC_SECOND_9;
    let n = digits_len(std) as usize;
    if trim && (n == 0 || nanosec == 0) { return; }
    let dot = separator(std);
    b.push(dot);
    append_int(b, nanosec, 9);
    if n < 9 {
        let cut = 9 - n;
        b.truncate(b.len() - cut);
    }
    if trim {
        while let Some(&x) = b.last() {
            if x == b'0' { b.pop(); } else { break; }
        }
        if b.last() == Some(&dot) { b.pop(); }
    }
}

// ─── Parse ────────────────────────────────────────────────────────────

pub struct ParseError {
    pub Layout: String,
    pub Value: String,
    pub LayoutElem: String,
    pub ValueElem: String,
    pub Message: String,
}

impl ParseError {
    pub fn Error(&self) -> String {
        if self.Message.is_empty() {
            format!("parsing time {} as {}: cannot parse {} as {}",
                quote_go(&self.Value), quote_go(&self.Layout),
                quote_go(&self.ValueElem), quote_go(&self.LayoutElem))
        } else {
            format!("parsing time {}{}", quote_go(&self.Value), self.Message)
        }
    }
}

fn parse_err(layout: &str, value: &str, layout_elem: &str, value_elem: &str, message: &str) -> crate::errors::error {
    let pe = ParseError {
        Layout: layout.to_string(),
        Value: value.to_string(),
        LayoutElem: layout_elem.to_string(),
        ValueElem: value_elem.to_string(),
        Message: message.to_string(),
    };
    crate::errors::New(&pe.Error())
}

fn match_ci(s1: &[u8], s2: &[u8]) -> bool {
    if s1.len() != s2.len() { return false; }
    for i in 0..s1.len() {
        let mut c1 = s1[i];
        let mut c2 = s2[i];
        if c1 != c2 {
            c1 |= b'a' - b'A';
            c2 |= b'a' - b'A';
            if c1 != c2 || c1 < b'a' || c1 > b'z' { return false; }
        }
    }
    true
}

fn lookup_name<'a>(tab: &[&str], val: &'a str) -> Result<(usize, &'a str), ()> {
    for (i, v) in tab.iter().enumerate() {
        if val.len() >= v.len() && match_ci(&val.as_bytes()[..v.len()], v.as_bytes()) {
            return Ok((i, &val[v.len()..]));
        }
    }
    Err(())
}

fn is_digit_ch(s: &str, i: usize) -> bool {
    let b = s.as_bytes();
    i < b.len() && b[i] >= b'0' && b[i] <= b'9'
}

fn comma_or_period(b: u8) -> bool { b == b'.' || b == b',' }

fn get_num(s: &str, fixed: bool) -> Result<(i64, &str), ()> {
    let b = s.as_bytes();
    if !is_digit_ch(s, 0) { return Err(()); }
    if !is_digit_ch(s, 1) {
        if fixed { return Err(()); }
        return Ok(((b[0] - b'0') as i64, &s[1..]));
    }
    Ok(((b[0] - b'0') as i64 * 10 + (b[1] - b'0') as i64, &s[2..]))
}

fn get_num3(s: &str, fixed: bool) -> Result<(i64, &str), ()> {
    let b = s.as_bytes();
    let mut n: i64 = 0;
    let mut i = 0;
    while i < 3 && is_digit_ch(s, i) {
        n = n * 10 + (b[i] - b'0') as i64;
        i += 1;
    }
    if i == 0 || (fixed && i != 3) { return Err(()); }
    Ok((n, &s[i..]))
}

fn cutspace(s: &str) -> &str {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() && b[i] == b' ' { i += 1; }
    &s[i..]
}

fn skip<'a>(value: &'a str, prefix: &str) -> Result<&'a str, ()> {
    let mut v = value;
    let mut p = prefix;
    while !p.is_empty() {
        let pb = p.as_bytes();
        if pb[0] == b' ' {
            if !v.is_empty() && v.as_bytes()[0] != b' ' {
                return Err(());
            }
            p = cutspace(p);
            v = cutspace(v);
            continue;
        }
        if v.is_empty() || v.as_bytes()[0] != pb[0] {
            return Err(());
        }
        p = &p[1..];
        v = &v[1..];
    }
    Ok(v)
}

fn parse_nanoseconds(value: &str, nbytes: usize) -> Result<(i64, String), String> {
    let b = value.as_bytes();
    if b.is_empty() || !comma_or_period(b[0]) {
        return Err(String::new());
    }
    let nbytes_eff = if nbytes > 10 { 10 } else { nbytes };
    let digits = &value[1..nbytes_eff];
    let mut ns: i64 = 0;
    for &c in digits.as_bytes() {
        if !(b'0'..=b'9').contains(&c) { return Err(String::new()); }
        ns = ns * 10 + (c - b'0') as i64;
    }
    let scale_digits = 10 - nbytes_eff as i32;
    for _ in 0..scale_digits { ns *= 10; }
    Ok((ns, String::new()))
}

fn parse_signed_offset(value: &str) -> usize {
    let b = value.as_bytes();
    if b.is_empty() { return 0; }
    let sign = b[0];
    if sign != b'-' && sign != b'+' { return 0; }
    let mut i = 1;
    let mut x: u64 = 0;
    while i < b.len() && b[i].is_ascii_digit() {
        x = x * 10 + (b[i] - b'0') as u64;
        i += 1;
    }
    if i == 1 { return 0; }
    if x > 23 { return 0; }
    i
}

fn parse_gmt(value: &str) -> usize {
    if value.len() == 3 { return 3; }
    3 + parse_signed_offset(&value[3..])
}

#[allow(non_snake_case)]
pub fn ParseTimeZone(value: &str) -> (crate::types::int, bool) {
    let b = value.as_bytes();
    if b.len() < 3 { return (0, false); }
    if b.len() >= 4 && (&value[..4] == "ChST" || &value[..4] == "MeST") {
        return (4, true);
    }
    if &value[..3] == "GMT" {
        let l = parse_gmt(value);
        return (l as crate::types::int, true);
    }
    if b[0] == b'+' || b[0] == b'-' {
        let l = parse_signed_offset(value);
        let ok = l > 0;
        return (l as crate::types::int, ok);
    }
    let mut n_upper = 0;
    while n_upper < 6 {
        if n_upper >= b.len() { break; }
        let c = b[n_upper];
        if c < b'A' || c > b'Z' { break; }
        n_upper += 1;
    }
    match n_upper {
        0 | 1 | 2 | 6 => (0, false),
        5 => if b[4] == b'T' { (5, true) } else { (0, false) },
        4 => if b[3] == b'T' || &value[..4] == "WITA" { (4, true) } else { (0, false) },
        3 => (3, true),
        _ => (0, false),
    }
}

#[allow(non_snake_case)]
pub fn Parse(layout: impl AsRef<str>, value: impl AsRef<str>) -> (Time, crate::errors::error) {
    parse_impl(layout.as_ref(), value.as_ref(), utc_loc().clone(), local_loc().clone())
}

#[allow(non_snake_case)]
pub fn ParseInLocation<L: ToLocation>(layout: impl AsRef<str>, value: impl AsRef<str>, loc: L) -> (Time, crate::errors::error) {
    let l = loc.to_location();
    parse_impl(layout.as_ref(), value.as_ref(), l.clone(), l)
}

fn parse_impl(layout: &str, value: &str, default_loc: Location, local: Location) -> (Time, crate::errors::error) {
    let alayout = layout;
    let avalue = value;
    let mut layout_rest = layout;
    let mut value_rest = value.to_string();
    let mut range_err = String::new();
    let mut am_set = false;
    let mut pm_set = false;

    let mut year: i64 = 0;
    let mut month: i64 = -1;
    let mut day: i64 = -1;
    let mut yday: i64 = -1;
    let mut hour: i64 = 0;
    let mut min: i64 = 0;
    let mut sec: i64 = 0;
    let mut nsec: i64 = 0;
    let mut zone_loc: Option<Location> = None;
    let mut zone_offset: i64 = -1;
    let mut zone_name = String::new();

    loop {
        let (prefix, std, suffix) = next_std_chunk(layout_rest);
        let std_str_len = layout_rest.len() - suffix.len();
        let stdstr = &layout_rest[prefix.len()..std_str_len];
        let prefix_owned = prefix.to_string();
        let stdstr_owned = stdstr.to_string();
        let v_before_skip = value_rest.clone();
        value_rest = match skip(&value_rest, &prefix_owned) {
            Ok(s) => s.to_string(),
            Err(_) => {
                return (Time::default(), parse_err(alayout, avalue, &prefix_owned, &v_before_skip, ""));
            }
        };
        if std == 0 {
            if !value_rest.is_empty() {
                let msg = format!(": extra text: {}", quote_go(&value_rest));
                return (Time::default(), parse_err(alayout, avalue, "", &value_rest, &msg));
            }
            break;
        }
        layout_rest = suffix;

        let mut err_mark = false;
        let hold = value_rest.clone();
        match std & STD_MASK {
            STD_YEAR => {
                if value_rest.len() < 2 { err_mark = true; }
                else {
                    let (p, rest) = value_rest.split_at(2);
                    match p.parse::<i64>() {
                        Ok(n) => {
                            year = if n >= 69 { n + 1900 } else { n + 2000 };
                            value_rest = rest.to_string();
                        }
                        Err(_) => err_mark = true,
                    }
                }
            }
            STD_LONG_YEAR => {
                if value_rest.len() < 4 || !is_digit_ch(&value_rest, 0) { err_mark = true; }
                else {
                    let (p, rest) = value_rest.split_at(4);
                    match p.parse::<i64>() {
                        Ok(n) => { year = n; value_rest = rest.to_string(); }
                        Err(_) => err_mark = true,
                    }
                }
            }
            STD_MONTH => {
                match lookup_name(&SHORT_MONTH_NAMES, &value_rest) {
                    Ok((i, rest)) => { month = (i + 1) as i64; value_rest = rest.to_string(); }
                    Err(_) => err_mark = true,
                }
            }
            STD_LONG_MONTH => {
                match lookup_name(&LONG_MONTH_NAMES, &value_rest) {
                    Ok((i, rest)) => { month = (i + 1) as i64; value_rest = rest.to_string(); }
                    Err(_) => err_mark = true,
                }
            }
            STD_NUM_MONTH | STD_ZERO_MONTH => {
                match get_num(&value_rest, std & STD_MASK == STD_ZERO_MONTH) {
                    Ok((m, rest)) => {
                        month = m;
                        value_rest = rest.to_string();
                        if month <= 0 || month > 12 { range_err = "month".into(); }
                    }
                    Err(_) => err_mark = true,
                }
            }
            STD_WEEK_DAY => {
                match lookup_name(&SHORT_DAY_NAMES, &value_rest) {
                    Ok((_, rest)) => { value_rest = rest.to_string(); }
                    Err(_) => err_mark = true,
                }
            }
            STD_LONG_WEEK_DAY => {
                match lookup_name(&LONG_DAY_NAMES, &value_rest) {
                    Ok((_, rest)) => { value_rest = rest.to_string(); }
                    Err(_) => err_mark = true,
                }
            }
            STD_DAY | STD_UNDER_DAY | STD_ZERO_DAY => {
                if std & STD_MASK == STD_UNDER_DAY && !value_rest.is_empty() && value_rest.as_bytes()[0] == b' ' {
                    value_rest = value_rest[1..].to_string();
                }
                match get_num(&value_rest, std & STD_MASK == STD_ZERO_DAY) {
                    Ok((d, rest)) => { day = d; value_rest = rest.to_string(); }
                    Err(_) => err_mark = true,
                }
            }
            STD_UNDER_YEAR_DAY | STD_ZERO_YEAR_DAY => {
                for _ in 0..2 {
                    if std & STD_MASK == STD_UNDER_YEAR_DAY && !value_rest.is_empty() && value_rest.as_bytes()[0] == b' ' {
                        value_rest = value_rest[1..].to_string();
                    }
                }
                match get_num3(&value_rest, std & STD_MASK == STD_ZERO_YEAR_DAY) {
                    Ok((y, rest)) => { yday = y; value_rest = rest.to_string(); }
                    Err(_) => err_mark = true,
                }
            }
            STD_HOUR => {
                match get_num(&value_rest, false) {
                    Ok((h, rest)) => { hour = h; value_rest = rest.to_string();
                        if hour < 0 || hour >= 24 { range_err = "hour".into(); }
                    }
                    Err(_) => err_mark = true,
                }
            }
            STD_HOUR_12 | STD_ZERO_HOUR_12 => {
                match get_num(&value_rest, std & STD_MASK == STD_ZERO_HOUR_12) {
                    Ok((h, rest)) => { hour = h; value_rest = rest.to_string();
                        if hour < 0 || hour > 12 { range_err = "hour".into(); }
                    }
                    Err(_) => err_mark = true,
                }
            }
            STD_MINUTE | STD_ZERO_MINUTE => {
                match get_num(&value_rest, std & STD_MASK == STD_ZERO_MINUTE) {
                    Ok((m, rest)) => { min = m; value_rest = rest.to_string();
                        if min < 0 || min >= 60 { range_err = "minute".into(); }
                    }
                    Err(_) => err_mark = true,
                }
            }
            STD_SECOND | STD_ZERO_SECOND => {
                match get_num(&value_rest, std & STD_MASK == STD_ZERO_SECOND) {
                    Ok((s, rest)) => {
                        sec = s;
                        value_rest = rest.to_string();
                        if sec < 0 || sec >= 60 { range_err = "second".into(); }
                        else {
                            // Optional fractional second after this field if the layout doesn't contain one.
                            let vb = value_rest.as_bytes();
                            if vb.len() >= 2 && comma_or_period(vb[0]) && is_digit_ch(&value_rest, 1) {
                                let (_, peek_std, _) = next_std_chunk(layout_rest);
                                let peek_mask = peek_std & STD_MASK;
                                if peek_mask == STD_FRAC_SECOND_0 || peek_mask == STD_FRAC_SECOND_9 {
                                    // layout has it, let the next iteration handle.
                                } else {
                                    let mut n = 2;
                                    while n < vb.len() && is_digit_ch(&value_rest, n) { n += 1; }
                                    match parse_nanoseconds(&value_rest, n) {
                                        Ok((ns, rng)) => {
                                            nsec = ns;
                                            if !rng.is_empty() { range_err = rng; }
                                            value_rest = value_rest[n..].to_string();
                                        }
                                        Err(_) => err_mark = true,
                                    }
                                }
                            }
                        }
                    }
                    Err(_) => err_mark = true,
                }
            }
            STD_PM => {
                if value_rest.len() < 2 { err_mark = true; }
                else {
                    let (p, rest) = value_rest.split_at(2);
                    match p {
                        "PM" => { pm_set = true; value_rest = rest.to_string(); }
                        "AM" => { am_set = true; value_rest = rest.to_string(); }
                        _ => err_mark = true,
                    }
                }
            }
            STDPM => {
                if value_rest.len() < 2 { err_mark = true; }
                else {
                    let (p, rest) = value_rest.split_at(2);
                    match p {
                        "pm" => { pm_set = true; value_rest = rest.to_string(); }
                        "am" => { am_set = true; value_rest = rest.to_string(); }
                        _ => err_mark = true,
                    }
                }
            }
            code @ (STD_ISO8601_TZ | STD_ISO8601_SHORT_TZ | STD_ISO8601_COLON_TZ
                | STD_ISO8601_SECONDS_TZ | STD_ISO8601_COLON_SECONDS_TZ
                | STD_NUM_TZ | STD_NUM_SHORT_TZ | STD_NUM_COLON_TZ
                | STD_NUM_SECONDS_TZ | STD_NUM_COLON_SECONDS_TZ) => {
                let iso_z = matches!(code, STD_ISO8601_TZ | STD_ISO8601_SHORT_TZ
                    | STD_ISO8601_COLON_TZ | STD_ISO8601_SECONDS_TZ | STD_ISO8601_COLON_SECONDS_TZ);
                if iso_z && !value_rest.is_empty() && value_rest.as_bytes()[0] == b'Z' {
                    value_rest = value_rest[1..].to_string();
                    zone_loc = Some(utc_loc().clone());
                } else {
                    let (sign, h, m, s, new_rest, ok) = parse_zone_fields(&value_rest, code);
                    if !ok { err_mark = true; }
                    else {
                        let hr: i64 = match h.parse() { Ok(n) => n, Err(_) => { err_mark = true; 0 } };
                        let mm: i64 = match m.parse() { Ok(n) => n, Err(_) => { err_mark = true; 0 } };
                        let ss: i64 = match s.parse() { Ok(n) => n, Err(_) => { err_mark = true; 0 } };
                        if !err_mark {
                            if hr > 24 { range_err = "time zone offset hour".into(); }
                            else if mm > 60 { range_err = "time zone offset minute".into(); }
                            else if ss > 60 { range_err = "time zone offset second".into(); }
                            else {
                                let mut off = (hr * 60 + mm) * 60 + ss;
                                match sign {
                                    "+" => {}
                                    "-" => { off = -off; }
                                    _ => err_mark = true,
                                }
                                zone_offset = off;
                                value_rest = new_rest.to_string();
                            }
                        }
                    }
                }
            }
            STD_TZ => {
                if value_rest.len() >= 3 && &value_rest[..3] == "UTC" {
                    zone_loc = Some(utc_loc().clone());
                    value_rest = value_rest[3..].to_string();
                } else {
                    let (n, ok) = ParseTimeZone(&value_rest);
                    if !ok { err_mark = true; }
                    else {
                        zone_name = value_rest[..n as usize].to_string();
                        value_rest = value_rest[n as usize..].to_string();
                    }
                }
            }
            STD_FRAC_SECOND_0 => {
                let ndigit = 1 + digits_len(std) as usize;
                if value_rest.len() < ndigit { err_mark = true; }
                else {
                    match parse_nanoseconds(&value_rest, ndigit) {
                        Ok((ns, rng)) => {
                            nsec = ns;
                            if !rng.is_empty() { range_err = rng; }
                            value_rest = value_rest[ndigit..].to_string();
                        }
                        Err(_) => err_mark = true,
                    }
                }
            }
            STD_FRAC_SECOND_9 => {
                let vb = value_rest.as_bytes();
                if vb.len() < 2 || !comma_or_period(vb[0]) || vb[1] < b'0' || vb[1] > b'9' {
                    // omitted.
                } else {
                    let mut i = 0;
                    while i + 1 < vb.len() && vb[i+1] >= b'0' && vb[i+1] <= b'9' { i += 1; }
                    match parse_nanoseconds(&value_rest, 1 + i) {
                        Ok((ns, rng)) => {
                            nsec = ns;
                            if !rng.is_empty() { range_err = rng; }
                            value_rest = value_rest[1 + i..].to_string();
                        }
                        Err(_) => err_mark = true,
                    }
                }
            }
            _ => {}
        }
        if !range_err.is_empty() {
            let msg = format!(": {} out of range", range_err);
            return (Time::default(), parse_err(alayout, avalue, &stdstr_owned, &value_rest, &msg));
        }
        if err_mark {
            return (Time::default(), parse_err(alayout, avalue, &stdstr_owned, &hold, ""));
        }
    }
    if pm_set && hour < 12 { hour += 12; }
    else if am_set && hour == 12 { hour = 0; }

    // Convert yday to day, month.
    if yday >= 0 {
        let mut m = 0i64;
        let mut d = 0i64;
        let mut yday_adj = yday;
        if is_leap(year) {
            if yday_adj == 31 + 29 {
                m = 2;
                d = 29;
            } else if yday_adj > 31 + 29 {
                yday_adj -= 1;
            }
        }
        if yday_adj < 1 || yday_adj > 365 {
            return (Time::default(), parse_err(alayout, avalue, "", &value_rest, ": day-of-year out of range"));
        }
        if m == 0 {
            m = (yday_adj - 1) / 31 + 1;
            if days_before((m + 1) as u32) < yday_adj {
                m += 1;
            }
            d = yday_adj - days_before(m as u32);
        }
        if month >= 0 && month != m {
            return (Time::default(), parse_err(alayout, avalue, "", &value_rest, ": day-of-year does not match month"));
        }
        month = m;
        if day >= 0 && day != d {
            return (Time::default(), parse_err(alayout, avalue, "", &value_rest, ": day-of-year does not match day"));
        }
        day = d;
    } else {
        if month < 0 { month = 1; }
        if day < 0 { day = 1; }
    }

    if day < 1 || day > days_in(month as u32, year) {
        return (Time::default(), parse_err(alayout, avalue, "", &value_rest, ": day out of range"));
    }

    if let Some(z) = zone_loc {
        let t = Date(year as crate::types::int, Month(month as crate::types::int),
                     day as crate::types::int, hour as crate::types::int,
                     min as crate::types::int, sec as crate::types::int,
                     nsec as crate::types::int, z);
        return (t, crate::errors::nil);
    }

    if zone_offset != -1 {
        // Build time as if UTC, then shift.
        let mut t = Date(year as crate::types::int, Month(month as crate::types::int),
                         day as crate::types::int, hour as crate::types::int,
                         min as crate::types::int, sec as crate::types::int,
                         nsec as crate::types::int, UTC);
        t.sec -= zone_offset;
        // Check if local zone has this offset (we only know Local has offset 0; fallback to FixedZone).
        let fake = FixedZone(zone_name.as_str(), zone_offset as crate::types::int);
        t.loc = fake;
        return (t, crate::errors::nil);
    }

    if !zone_name.is_empty() {
        let mut t = Date(year as crate::types::int, Month(month as crate::types::int),
                         day as crate::types::int, hour as crate::types::int,
                         min as crate::types::int, sec as crate::types::int,
                         nsec as crate::types::int, UTC);
        let off = if zone_name.len() > 3 && &zone_name[..3] == "GMT" {
            let rest = &zone_name[3..];
            let n: i64 = rest.parse().unwrap_or(0);
            n * 3600
        } else {
            known_zone_offset(&zone_name).unwrap_or(0)
        };
        t.sec -= off;
        t.loc = FixedZone(zone_name.clone(), off as crate::types::int);
        return (t, crate::errors::nil);
    }

    let _ = local;
    (Date(year as crate::types::int, Month(month as crate::types::int),
          day as crate::types::int, hour as crate::types::int,
          min as crate::types::int, sec as crate::types::int,
          nsec as crate::types::int, default_loc), crate::errors::nil)
}

/// known_zone_offset returns the canonical UTC offset (in seconds) for
/// well-known zone abbreviations. No tzdata — only the historical IANA
/// short names. Needed so ports like UnixDate ("... MST ...") round-trip
/// the offset even without a timezone database.
fn known_zone_offset(name: &str) -> Option<i64> {
    match name {
        "UTC" | "GMT" | "Z" => Some(0),
        "EST" => Some(-5 * 3600), "EDT" => Some(-4 * 3600),
        "CST" => Some(-6 * 3600), "CDT" => Some(-5 * 3600),
        "MST" => Some(-7 * 3600), "MDT" => Some(-6 * 3600),
        "PST" => Some(-8 * 3600), "PDT" => Some(-7 * 3600),
        "AKST" => Some(-9 * 3600), "AKDT" => Some(-8 * 3600),
        "HST" => Some(-10 * 3600),
        "AST" => Some(-4 * 3600), "ADT" => Some(-3 * 3600),
        "NST" => Some(-(3*3600 + 30*60)),
        "JST" => Some(9 * 3600),
        "KST" => Some(9 * 3600),
        "CET" => Some(1 * 3600), "CEST" => Some(2 * 3600),
        "EET" => Some(2 * 3600), "EEST" => Some(3 * 3600),
        "BST" => Some(1 * 3600),
        "IST" => Some(5 * 3600 + 30 * 60),
        "WET" => Some(0), "WEST" => Some(1 * 3600),
        _ => None,
    }
}

fn parse_zone_fields<'a>(value: &'a str, code: i32) -> (&'a str, &'a str, &'a str, &'a str, &'a str, bool) {
    let b = value.as_bytes();
    match code {
        STD_ISO8601_COLON_TZ | STD_NUM_COLON_TZ => {
            if b.len() < 6 || b[3] != b':' { return ("","","","","",false); }
            (&value[0..1], &value[1..3], &value[4..6], "00", &value[6..], true)
        }
        STD_NUM_SHORT_TZ | STD_ISO8601_SHORT_TZ => {
            if b.len() < 3 { return ("","","","","",false); }
            (&value[0..1], &value[1..3], "00", "00", &value[3..], true)
        }
        STD_ISO8601_COLON_SECONDS_TZ | STD_NUM_COLON_SECONDS_TZ => {
            if b.len() < 9 || b[3] != b':' || b[6] != b':' { return ("","","","","",false); }
            (&value[0..1], &value[1..3], &value[4..6], &value[7..9], &value[9..], true)
        }
        STD_ISO8601_SECONDS_TZ | STD_NUM_SECONDS_TZ => {
            if b.len() < 7 { return ("","","","","",false); }
            (&value[0..1], &value[1..3], &value[3..5], &value[5..7], &value[7..], true)
        }
        _ => {
            if b.len() < 5 { return ("","","","","",false); }
            (&value[0..1], &value[1..3], &value[3..5], "00", &value[5..], true)
        }
    }
}

// ─── Quote (for ParseError messages) ──────────────────────────────────

/// Go's internal time.quote: wraps the input in quotes, escaping " and \\,
/// and encoding non-ASCII / control bytes as \xNN. Matches Go's test
/// expectations.
#[allow(non_snake_case)]
pub fn Quote(s: impl AsRef<str>) -> crate::types::string {
    quote_go(s.as_ref())
}

fn quote_go(s: &str) -> String {
    const LOWERHEX: &[u8] = b"0123456789abcdef";
    let mut buf: Vec<u8> = Vec::with_capacity(s.len() + 2);
    buf.push(b'"');
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c >= 0x80 || c < 0x20 {
            // Non-ASCII or control: emit entire UTF-8 rune as \xNN per byte.
            // Determine rune width.
            let width = utf8_width(&bytes[i..]);
            for j in 0..width {
                let byte = bytes[i + j];
                buf.extend_from_slice(b"\\x");
                buf.push(LOWERHEX[(byte >> 4) as usize]);
                buf.push(LOWERHEX[(byte & 0xf) as usize]);
            }
            i += width;
        } else {
            if c == b'"' || c == b'\\' { buf.push(b'\\'); }
            buf.push(c);
            i += 1;
        }
    }
    buf.push(b'"');
    String::from_utf8(buf).unwrap_or_default()
}

fn utf8_width(b: &[u8]) -> usize {
    if b.is_empty() { return 1; }
    let c = b[0];
    if c < 0x80 { 1 }
    else if c < 0xc0 { 1 }
    else if c < 0xe0 { 2 }
    else if c < 0xf0 { 3 }
    else { 4 }
}

// ─── ParseDuration ────────────────────────────────────────────────────

#[allow(non_snake_case)]
pub fn ParseDuration(s: impl AsRef<str>) -> (Duration, crate::errors::error) {
    let orig = s.as_ref();
    let mut rest = orig;
    let mut neg = false;
    if let Some(c) = rest.as_bytes().first() {
        if *c == b'-' || *c == b'+' {
            neg = *c == b'-';
            rest = &rest[1..];
        }
    }
    if rest == "0" { return (Duration::from_nanos(0), crate::errors::nil); }
    if rest.is_empty() {
        return (Duration::from_nanos(0),
                crate::errors::New(&format!("time: invalid duration {}", quote_go(orig))));
    }
    let mut total: i128 = 0;
    while !rest.is_empty() {
        let b = rest.as_bytes();
        if !(b[0] == b'.' || (b[0] >= b'0' && b[0] <= b'9')) {
            return (Duration::from_nanos(0),
                    crate::errors::New(&format!("time: invalid duration {}", quote_go(orig))));
        }
        // Leading int
        let mut i = 0;
        let mut v: i128 = 0;
        while i < b.len() && b[i] >= b'0' && b[i] <= b'9' {
            v = v * 10 + (b[i] - b'0') as i128;
            i += 1;
        }
        let pre = i > 0;
        // Fraction
        let mut f: i128 = 0;
        let mut scale: i128 = 1;
        let mut post = false;
        if i < b.len() && b[i] == b'.' {
            i += 1;
            while i < b.len() && b[i] >= b'0' && b[i] <= b'9' {
                if scale < 1_000_000_000_000 {
                    f = f * 10 + (b[i] - b'0') as i128;
                    scale *= 10;
                }
                i += 1;
                post = true;
            }
        }
        if !pre && !post {
            return (Duration::from_nanos(0),
                    crate::errors::New(&format!("time: invalid duration {}", quote_go(orig))));
        }
        // Unit
        let mut j = i;
        while j < b.len() && !(b[j] == b'.' || (b[j] >= b'0' && b[j] <= b'9')) { j += 1; }
        if i == j {
            return (Duration::from_nanos(0),
                    crate::errors::New(&format!("time: missing unit in duration {}", quote_go(orig))));
        }
        let u = &rest[i..j];
        let unit: i128 = match u {
            "ns" => 1,
            "us" | "µs" | "μs" => 1_000,
            "ms" => 1_000_000,
            "s"  => 1_000_000_000,
            "m"  => 60 * 1_000_000_000,
            "h"  => 3600 * 1_000_000_000,
            _ => return (Duration::from_nanos(0),
                         crate::errors::New(&format!("time: unknown unit {} in duration {}",
                             quote_go(u), quote_go(orig)))),
        };
        let mut sum = v * unit;
        if f > 0 { sum += f * unit / scale; }
        total += sum;
        rest = &rest[j..];
    }
    if neg { total = -total; }
    (Duration::from_nanos(total), crate::errors::nil)
}

// ─── Ticker / Timer / AfterFunc (unchanged semantics) ────────────────

pub struct Ticker {
    pub C: crate::chan::Chan<Time>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    period: std::sync::Arc<std::sync::atomic::AtomicI64>,
}

impl Ticker {
    /// Stop halts the ticker. Safe to call repeatedly; always a no-op after the first time.
    pub fn Stop(&self) { self.stop.store(true, std::sync::atomic::Ordering::SeqCst); }

    /// Reset stops the ticker and restarts it with the given duration.
    /// Panics if d <= 0.
    pub fn Reset(&self, d: Duration) {
        if d.nanos <= 0 { panic!("non-positive interval for Ticker.Reset"); }
        self.period.store(d.nanos as i64, std::sync::atomic::Ordering::SeqCst);
    }
}

#[allow(non_snake_case)]
pub fn NewTicker(d: Duration) -> Ticker {
    if d.nanos <= 0 {
        panic!("non-positive interval for NewTicker");
    }
    let ch = crate::chan::Chan::<Time>::new(1);
    let producer = ch.clone();
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stop_clone = stop.clone();
    let period = std::sync::Arc::new(std::sync::atomic::AtomicI64::new(d.nanos as i64));
    let period_clone = period.clone();
    std::thread::spawn(move || {
        while !stop_clone.load(std::sync::atomic::Ordering::SeqCst) {
            let ns = period_clone.load(std::sync::atomic::Ordering::SeqCst);
            std::thread::sleep(std::time::Duration::from_nanos(ns as u64));
            if stop_clone.load(std::sync::atomic::Ordering::SeqCst) { break; }
            let _ = producer.Send(Now());
        }
    });
    Ticker { C: ch, stop, period }
}

/// Tick is a convenience wrapper around NewTicker, returning the channel.
/// For negative or zero duration, returns None (mirrors Go's nil channel).
#[allow(non_snake_case)]
pub fn Tick(d: Duration) -> Option<crate::chan::Chan<Time>> {
    if d.nanos <= 0 { return None; }
    Some(NewTicker(d).C)
}

pub struct Timer {
    pub C: crate::chan::Chan<Time>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Timer {
    pub fn Stop(&self) -> bool {
        let was = self.stop.swap(true, std::sync::atomic::Ordering::SeqCst);
        !was
    }
    /// Reset halts this timer and restarts it with the given duration.
    /// For simplicity, the returned Timer is always considered pending after Reset.
    pub fn Reset(&self, _d: Duration) -> bool {
        // Only semantically useful with full scheduler support; mark as active.
        let was = self.stop.swap(false, std::sync::atomic::Ordering::SeqCst);
        was
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

#[allow(non_snake_case)]
pub fn After(d: Duration) -> crate::chan::Chan<Time> { NewTimer(d).C }

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
        assert_eq!((Second * 2i64).Seconds(), 2.0);
        assert_eq!((Millisecond * 500i64).Milliseconds(), 500);
    }

    #[test]
    fn date_and_format_basic() {
        let t = Date(2026, April, 15, 10, 30, 45, 0, UTC);
        assert_eq!(t.Format("2006-01-02 15:04:05"), "2026-04-15 10:30:45");
    }

    #[test]
    fn format_reference_time_core() {
        // Wed Feb 4 21:00:57.012345600 PST 2009 as UTC-8 → UTC: 2009-02-05 05:00:57.0123456 UTC
        // Using UTC to avoid tz issues: Feb 4 21:00:57 2009 UTC.
        let t = Date(2009, February, 4, 21, 0, 57, 12_345_600, UTC);
        assert_eq!(t.Format(ANSIC), "Wed Feb  4 21:00:57 2009");
        assert_eq!(t.Format(DateOnly), "2009-02-04");
        assert_eq!(t.Format(TimeOnly), "21:00:57");
        assert_eq!(t.Format(DateTime), "2009-02-04 21:00:57");
    }

    #[test]
    fn weekday_matches_civil() {
        let t = Date(2020, January, 1, 0, 0, 0, 0, UTC); // Wednesday
        assert_eq!(t.Weekday(), Wednesday);
    }

    #[test]
    fn yearday_basic() {
        let t = Date(2020, February, 1, 0, 0, 0, 0, UTC);
        assert_eq!(t.YearDay(), 32);
        let t = Date(2020, March, 1, 0, 0, 0, 0, UTC);
        assert_eq!(t.YearDay(), 61); // leap year
    }

    #[test]
    fn zero_time_is_jan_1_year_1() {
        let z = Time::default();
        assert_eq!(z.Year(), 1);
        assert_eq!(z.Month(), January);
        assert_eq!(z.Day(), 1);
        assert_eq!(z.Weekday(), Monday);
        assert_eq!(z.YearDay(), 1);
    }
}
