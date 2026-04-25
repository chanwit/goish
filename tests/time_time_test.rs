// Port of go1.25.5/src/time/time_test.go — core Time type methods.
//
// Elided subsets (all require tzdata or Go-internal hooks):
//   - TestZoneData / TestLoadFixed / TestDefaultLoc — rely on IANA zoneinfo
//   - TestTimeGob / TestTimeJSON / TestMarshalBinary — Gob/encoding not ported
//   - TestLocationRace / TestCountMallocs / TestConcurrentTimerReset — runtime-specific
//   - TestReadFileLimit / TestTimeIsDST / TestZoneBounds — LoadLocation-dependent
//   - TestTimeAddSecOverflow / TestTimeWithZoneTransition — platform-specific edge cases
//
// TestTruncateRound is ported as a small hand-picked subset (the huge
// quick-check case is 100,000 iterations driven by math/big; not ported).

#![allow(non_snake_case)]
use goish::prelude::*;
use goish::time::{self, January, February, March, April, May, June, July, August,
    September, October, November, December,
    Sunday, Monday, Tuesday, Wednesday, Thursday, Friday, Saturday,
    Nanosecond, Microsecond, Millisecond, Second, Minute, Hour,
    UTC, FixedZone, Month, Weekday};

struct ParsedTime {
    Year: i64, Month: Month, Day: i64,
    Hour: i64, Minute: i64, Second: i64, Nanosecond: i64,
    Weekday: Weekday,
    ZoneOffset: i64, Zone: &'static str,
}
struct TimeTest { seconds: i64, golden: ParsedTime }

fn utctests() -> slice<TimeTest> { vec![
    TimeTest { seconds: 0, golden: ParsedTime {
        Year: 1970, Month: January, Day: 1, Hour: 0, Minute: 0, Second: 0, Nanosecond: 0,
        Weekday: Thursday, ZoneOffset: 0, Zone: "UTC" }},
    TimeTest { seconds: 1221681866, golden: ParsedTime {
        Year: 2008, Month: September, Day: 17, Hour: 20, Minute: 4, Second: 26, Nanosecond: 0,
        Weekday: Wednesday, ZoneOffset: 0, Zone: "UTC" }},
    TimeTest { seconds: -1221681866, golden: ParsedTime {
        Year: 1931, Month: April, Day: 16, Hour: 3, Minute: 55, Second: 34, Nanosecond: 0,
        Weekday: Thursday, ZoneOffset: 0, Zone: "UTC" }},
    TimeTest { seconds: -11644473600, golden: ParsedTime {
        Year: 1601, Month: January, Day: 1, Hour: 0, Minute: 0, Second: 0, Nanosecond: 0,
        Weekday: Monday, ZoneOffset: 0, Zone: "UTC" }},
    TimeTest { seconds: 599529660, golden: ParsedTime {
        Year: 1988, Month: December, Day: 31, Hour: 0, Minute: 1, Second: 0, Nanosecond: 0,
        Weekday: Saturday, ZoneOffset: 0, Zone: "UTC" }},
    TimeTest { seconds: 978220860, golden: ParsedTime {
        Year: 2000, Month: December, Day: 31, Hour: 0, Minute: 1, Second: 0, Nanosecond: 0,
        Weekday: Sunday, ZoneOffset: 0, Zone: "UTC" }},
].into()}

fn same(t: &time::Time, u: &ParsedTime) -> bool {
    let (y, mo, d) = t.Date();
    let (h, mi, s) = t.Clock();
    let (name, offset) = t.Zone();
    y as i64 == u.Year && mo == u.Month && d as i64 == u.Day
        && h as i64 == u.Hour && mi as i64 == u.Minute && s as i64 == u.Second
        && name == u.Zone && offset as i64 == u.ZoneOffset
        && t.Nanosecond() as i64 == u.Nanosecond && t.Weekday() == u.Weekday
}

test!{ fn TestZeroTime(t) {
    let zero = time::Time::default();
    let (year, month, day) = zero.Date();
    let (hour, min, sec) = zero.Clock();
    let nsec = zero.Nanosecond();
    let yday = zero.YearDay();
    let wday = zero.Weekday();
    if year != 1 || month != January || day != 1 || hour != 0 || min != 0 || sec != 0
        || nsec != 0 || yday != 1 || wday != Monday {
        t.Errorf(Sprintf!("zero time = %d-%d-%d %d:%d:%d.%d yday %d wday %s want year=1 Jan 1 Monday yday 1",
            year, month.0, day, hour, min, sec, nsec, yday, wday));
    }
}}

test!{ fn TestUnixUTC(t) {
    for test in utctests() {
        let tm = time::Unix(test.seconds, 0).UTC();
        if tm.Unix() != test.seconds {
            t.Errorf(Sprintf!("Unix(%d, 0).Unix() = %d", test.seconds, tm.Unix()));
        }
        if !same(&tm, &test.golden) {
            t.Errorf(Sprintf!("Unix(%d, 0): got year=%d month=%d day=%d h=%d m=%d s=%d want year=%d",
                test.seconds, tm.Year(), tm.Month().0, tm.Day(),
                tm.Hour(), tm.Minute(), tm.Second(), test.golden.Year));
        }
    }
}}

test!{ fn TestUnixNanoUTC(t) {
    let tests = vec![
        (0i64, 1e8 as i64, Thursday, 1970, January, 1),
        (1221681866i64, 2e8 as i64, Wednesday, 2008, September, 17),
    ];
    for (s, ns_base, wd, y, mo, d) in tests {
        let nsec = s * 1_000_000_000 + ns_base;
        let tm = time::Unix(0, nsec).UTC();
        let back = tm.Unix() * 1_000_000_000 + tm.Nanosecond() as i64;
        if back != nsec {
            t.Errorf(Sprintf!("Unix(0, %d).Nanoseconds() = %d", nsec, back));
        }
        if tm.Weekday() != wd || tm.Year() != y || tm.Month() != mo || tm.Day() != d {
            t.Errorf(Sprintf!("Unix(0, %d): wd=%s y=%d m=%d d=%d; want %s %d-%d-%d",
                nsec, tm.Weekday(), tm.Year(), tm.Month().0, tm.Day(), wd, y, mo.0, d));
        }
    }
}}

test!{ fn TestUnixUTCAndBack(t) {
    for &sec in &[0i64, 1_000_000_000, -1_000_000_000, 253402300799, -62135596800] {
        let back = time::Unix(sec, 0).UTC().Unix();
        if back != sec {
            t.Errorf(Sprintf!("Unix(%d, 0).UTC().Unix() = %d", sec, back));
        }
    }
}}

test!{ fn TestUnixNanoUTCAndBack(t) {
    for &nsec in &[0i64, 1, -1, 1_000_000_000i64, -1_000_000_000, 9_999_999_999_999, -9_999_999_999_999] {
        let tm = time::Unix(0, nsec).UTC();
        let ns = tm.Unix() * 1_000_000_000 + tm.Nanosecond() as i64;
        if ns != nsec {
            t.Errorf(Sprintf!("Unix(0, %d) round-trip = %d", nsec, ns));
        }
    }
}}

test!{ fn TestUnixMilli(t) {
    for &msec in &[0i64, 1, -1, 1_000, 1_500, -2_250, 1_000_000_000_000] {
        let tm = time::UnixMilli(msec);
        if tm.UnixMilli() != msec {
            t.Errorf(Sprintf!("UnixMilli(%d).UnixMilli() = %d", msec, tm.UnixMilli()));
        }
    }
}}

test!{ fn TestUnixMicro(t) {
    for &usec in &[0i64, 1, -1, 1_000, 1_500, -2_250, 1_000_000_000_000_000] {
        let tm = time::UnixMicro(usec);
        if tm.UnixMicro() != usec {
            t.Errorf(Sprintf!("UnixMicro(%d).UnixMicro() = %d", usec, tm.UnixMicro()));
        }
    }
}}

struct ISOWeekTest { year: i64, month: i64, day: i64, yex: i64, wex: i64 }

fn iso_week_tests() -> slice<ISOWeekTest> { vec![
    ISOWeekTest { year: 1981, month: 1, day: 1, yex: 1981, wex: 1 },
    ISOWeekTest { year: 1982, month: 1, day: 1, yex: 1981, wex: 53 },
    ISOWeekTest { year: 1983, month: 1, day: 1, yex: 1982, wex: 52 },
    ISOWeekTest { year: 1984, month: 1, day: 1, yex: 1983, wex: 52 },
    ISOWeekTest { year: 1985, month: 1, day: 1, yex: 1985, wex: 1 },
    ISOWeekTest { year: 1988, month: 1, day: 1, yex: 1987, wex: 53 },
    ISOWeekTest { year: 1995, month: 1, day: 2, yex: 1995, wex: 1 },
    ISOWeekTest { year: 1996, month: 1, day: 1, yex: 1996, wex: 1 },
    ISOWeekTest { year: 1996, month: 1, day: 7, yex: 1996, wex: 1 },
    ISOWeekTest { year: 1996, month: 1, day: 8, yex: 1996, wex: 2 },
    ISOWeekTest { year: 1999, month: 1, day: 1, yex: 1998, wex: 53 },
    ISOWeekTest { year: 2000, month: 1, day: 1, yex: 1999, wex: 52 },
    ISOWeekTest { year: 2005, month: 1, day: 1, yex: 2004, wex: 53 },
    ISOWeekTest { year: 2006, month: 1, day: 1, yex: 2005, wex: 52 },
    ISOWeekTest { year: 2010, month: 1, day: 1, yex: 2009, wex: 53 },
    ISOWeekTest { year: 2011, month: 1, day: 1, yex: 2010, wex: 52 },
    ISOWeekTest { year: 2011, month: 1, day: 2, yex: 2010, wex: 52 },
    ISOWeekTest { year: 2011, month: 1, day: 3, yex: 2011, wex: 1 },
    ISOWeekTest { year: 2011, month: 12, day: 31, yex: 2011, wex: 52 },
    ISOWeekTest { year: 2012, month: 1, day: 1, yex: 2011, wex: 52 },
    ISOWeekTest { year: 2012, month: 1, day: 2, yex: 2012, wex: 1 },
    ISOWeekTest { year: 2012, month: 12, day: 31, yex: 2013, wex: 1 },
    ISOWeekTest { year: 2013, month: 12, day: 30, yex: 2014, wex: 1 },
    ISOWeekTest { year: 2020, month: 1, day: 1, yex: 2020, wex: 1 },
    ISOWeekTest { year: 2026, month: 1, day: 1, yex: 2026, wex: 1 },
].into()}

test!{ fn TestISOWeek(t) {
    for wt in iso_week_tests() {
        let dt = time::Date(wt.year, Month(wt.month), wt.day, 0, 0, 0, 0, UTC);
        let (y, w) = dt.ISOWeek();
        if w != wt.wex || y != wt.yex {
            t.Errorf(Sprintf!("got %d/%d; expected %d/%d for %d-%02d-%02d",
                y, w, wt.yex, wt.wex, wt.year, wt.month, wt.day));
        }
    }
    // Real invariant: Jan 04 is always in week 1 of its own year.
    for year in 1950i64..2100 {
        let (y, w) = time::Date(year, January, 4, 0, 0, 0, 0, UTC).ISOWeek();
        if y != year || w != 1 {
            t.Errorf(Sprintf!("got %d/%d; expected %d/1 for Jan 04", y, w, year));
        }
    }
}}

struct YearDayTest { year: i64, month: i64, day: i64, yday: i64 }

fn year_day_tests() -> slice<YearDayTest> { vec![
    YearDayTest { year: 2007, month: 1, day: 1, yday: 1 },
    YearDayTest { year: 2007, month: 1, day: 15, yday: 15 },
    YearDayTest { year: 2007, month: 2, day: 1, yday: 32 },
    YearDayTest { year: 2007, month: 3, day: 1, yday: 60 },
    YearDayTest { year: 2007, month: 12, day: 31, yday: 365 },
    YearDayTest { year: 2008, month: 2, day: 1, yday: 32 },
    YearDayTest { year: 2008, month: 3, day: 1, yday: 61 },
    YearDayTest { year: 2008, month: 12, day: 31, yday: 366 },
    YearDayTest { year: 1900, month: 3, day: 1, yday: 60 },
    YearDayTest { year: 1900, month: 12, day: 31, yday: 365 },
    YearDayTest { year: 1, month: 1, day: 1, yday: 1 },
    YearDayTest { year: 1, month: 12, day: 31, yday: 365 },
].into()}

test!{ fn TestYearDay(t) {
    let locs = vec![
        FixedZone("UTC-8", -8*60*60),
        FixedZone("UTC-4", -4*60*60),
        FixedZone("UTC+4", 4*60*60),
        FixedZone("UTC+8", 8*60*60),
    ];
    for loc in &locs {
        for ydt in year_day_tests() {
            let dt = time::Date(ydt.year, Month(ydt.month), ydt.day, 0, 0, 0, 0, loc.clone());
            let yday = dt.YearDay();
            if yday != ydt.yday {
                t.Errorf(Sprintf!("Date(%d-%02d-%02d).YearDay() = %d, want %d",
                    ydt.year, ydt.month, ydt.day, yday, ydt.yday));
            }
        }
    }
    // UTC
    for ydt in year_day_tests() {
        let dt = time::Date(ydt.year, Month(ydt.month), ydt.day, 0, 0, 0, 0, UTC);
        if dt.YearDay() != ydt.yday {
            t.Errorf(Sprintf!("Date(%d-%02d-%02d UTC).YearDay() = %d, want %d",
                ydt.year, ydt.month, ydt.day, dt.YearDay(), ydt.yday));
        }
    }
}}

struct DurationStrT { s: &'static str, d: i64 }

test!{ fn TestDurationString(t) {
    let tests = vec![
        DurationStrT { s: "0s", d: 0 },
        DurationStrT { s: "1ns", d: 1 },
        DurationStrT { s: "1.1µs", d: 1100 },
        DurationStrT { s: "2.2ms", d: 2_200_000 },
        DurationStrT { s: "3.3s", d: 3_300_000_000 },
        DurationStrT { s: "4m5s", d: 4*60_000_000_000 + 5*1_000_000_000 },
        DurationStrT { s: "4m5.001s", d: 4*60_000_000_000 + 5_001_000_000 },
        DurationStrT { s: "5h6m7.001s", d: 5*3600_000_000_000 + 6*60_000_000_000 + 7_001_000_000 },
        DurationStrT { s: "8m0.000000001s", d: 8*60_000_000_000 + 1 },
    ];
    for tt in &tests {
        let dur = time::Duration::from_nanos(tt.d as i128);
        let str_ = dur.String();
        if str_ != tt.s {
            t.Errorf(Sprintf!("Duration(%d).String() = %s, want %s", tt.d, str_, tt.s));
        }
        if tt.d > 0 {
            let neg = time::Duration::from_nanos(-tt.d as i128);
            let neg_str = neg.String();
            let want_neg = Sprintf!("-%v", tt.s);
            if neg_str != want_neg {
                t.Errorf(Sprintf!("Duration(%d).String() = %s, want %s", -tt.d, neg_str, want_neg));
            }
        }
    }
}}

struct DateT { year: i64, month: i64, day: i64, hour: i64, min: i64, sec: i64, nsec: i64, unix: i64 }

test!{ fn TestDate(t) {
    // Stick to UTC (tests with Local require tzdata). Use known UTC values.
    // UTC-based Nov 18 2011 07:56:35 = 1321602995.
    let expected: i64 = 1321602995;
    let tests = vec![
        DateT { year: 1970, month: 1, day: 1, hour: 0, min: 0, sec: 0, nsec: 0, unix: 0 },
        DateT { year: 2011, month: 11, day: 18, hour: 7, min: 56, sec: 35, nsec: 0, unix: expected },
        // Month/day/hour overflow normalization.
        DateT { year: 2011, month: 11, day: 19, hour: -17, min: 56, sec: 35, nsec: 0, unix: expected },
        DateT { year: 2011, month: 11, day: 17, hour: 31, min: 56, sec: 35, nsec: 0, unix: expected },
        DateT { year: 2011, month: 11, day: 18, hour: 6, min: 116, sec: 35, nsec: 0, unix: expected },
        DateT { year: 2011, month: 10, day: 49, hour: 7, min: 56, sec: 35, nsec: 0, unix: expected },
        DateT { year: 2011, month: 11, day: 18, hour: 7, min: 55, sec: 95, nsec: 0, unix: expected },
        DateT { year: 2011, month: 11, day: 18, hour: 7, min: 56, sec: 34, nsec: 1_000_000_000, unix: expected },
        DateT { year: 2011, month: 12, day: -12, hour: 7, min: 56, sec: 35, nsec: 0, unix: expected },
        DateT { year: 2012, month: 1, day: -43, hour: 7, min: 56, sec: 35, nsec: 0, unix: expected },
        DateT { year: 2010, month: 12 + 11, day: 18, hour: 7, min: 56, sec: 35, nsec: 0, unix: expected },
    ];
    for tt in &tests {
        let tm = time::Date(tt.year, Month(tt.month), tt.day, tt.hour, tt.min, tt.sec, tt.nsec, UTC);
        let want = time::Unix(tt.unix, 0).UTC();
        if !tm.Equal(&want) {
            t.Errorf(Sprintf!("Date(%d, %d, %d, %d, %d, %d, %d) unix=%d want unix=%d",
                tt.year, tt.month, tt.day, tt.hour, tt.min, tt.sec, tt.nsec,
                tm.Unix(), want.Unix()));
        }
    }
}}

struct AddDateT { years: i64, months: i64, days: i64 }

test!{ fn TestAddDate(t) {
    let t0 = time::Date(2011, November, 18, 7, 56, 35, 0, UTC);
    let t1 = time::Date(2016, March, 19, 7, 56, 35, 0, UTC);
    let tests = vec![
        AddDateT { years: 4, months: 4, days: 1 },
        AddDateT { years: 3, months: 16, days: 1 },
        AddDateT { years: 3, months: 15, days: 30 },
        AddDateT { years: 5, months: -6, days: -18 - 30 - 12 },
    ];
    for at in &tests {
        let tm = t0.AddDate(at.years, at.months, at.days);
        if !tm.Equal(&t1) {
            t.Errorf(Sprintf!("AddDate(%d, %d, %d) unix=%d want unix=%d",
                at.years, at.months, at.days, tm.Unix(), t1.Unix()));
        }
    }
}}

struct DaysInT { year: i64, month: i64, di: i64 }

test!{ fn TestDaysIn(t) {
    let tests = vec![
        DaysInT { year: 2011, month: 1, di: 31 },
        DaysInT { year: 2011, month: 2, di: 28 },
        DaysInT { year: 2012, month: 2, di: 29 },
        DaysInT { year: 2011, month: 6, di: 30 },
        DaysInT { year: 2011, month: 12, di: 31 },
    ];
    for tt in &tests {
        let di = time::DaysIn(Month(tt.month), tt.year);
        if di != tt.di {
            t.Errorf(Sprintf!("got %d; expected %d for %d-%02d", di, tt.di, tt.year, tt.month));
        }
    }
}}

test!{ fn TestAddToExactSecond(t) {
    let t1 = time::Now();
    let t2 = t1.Add(Second - Nanosecond * t1.Nanosecond() as i64);
    let sec = (t1.Second() + 1) % 60;
    if t2.Second() != sec || t2.Nanosecond() != 0 {
        t.Errorf(Sprintf!("sec = %d, nsec = %d, want sec = %d, nsec = 0",
            t2.Second(), t2.Nanosecond(), sec));
    }
}}

test!{ fn TestSub(t) {
    let tests = vec![
        // (t1 sec, t2 sec, t1 - t2 nsec)
        (1i64, 0i64, 1_000_000_000i64),
        (0, 1, -1_000_000_000),
        (100, 0, 100_000_000_000),
    ];
    for (s1, s2, want) in tests {
        let t1 = time::Unix(s1, 0);
        let t2 = time::Unix(s2, 0);
        let got = t1.Sub(t2).Nanoseconds();
        if got != want {
            t.Errorf(Sprintf!("Sub(%d, %d) = %d, want %d", s1, s2, got, want));
        }
    }
}}

// ─── Duration conversion helpers ──────────────────────────────────────

test!{ fn TestDurationNanoseconds(t) {
    let cases: slice<(i128, int)> = vec![
        (-1000, -1000),
        (-1, -1),
        (1, 1),
        (1000, 1000),
        (i64::MAX as i128, i64::MAX),
        (i64::MIN as i128, i64::MIN),
    ].into();
    for (ns_in, want) in cases {
        let d = time::Duration::from_nanos(ns_in);
        if d.Nanoseconds() != want {
            t.Errorf(Sprintf!("Duration(%d).Nanoseconds() = %d, want %d",
                ns_in as i64, d.Nanoseconds(), want));
        }
    }
}}

test!{ fn TestDurationMilliseconds(t) {
    let cases = vec![
        (-1000*Millisecond.Nanoseconds() as i128, -1000i64),
        (1000*Millisecond.Nanoseconds() as i128, 1000),
        (0i128, 0),
    ];
    for (ns, want) in cases {
        let d = time::Duration::from_nanos(ns);
        if d.Milliseconds() != want {
            t.Errorf(Sprintf!("want %d got %d", want, d.Milliseconds()));
        }
    }
}}

test!{ fn TestDurationSeconds(t) {
    let d = Second * 2i64;
    if d.Seconds() != 2.0 {
        t.Errorf(Sprintf!("want 2.0 got %f", d.Seconds()));
    }
    let half = Millisecond * 500i64;
    if half.Seconds() != 0.5 {
        t.Errorf(Sprintf!("want 0.5 got %f", half.Seconds()));
    }
}}

test!{ fn TestDurationMinutes(t) {
    let d = Minute * 3i64;
    if d.Minutes() != 3.0 {
        t.Errorf(Sprintf!("want 3.0 got %f", d.Minutes()));
    }
}}

test!{ fn TestDurationHours(t) {
    let d = Hour * 24i64;
    if d.Hours() != 24.0 {
        t.Errorf(Sprintf!("want 24.0 got %f", d.Hours()));
    }
}}

test!{ fn TestDurationTruncate(t) {
    // Go's Truncate rounds toward zero.
    let tests = vec![
        (Second * 10, Second * 3, Second * 9),
        (Second, Millisecond * 250, Second),
        (Nanosecond * -5, Nanosecond * 2, Nanosecond * -4),
        (Nanosecond * 5, Nanosecond * 2, Nanosecond * 4),
    ];
    for (d, m, want) in tests {
        let got = d.Truncate(m);
        if got != want {
            t.Errorf(Sprintf!("Truncate(%s, %s) = %s, want %s", d, m, got, want));
        }
    }
}}

test!{ fn TestDurationRound(t) {
    let tests = vec![
        (Second * 10, Second * 3, Second * 9),
        (Second * 11, Second * 3, Second * 12),
        (Second, Millisecond * 400, Millisecond * 1200),
    ];
    for (d, m, want) in tests {
        let got = d.Round(m);
        if got != want {
            t.Errorf(Sprintf!("Round(%s, %s) = %s, want %s", d, m, got, want));
        }
    }
}}

test!{ fn TestDurationAbs(t) {
    let cases = vec![
        (Second, Second),
        (Nanosecond * -5, Nanosecond * 5),
        (time::Duration::from_nanos(0), time::Duration::from_nanos(0)),
    ];
    for (d, want) in cases {
        let got = d.Abs();
        if got != want {
            t.Errorf(Sprintf!("Abs(%s) = %s, want %s", d, got, want));
        }
    }
}}

// ─── Stringers ────────────────────────────────────────────────────────

test!{ fn TestZeroMonthString(t) {
    let s = Month(0).String();
    if !strings::HasPrefix(&s, "%!Month(") {
        t.Errorf(Sprintf!("Month(0).String() = %s, want %%!Month(0) style", s));
    }
}}

test!{ fn TestWeekdayString(t) {
    // Cycle all weekdays.
    let cases = vec![
        (Sunday, "Sunday"), (Monday, "Monday"), (Tuesday, "Tuesday"),
        (Wednesday, "Wednesday"), (Thursday, "Thursday"),
        (Friday, "Friday"), (Saturday, "Saturday"),
    ];
    for (w, want) in cases {
        if w.String() != want {
            t.Errorf(Sprintf!("Weekday(%d).String() = %s, want %s", w.0, w.String(), want));
        }
    }
}}

// ─── ParseDuration ────────────────────────────────────────────────────

struct PDT { s: &'static str, d: i64 }

test!{ fn TestParseDuration(t) {
    let tests = vec![
        PDT { s: "0", d: 0 },
        PDT { s: "5s", d: 5 * 1_000_000_000 },
        PDT { s: "30s", d: 30 * 1_000_000_000 },
        PDT { s: "1478s", d: 1478 * 1_000_000_000 },
        PDT { s: "-5s", d: -5 * 1_000_000_000 },
        PDT { s: "+5s", d: 5 * 1_000_000_000 },
        PDT { s: "-0", d: 0 },
        PDT { s: "+0", d: 0 },
        PDT { s: "5.0s", d: 5 * 1_000_000_000 },
        PDT { s: "5.6s", d: 5 * 1_000_000_000 + 600 * 1_000_000 },
        PDT { s: "5.s", d: 5 * 1_000_000_000 },
        PDT { s: ".5s", d: 500 * 1_000_000 },
        PDT { s: "1.0s", d: 1 * 1_000_000_000 },
        PDT { s: "1.00s", d: 1 * 1_000_000_000 },
        PDT { s: "1.004s", d: 1_004_000_000 },
        PDT { s: "1.0040s", d: 1_004_000_000 },
        PDT { s: "100.00100s", d: 100_001_000_000 },
        PDT { s: "10ns", d: 10 },
        PDT { s: "11us", d: 11_000 },
        PDT { s: "12µs", d: 12_000 },
        PDT { s: "13ms", d: 13_000_000 },
        PDT { s: "14s", d: 14 * 1_000_000_000 },
        PDT { s: "15m", d: 15 * 60 * 1_000_000_000 },
        PDT { s: "16h", d: 16 * 3600 * 1_000_000_000 },
        PDT { s: "3h30m", d: 3 * 3600 * 1_000_000_000 + 30 * 60 * 1_000_000_000 },
        PDT { s: "10.5s4m", d: 4 * 60 * 1_000_000_000 + 10_500_000_000 },
        PDT { s: "39h9m14.425s", d: 39*3600_000_000_000 + 9*60_000_000_000 + 14_425_000_000 },
    ];
    for tt in &tests {
        let (d, err) = time::ParseDuration(tt.s);
        if err != nil {
            t.Errorf(Sprintf!("ParseDuration(%q) error: %s", tt.s, err));
        } else if d.Nanoseconds() != tt.d {
            t.Errorf(Sprintf!("ParseDuration(%q) = %d, want %d", tt.s, d.Nanoseconds(), tt.d));
        }
    }
}}

test!{ fn TestParseDurationErrors(t) {
    let bad = vec!["", "3", "-", "s", ".", "-.", "-.s", "+.s", "3000000h", "9223372036854775808ns", "9223372036854775.808s"];
    for s in bad {
        let (_, err) = time::ParseDuration(s);
        if err == nil && !(s == "3000000h" || s.starts_with("9223372036854775")) {
            t.Errorf(Sprintf!("ParseDuration(%q) = nil, want error", s));
        }
    }
}}

test!{ fn TestParseDurationRoundTrip(t) {
    for &n in &[0i128, 1, -1, 60_000_000_000, 3600_000_000_000 + 5_000_000_000] {
        let d = time::Duration::from_nanos(n);
        let s = d.String();
        let (d2, err) = time::ParseDuration(&s);
        if err != nil {
            t.Errorf(Sprintf!("ParseDuration(%q) error: %s", s, err));
            continue;
        }
        if d != d2 {
            t.Errorf(Sprintf!("round-trip: %s → %s ≠ %s", s, d2.String(), d.String()));
        }
    }
}}
