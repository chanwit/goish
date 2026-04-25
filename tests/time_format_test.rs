// Port of go1.25.5/src/time/format_test.go — the Format/Parse round-trip
// table built on Go's reference time "Mon Jan 2 15:04:05 MST 2006".
//
// What's elided: anything requiring LoadLocation (IANA tzdata) — namely
// TestParseInLocation, TestLoadLocationZipFile, TestZoneData. The PST/PDT
// cases in the format table are recomputed against UTC with the right
// offsets so the semantic round-trip is still verified.
//
// Fuzz tests (FuzzFormatRFC3339, FuzzParseRFC3339) are elided; go's quick
// framework isn't ported. The seed cases from those fuzz tests are covered
// by the explicit ParseFractionalSeconds tests below.

#![allow(non_snake_case)]
use goish::prelude::*;
use goish::time::{self, ANSIC, UnixDate, RubyDate, RFC822, RFC850, RFC1123, RFC1123Z,
    RFC3339, RFC3339Nano, Kitchen, Stamp, StampMilli, StampMicro, StampNano,
    DateTime, DateOnly, TimeOnly, February, January, Wednesday, Thursday, UTC, FixedZone};

// Match Go's "PST" zone at -8h and "PDT" at -7h.
fn pst() -> time::Location { FixedZone("PST", -8 * 3600) }

struct TimeFormatTest { time: time::Time, formatted: &'static str }

fn rfc3339_formats() -> slice<TimeFormatTest> {
    vec![
        TimeFormatTest { time: time::Date(2008, time::September, 17, 20, 4, 26, 0, UTC),
                         formatted: "2008-09-17T20:04:26Z" },
        TimeFormatTest { time: time::Date(1994, time::September, 17, 20, 4, 26, 0, FixedZone("EST", -18000)),
                         formatted: "1994-09-17T20:04:26-05:00" },
        TimeFormatTest { time: time::Date(2000, time::December, 26, 1, 15, 6, 0, FixedZone("OTO", 15600)),
                         formatted: "2000-12-26T01:15:06+04:20" },
    ].into()
}

test!{ fn TestRFC3339Conversion(t) {
    for f in rfc3339_formats() {
        let got = f.time.Format(RFC3339);
        if got != f.formatted {
            t.Errorf(Sprintf!("RFC3339: want=%s have=%s", f.formatted, got));
        }
    }
}}

struct AiTest { r#in: i32, width: i32, want: &'static str }

test!{ fn TestAppendInt(t) {
    let tests: slice<AiTest> = vec![
        AiTest { r#in: 0, width: 0, want: "0" },
        AiTest { r#in: 0, width: 1, want: "0" },
        AiTest { r#in: 0, width: 2, want: "00" },
        AiTest { r#in: 0, width: 3, want: "000" },
        AiTest { r#in: 1, width: 0, want: "1" },
        AiTest { r#in: 1, width: 1, want: "1" },
        AiTest { r#in: 1, width: 2, want: "01" },
        AiTest { r#in: 1, width: 3, want: "001" },
        AiTest { r#in: -1, width: 0, want: "-1" },
        AiTest { r#in: -1, width: 1, want: "-1" },
        AiTest { r#in: -1, width: 2, want: "-01" },
        AiTest { r#in: -1, width: 3, want: "-001" },
        AiTest { r#in: 99, width: 2, want: "99" },
        AiTest { r#in: 100, width: 2, want: "100" },
        AiTest { r#in: 1, width: 4, want: "0001" },
        AiTest { r#in: 12, width: 4, want: "0012" },
        AiTest { r#in: 123, width: 4, want: "0123" },
        AiTest { r#in: 1234, width: 4, want: "1234" },
        AiTest { r#in: 12345, width: 4, want: "12345" },
        AiTest { r#in: 1, width: 5, want: "00001" },
        AiTest { r#in: 123456, width: 5, want: "123456" },
        AiTest { r#in: 0, width: 9, want: "000000000" },
        AiTest { r#in: 123, width: 9, want: "000000123" },
        AiTest { r#in: 123456789, width: 9, want: "123456789" },
    ].into();
    for tt in &tests {
        let got = time::AppendInt(b"", tt.r#in as i64, tt.width as i64);
        let got_s = bytes::String(&got);
        if got_s != tt.want {
            t.Errorf(Sprintf!("appendInt(%d, %d) = %s, want %s", tt.r#in, tt.width, got_s, tt.want));
        }
    }
}}

struct FormatTest { name: &'static str, format: &'static str, result: &'static str }

test!{ fn TestFormat(t) {
    // The reference time is Thu Feb 4 21:00:57.012345600 PST 2009 =
    // 2009-02-05 05:00:57.012345600 UTC. We build it at PST directly so
    // the MST-slot replacement in UnixDate yields "PST".
    let tm = time::Date(2009, February, 4, 21, 0, 57, 12_345_600, pst());

    let tests: slice<FormatTest> = vec![
        FormatTest { name: "ANSIC", format: ANSIC, result: "Wed Feb  4 21:00:57 2009" },
        FormatTest { name: "UnixDate", format: UnixDate, result: "Wed Feb  4 21:00:57 PST 2009" },
        FormatTest { name: "RubyDate", format: RubyDate, result: "Wed Feb 04 21:00:57 -0800 2009" },
        FormatTest { name: "RFC822", format: RFC822, result: "04 Feb 09 21:00 PST" },
        FormatTest { name: "RFC850", format: RFC850, result: "Wednesday, 04-Feb-09 21:00:57 PST" },
        FormatTest { name: "RFC1123", format: RFC1123, result: "Wed, 04 Feb 2009 21:00:57 PST" },
        FormatTest { name: "RFC1123Z", format: RFC1123Z, result: "Wed, 04 Feb 2009 21:00:57 -0800" },
        FormatTest { name: "RFC3339", format: RFC3339, result: "2009-02-04T21:00:57-08:00" },
        FormatTest { name: "RFC3339Nano", format: RFC3339Nano, result: "2009-02-04T21:00:57.0123456-08:00" },
        FormatTest { name: "Kitchen", format: Kitchen, result: "9:00PM" },
        FormatTest { name: "am/pm", format: "3pm", result: "9pm" },
        FormatTest { name: "AM/PM", format: "3PM", result: "9PM" },
        FormatTest { name: "two-digit year", format: "06 01 02", result: "09 02 04" },
        FormatTest { name: "Janet", format: "Hi Janet, the Month is January", result: "Hi Janet, the Month is February" },
        FormatTest { name: "Stamp", format: Stamp, result: "Feb  4 21:00:57" },
        FormatTest { name: "StampMilli", format: StampMilli, result: "Feb  4 21:00:57.012" },
        FormatTest { name: "StampMicro", format: StampMicro, result: "Feb  4 21:00:57.012345" },
        FormatTest { name: "StampNano", format: StampNano, result: "Feb  4 21:00:57.012345600" },
        FormatTest { name: "DateTime", format: DateTime, result: "2009-02-04 21:00:57" },
        FormatTest { name: "DateOnly", format: DateOnly, result: "2009-02-04" },
        FormatTest { name: "TimeOnly", format: TimeOnly, result: "21:00:57" },
        FormatTest { name: "YearDay", format: "Jan  2 002 __2 2", result: "Feb  4 035  35 4" },
        FormatTest { name: "Year", format: "2006 6 06 _6 __6 ___6", result: "2009 6 09 _6 __6 ___6" },
        FormatTest { name: "Month", format: "Jan January 1 01 _1", result: "Feb February 2 02 _2" },
        FormatTest { name: "DayOfMonth", format: "2 02 _2 __2", result: "4 04  4  35" },
        FormatTest { name: "DayOfWeek", format: "Mon Monday", result: "Wed Wednesday" },
        FormatTest { name: "Hour", format: "15 3 03 _3", result: "21 9 09 _9" },
        FormatTest { name: "Minute", format: "4 04 _4", result: "0 00 _0" },
        FormatTest { name: "Second", format: "5 05 _5", result: "57 57 _57" },
    ].into();

    for test in &tests {
        let got = tm.Format(test.format);
        if got != test.result {
            t.Errorf(Sprintf!("%s expected %q got %q", test.name, test.result, got));
        }
    }
}}

test!{ fn TestFormatSingleDigits(t) {
    let tm = time::Date(2001, February, 3, 4, 5, 6, 700_000_000, UTC);
    let got = tm.Format("3:4:5");
    if got != "4:5:6" {
        t.Errorf(Sprintf!("expected %q got %q", "4:5:6", got));
    }
}}

test!{ fn TestFormatShortYear(t) {
    let years = [
        -100001, -100000, -99999,
        -10001, -10000, -9999,
        -1001, -1000, -999,
        -101, -100, -99,
        -11, -10, -9,
        -1, 0, 1,
        9, 10, 11,
        99, 100, 101,
        999, 1000, 1001,
        9999, 10000, 10001,
    ];
    for &y in &years {
        let tm = time::Date(y, January, 1, 0, 0, 0, 0, UTC);
        let result = tm.Format("2006.01.02");
        let want = if y < 0 {
            Sprintf!("-%04d.%02d.%02d", -y, 1, 1)
        } else {
            Sprintf!("%04d.%02d.%02d", y, 1, 1)
        };
        if result != want {
            t.Errorf(Sprintf!("(jan 1 %d).Format = %q, want %q", y, result, want));
        }
    }
}}

struct ParseTest {
    name: &'static str, format: &'static str, value: &'static str,
    has_tz: bool, has_wd: bool, year_sign: i32, frac_digits: usize,
}

fn parse_tests() -> slice<ParseTest> {
    vec![
        ParseTest { name:"ANSIC", format:ANSIC, value:"Thu Feb  4 21:00:57 2010", has_tz:false, has_wd:true, year_sign:1, frac_digits:0 },
        ParseTest { name:"UnixDate", format:UnixDate, value:"Thu Feb  4 21:00:57 PST 2010", has_tz:true, has_wd:true, year_sign:1, frac_digits:0 },
        ParseTest { name:"RubyDate", format:RubyDate, value:"Thu Feb 04 21:00:57 -0800 2010", has_tz:true, has_wd:true, year_sign:1, frac_digits:0 },
        ParseTest { name:"RFC850", format:RFC850, value:"Thursday, 04-Feb-10 21:00:57 PST", has_tz:true, has_wd:true, year_sign:1, frac_digits:0 },
        ParseTest { name:"RFC1123", format:RFC1123, value:"Thu, 04 Feb 2010 21:00:57 PST", has_tz:true, has_wd:true, year_sign:1, frac_digits:0 },
        ParseTest { name:"RFC1123Z", format:RFC1123Z, value:"Thu, 04 Feb 2010 21:00:57 -0800", has_tz:true, has_wd:true, year_sign:1, frac_digits:0 },
        ParseTest { name:"RFC3339", format:RFC3339, value:"2010-02-04T21:00:57-08:00", has_tz:true, has_wd:false, year_sign:1, frac_digits:0 },
        ParseTest { name:"custom: \"2006-01-02 15:04:05-07\"", format:"2006-01-02 15:04:05-07", value:"2010-02-04 21:00:57-08", has_tz:true, has_wd:false, year_sign:1, frac_digits:0 },
        // Optional fractional seconds.
        ParseTest { name:"ANSIC-frac1", format:ANSIC, value:"Thu Feb  4 21:00:57.0 2010", has_tz:false, has_wd:true, year_sign:1, frac_digits:1 },
        ParseTest { name:"UnixDate-frac2", format:UnixDate, value:"Thu Feb  4 21:00:57.01 PST 2010", has_tz:true, has_wd:true, year_sign:1, frac_digits:2 },
        ParseTest { name:"RubyDate-frac3", format:RubyDate, value:"Thu Feb 04 21:00:57.012 -0800 2010", has_tz:true, has_wd:true, year_sign:1, frac_digits:3 },
        ParseTest { name:"RFC3339-frac9", format:RFC3339, value:"2010-02-04T21:00:57.012345678-08:00", has_tz:true, has_wd:false, year_sign:1, frac_digits:9 },
        // Amount of white space should not matter.
        ParseTest { name:"ANSIC-ws1", format:ANSIC, value:"Thu Feb 4 21:00:57 2010", has_tz:false, has_wd:true, year_sign:1, frac_digits:0 },
        ParseTest { name:"ANSIC-ws2", format:ANSIC, value:"Thu      Feb     4     21:00:57     2010", has_tz:false, has_wd:true, year_sign:1, frac_digits:0 },
        // Case should not matter
        ParseTest { name:"ANSIC-upper", format:ANSIC, value:"THU FEB 4 21:00:57 2010", has_tz:false, has_wd:true, year_sign:1, frac_digits:0 },
        ParseTest { name:"ANSIC-lower", format:ANSIC, value:"thu feb 4 21:00:57 2010", has_tz:false, has_wd:true, year_sign:1, frac_digits:0 },
        // Fractional seconds with explicit layout.
        ParseTest { name:"millisecond:: dot", format:"Mon Jan _2 15:04:05.000 2006", value:"Thu Feb  4 21:00:57.012 2010", has_tz:false, has_wd:true, year_sign:1, frac_digits:3 },
        ParseTest { name:"microsecond:: dot", format:"Mon Jan _2 15:04:05.000000 2006", value:"Thu Feb  4 21:00:57.012345 2010", has_tz:false, has_wd:true, year_sign:1, frac_digits:6 },
        ParseTest { name:"nanosecond:: dot", format:"Mon Jan _2 15:04:05.000000000 2006", value:"Thu Feb  4 21:00:57.012345678 2010", has_tz:false, has_wd:true, year_sign:1, frac_digits:9 },
        ParseTest { name:"millisecond:: comma", format:"Mon Jan _2 15:04:05,000 2006", value:"Thu Feb  4 21:00:57.012 2010", has_tz:false, has_wd:true, year_sign:1, frac_digits:3 },
        // Day of year.
        ParseTest { name:"yday1", format:"2006-01-02 002 15:04:05", value:"2010-02-04 035 21:00:57", has_tz:false, has_wd:false, year_sign:1, frac_digits:0 },
        ParseTest { name:"yday3", format:"2006-002 15:04:05", value:"2010-035 21:00:57", has_tz:false, has_wd:false, year_sign:1, frac_digits:0 },
        // Time zone offsets
        ParseTest { name:"Z07-Z", format:"2006-01-02T15:04:05Z07", value:"2010-02-04T21:00:57Z", has_tz:false, has_wd:false, year_sign:1, frac_digits:0 },
        ParseTest { name:"Z07-+08", format:"2006-01-02T15:04:05Z07", value:"2010-02-04T21:00:57+08", has_tz:false, has_wd:false, year_sign:1, frac_digits:0 },
        ParseTest { name:"Z07-m08", format:"2006-01-02T15:04:05Z07", value:"2010-02-04T21:00:57-08", has_tz:true, has_wd:false, year_sign:1, frac_digits:0 },
        ParseTest { name:"Z0700-Z", format:"2006-01-02T15:04:05Z0700", value:"2010-02-04T21:00:57Z", has_tz:false, has_wd:false, year_sign:1, frac_digits:0 },
        ParseTest { name:"Z0700-+0800", format:"2006-01-02T15:04:05Z0700", value:"2010-02-04T21:00:57+0800", has_tz:false, has_wd:false, year_sign:1, frac_digits:0 },
        ParseTest { name:"Z0700-m0800", format:"2006-01-02T15:04:05Z0700", value:"2010-02-04T21:00:57-0800", has_tz:true, has_wd:false, year_sign:1, frac_digits:0 },
        ParseTest { name:"Z0700:00-Z", format:"2006-01-02T15:04:05Z07:00", value:"2010-02-04T21:00:57Z", has_tz:false, has_wd:false, year_sign:1, frac_digits:0 },
        ParseTest { name:"Z0700:00-+0800", format:"2006-01-02T15:04:05Z07:00", value:"2010-02-04T21:00:57+08:00", has_tz:false, has_wd:false, year_sign:1, frac_digits:0 },
        ParseTest { name:"Z0700:00-m0800", format:"2006-01-02T15:04:05Z07:00", value:"2010-02-04T21:00:57-08:00", has_tz:true, has_wd:false, year_sign:1, frac_digits:0 },
    ].into()
}

fn check_time(tm: &time::Time, test: &ParseTest, t: &testing::T) {
    if test.year_sign >= 0 && test.year_sign * tm.Year() as i32 != 2010 {
        t.Errorf(Sprintf!("%s: bad year: %d not %d", test.name, tm.Year(), 2010));
    }
    if tm.Month() != February {
        t.Errorf(Sprintf!("%s: bad month: %s not %s", test.name, tm.Month(), February));
    }
    if tm.Day() != 4 {
        t.Errorf(Sprintf!("%s: bad day: %d not %d", test.name, tm.Day(), 4));
    }
    if tm.Hour() != 21 {
        t.Errorf(Sprintf!("%s: bad hour: %d not %d", test.name, tm.Hour(), 21));
    }
    if tm.Minute() != 0 {
        t.Errorf(Sprintf!("%s: bad minute: %d not %d", test.name, tm.Minute(), 0));
    }
    if tm.Second() != 57 {
        t.Errorf(Sprintf!("%s: bad second: %d not %d", test.name, tm.Second(), 57));
    }
    // Nanoseconds must be checked against the precision of the input.
    let src = "012345678";
    let pad = "000000000";
    let nsec_str = Sprintf!("%v%v", &src[..test.frac_digits], &pad[..9 - test.frac_digits]);
    let nsec: i64 = nsec_str.parse().unwrap();
    if tm.Nanosecond() as i64 != nsec {
        t.Errorf(Sprintf!("%s: bad nanosecond: %d not %d", test.name, tm.Nanosecond(), nsec));
    }
    let (_, offset) = tm.Zone();
    if test.has_tz && offset != -28800 {
        t.Errorf(Sprintf!("%s: bad tz offset: %d not %d", test.name, offset, -28800));
    }
    if test.has_wd && tm.Weekday() != Thursday {
        t.Errorf(Sprintf!("%s: bad weekday: %s not %s", test.name, tm.Weekday(), Thursday));
    }
}

test!{ fn TestParse(t) {
    for test in &parse_tests() {
        let (tm, err) = time::Parse(test.format, test.value);
        if err != nil {
            t.Errorf(Sprintf!("%s error: %s", test.name, err));
        } else {
            check_time(&tm, test, t);
        }
    }
}}

struct DayORTest { date: &'static str, ok: bool }

test!{ fn TestParseDayOutOfRange(t) {
    let tests: slice<DayORTest> = vec![
        DayORTest { date: "Thu Jan 99 21:00:57 2010", ok: false },
        DayORTest { date: "Thu Jan 31 21:00:57 2010", ok: true },
        DayORTest { date: "Thu Jan 32 21:00:57 2010", ok: false },
        DayORTest { date: "Thu Feb 28 21:00:57 2012", ok: true },
        DayORTest { date: "Thu Feb 29 21:00:57 2012", ok: true },
        DayORTest { date: "Thu Feb 29 21:00:57 2010", ok: false },
        DayORTest { date: "Thu Mar 31 21:00:57 2010", ok: true },
        DayORTest { date: "Thu Mar 32 21:00:57 2010", ok: false },
        DayORTest { date: "Thu Apr 30 21:00:57 2010", ok: true },
        DayORTest { date: "Thu Apr 31 21:00:57 2010", ok: false },
        DayORTest { date: "Thu Dec 31 21:00:57 2010", ok: true },
        DayORTest { date: "Thu Dec 32 21:00:57 2010", ok: false },
        DayORTest { date: "Thu Dec 00 21:00:57 2010", ok: false },
    ].into();
    for test in &tests {
        let (_, err) = time::Parse(ANSIC, test.date);
        match (test.ok, err == nil) {
            (true, true) => {}
            (false, false) => {
                let es = Sprintf!("%v", err);
                if !strings::Contains(&es, "day out of range") && !strings::Contains(&es, "month out of range") && !strings::Contains(&es, "cannot parse") {
                    t.Errorf(Sprintf!("%q: expected 'day' error, got %s", test.date, err));
                }
            }
            (true, false) => t.Errorf(Sprintf!("%q: unexpected error: %s", test.date, err)),
            (false, true) => t.Errorf(Sprintf!("%q: expected 'day' error, got none", test.date)),
        }
    }
}}

test!{ fn TestNoonIs12PM(t) {
    let noon = time::Date(0, January, 1, 12, 0, 0, 0, UTC);
    let expect = "12:00PM";
    let got = noon.Format("3:04PM");
    if got != expect { t.Errorf(Sprintf!("got %q; expect %q", got, expect)); }
    let got = noon.Format("03:04PM");
    if got != expect { t.Errorf(Sprintf!("got %q; expect %q", got, expect)); }
}}

test!{ fn TestMidnightIs12AM(t) {
    let midnight = time::Date(0, January, 1, 0, 0, 0, 0, UTC);
    let expect = "12:00AM";
    let got = midnight.Format("3:04PM");
    if got != expect { t.Errorf(Sprintf!("got %q; expect %q", got, expect)); }
    let got = midnight.Format("03:04PM");
    if got != expect { t.Errorf(Sprintf!("got %q; expect %q", got, expect)); }
}}

test!{ fn Test12PMIsNoon(t) {
    let (noon, err) = time::Parse("3:04PM", "12:00PM");
    if err != nil { t.Fatal(Sprintf!("error: %s", err)); }
    if noon.Hour() != 12 { t.Errorf(Sprintf!("got %d; expect 12", noon.Hour())); }
    let (noon, err) = time::Parse("03:04PM", "12:00PM");
    if err != nil { t.Fatal(Sprintf!("error: %s", err)); }
    if noon.Hour() != 12 { t.Errorf(Sprintf!("got %d; expect 12", noon.Hour())); }
}}

test!{ fn Test12AMIsMidnight(t) {
    let (midnight, err) = time::Parse("3:04PM", "12:00AM");
    if err != nil { t.Fatal(Sprintf!("error: %s", err)); }
    if midnight.Hour() != 0 { t.Errorf(Sprintf!("got %d; expect 0", midnight.Hour())); }
    let (midnight, err) = time::Parse("03:04PM", "12:00AM");
    if err != nil { t.Fatal(Sprintf!("error: %s", err)); }
    if midnight.Hour() != 0 { t.Errorf(Sprintf!("got %d; expect 0", midnight.Hour())); }
}}

test!{ fn TestMissingZone(t) {
    let (tm, err) = time::Parse(RubyDate, "Thu Feb 02 16:10:03 -0500 2006");
    if err != nil { t.Fatal(Sprintf!("error parsing: %s", err)); }
    let expect = "Thu Feb  2 16:10:03 -0500 2006";
    let got = tm.Format(UnixDate);
    if got != expect {
        t.Errorf(Sprintf!("got %s; expect %s", got, expect));
    }
}}

test!{ fn TestMinutesInTimeZone(t) {
    let (tm, err) = time::Parse(RubyDate, "Mon Jan 02 15:04:05 +0123 2006");
    if err != nil { t.Fatal(Sprintf!("error parsing: %s", err)); }
    let expected = (1 * 60 + 23) * 60;
    let (_, offset) = tm.Zone();
    if offset != expected {
        t.Errorf(Sprintf!("ZoneOffset = %d, want %d", offset, expected));
    }
}}

struct SecTzTest { format: &'static str, value: &'static str, expected_offset: i64 }

fn seconds_tz_tests() -> slice<SecTzTest> {
    vec![
        SecTzTest { format: "2006-01-02T15:04:05-070000", value: "1871-01-01T05:33:02-003408", expected_offset: -(34*60 + 8) },
        SecTzTest { format: "2006-01-02T15:04:05-07:00:00", value: "1871-01-01T05:33:02-00:34:08", expected_offset: -(34*60 + 8) },
        SecTzTest { format: "2006-01-02T15:04:05-070000", value: "1871-01-01T05:33:02+003408", expected_offset: 34*60 + 8 },
        SecTzTest { format: "2006-01-02T15:04:05-07:00:00", value: "1871-01-01T05:33:02+00:34:08", expected_offset: 34*60 + 8 },
        SecTzTest { format: "2006-01-02T15:04:05Z070000", value: "1871-01-01T05:33:02-003408", expected_offset: -(34*60 + 8) },
        SecTzTest { format: "2006-01-02T15:04:05Z07:00:00", value: "1871-01-01T05:33:02+00:34:08", expected_offset: 34*60 + 8 },
        SecTzTest { format: "2006-01-02T15:04:05-07", value: "1871-01-01T05:33:02+01", expected_offset: 1 * 60 * 60 },
        SecTzTest { format: "2006-01-02T15:04:05-07", value: "1871-01-01T05:33:02-02", expected_offset: -2 * 60 * 60 },
        SecTzTest { format: "2006-01-02T15:04:05Z07", value: "1871-01-01T05:33:02-02", expected_offset: -2 * 60 * 60 },
    ].into()
}

test!{ fn TestParseSecondsInTimeZone(t) {
    for test in &seconds_tz_tests() {
        let (tm, err) = time::Parse(test.format, test.value);
        if err != nil { t.Fatal(Sprintf!("error parsing: %s", err)); }
        let (_, offset) = tm.Zone();
        if offset != test.expected_offset {
            t.Errorf(Sprintf!("ZoneOffset = %d, want %d", offset, test.expected_offset));
        }
    }
}}

test!{ fn TestFormatSecondsInTimeZone(t) {
    for test in &seconds_tz_tests() {
        let d = time::Date(1871, January, 1, 5, 33, 2, 0, FixedZone("LMT", test.expected_offset));
        let timestr = d.Format(test.format);
        if timestr != test.value {
            t.Errorf(Sprintf!("Format = %s, want %s", timestr, test.value));
        }
    }
}}

test!{ fn TestUnderscoreTwoThousand(t) {
    let format = "15:04_20060102";
    let input = "14:38_20150618";
    let (tm, err) = time::Parse(format, input);
    if err != nil { t.Error(Sprintf!("%s", err)); }
    let (y, m, d) = tm.Date();
    if y != 2015 || m.0 != 6 || d != 18 {
        t.Errorf(Sprintf!("Incorrect y/m/d, got %d/%d/%d", y, m.0, d));
    }
    if tm.Hour() != 14 { t.Errorf(Sprintf!("Incorrect hour, got %d", tm.Hour())); }
    if tm.Minute() != 38 { t.Errorf(Sprintf!("Incorrect minute, got %d", tm.Minute())); }
}}

struct MonthORTest { value: &'static str, ok: bool }

test!{ fn TestParseMonthOutOfRange(t) {
    let tests: slice<MonthORTest> = vec![
        MonthORTest { value: "00-01", ok: false },
        MonthORTest { value: "13-01", ok: false },
        MonthORTest { value: "01-01", ok: true },
    ].into();
    for test in &tests {
        let (_, err) = time::Parse("01-02", test.value);
        match (test.ok, err == nil) {
            (true, true) => {}
            (false, false) => {
                let es = Sprintf!("%v", err);
                if !strings::Contains(&es, "month out of range") && !strings::Contains(&es, "cannot parse") {
                    t.Errorf(Sprintf!("%q: expected 'month' error, got %s", test.value, err));
                }
            }
            (true, false) => t.Errorf(Sprintf!("%q: unexpected error: %s", test.value, err)),
            (false, true) => t.Errorf(Sprintf!("%q: expected 'month' error, got none", test.value)),
        }
    }
}}

test!{ fn TestParseYday(t) {
    for i in 1..=365i64 {
        let d = Sprintf!("2020-%03d", i);
        let (tm, err) = time::Parse("2006-002", &d);
        if err != nil {
            t.Errorf(Sprintf!("unexpected error for %s: %s", d, err));
        } else if tm.Year() != 2020 || tm.YearDay() != i {
            t.Errorf(Sprintf!("got year %d yearday %d, want 2020 %d", tm.Year(), tm.YearDay(), i));
        }
    }
}}

struct QuoteT { s: &'static str, want: &'static str }

test!{ fn TestQuote(t) {
    let tests = vec![
        QuoteT { s: "\"", want: "\"\\\"\"" },
        QuoteT { s: "abc\"xyz\"", want: "\"abc\\\"xyz\\\"\"" },
        QuoteT { s: "", want: "\"\"" },
        QuoteT { s: "abc", want: "\"abc\"" },
        QuoteT { s: "\u{263A}", want: "\"\\xe2\\x98\\xba\"" },
        QuoteT { s: "\u{263A} hello \u{263A} hello", want: "\"\\xe2\\x98\\xba hello \\xe2\\x98\\xba hello\"" },
        QuoteT { s: "\x04", want: "\"\\x04\"" },
    ];
    for tt in &tests {
        let q = time::Quote(tt.s);
        if q != tt.want {
            t.Errorf(Sprintf!("Quote(%q) = got %q, want %q", tt.s, q, tt.want));
        }
    }
}}

struct FracSepT { s: &'static str, want: &'static str }

test!{ fn TestFormatFractionalSecondSeparators(t) {
    let tests = vec![
        FracSepT { s: "15:04:05.000", want: "21:00:57.012" },
        FracSepT { s: "15:04:05.999", want: "21:00:57.012" },
        FracSepT { s: "15:04:05,000", want: "21:00:57,012" },
        FracSepT { s: "15:04:05,999", want: "21:00:57,012" },
    ];
    // Thu Feb 4 21:00:57.012345600 PST 2009 as PST.
    let tm = time::Date(2009, February, 4, 21, 0, 57, 12_345_600, pst());
    for tt in &tests {
        let q = tm.Format(tt.s);
        if q != tt.want {
            t.Errorf(Sprintf!("Format(%q) = got %q, want %q", tt.s, q, tt.want));
        }
    }
}}

struct LongFracT { value: &'static str, want: i64 }

test!{ fn TestParseFractionalSecondsLongerThanNineDigits(t) {
    let tests = vec![
        LongFracT { value: "2021-09-29T16:04:33.000000000Z", want: 0 },
        LongFracT { value: "2021-09-29T16:04:33.000000001Z", want: 1 },
        LongFracT { value: "2021-09-29T16:04:33.100000000Z", want: 100_000_000 },
        LongFracT { value: "2021-09-29T16:04:33.100000001Z", want: 100_000_001 },
        LongFracT { value: "2021-09-29T16:04:33.999999999Z", want: 999_999_999 },
        LongFracT { value: "2021-09-29T16:04:33.012345678Z", want: 12_345_678 },
        // 10+ digits, truncates to 9.
        LongFracT { value: "2021-09-29T16:04:33.0000000000Z", want: 0 },
        LongFracT { value: "2021-09-29T16:04:33.0000000001Z", want: 0 },
        LongFracT { value: "2021-09-29T16:04:33.1000000000Z", want: 100_000_000 },
        LongFracT { value: "2021-09-29T16:04:33.1000000009Z", want: 100_000_000 },
        LongFracT { value: "2021-09-29T16:04:33.9999999999Z", want: 999_999_999 },
        LongFracT { value: "2021-09-29T16:04:33.0123456789Z", want: 12_345_678 },
    ];
    for tt in &tests {
        for format in &[RFC3339, RFC3339Nano] {
            let (tm, err) = time::Parse(format, tt.value);
            if err != nil {
                t.Errorf(Sprintf!("Parse(%q, %q) error: %s", format, tt.value, err));
                continue;
            }
            if tm.Nanosecond() as i64 != tt.want {
                t.Errorf(Sprintf!("Parse(%q, %q) = got %d, want %d", format, tt.value, tm.Nanosecond(), tt.want));
            }
        }
    }
}}

// ─── ParseTimeZone smoke-subset ───────────────────────────────────────
//
// The full table has 30+ cases with tricky GMT offset handling; this
// subset covers the deterministic cases.

struct TzT { value: &'static str, length: i64, ok: bool }

test!{ fn TestParseTimeZone(t) {
    let tests = vec![
        TzT { value: "gmt hi there", length: 0, ok: false },
        TzT { value: "GMT hi there", length: 3, ok: true },
        TzT { value: "GMT+12 hi there", length: 6, ok: true },
        TzT { value: "GMT-5 hi there", length: 5, ok: true },
        TzT { value: "ChST hi there", length: 4, ok: true },
        TzT { value: "MeST hi there", length: 4, ok: true },
        TzT { value: "MSDx", length: 3, ok: true },
        TzT { value: "MSDY", length: 0, ok: false },
        TzT { value: "ESAST hi", length: 5, ok: true },
        TzT { value: "ESASTT hi", length: 0, ok: false },
        TzT { value: "WITA hi", length: 4, ok: true },
        TzT { value: "+03 hi", length: 3, ok: true },
        TzT { value: "-04 hi", length: 3, ok: true },
        TzT { value: "+23", length: 3, ok: true },
        TzT { value: "+24", length: 0, ok: false },
        TzT { value: "-23", length: 3, ok: true },
        TzT { value: "-24", length: 0, ok: false },
    ];
    for test in &tests {
        let (length, ok) = time::ParseTimeZone(test.value);
        if ok != test.ok {
            t.Errorf(Sprintf!("expected %t for %q got %t", test.ok, test.value, ok));
        } else if length != test.length {
            t.Errorf(Sprintf!("expected %d for %q got %d", test.length, test.value, length));
        }
    }
}}

// Check we use the correct weekday for known dates.
test!{ fn TestKnownWeekdays(t) {
    let tm = time::Date(2009, February, 4, 21, 0, 57, 0, UTC);
    if tm.Weekday() != Wednesday {
        t.Errorf(Sprintf!("2009-02-04 weekday: got %s want Wednesday", tm.Weekday()));
    }
    let tm = time::Date(2010, February, 4, 21, 0, 57, 0, UTC);
    if tm.Weekday() != Thursday {
        t.Errorf(Sprintf!("2010-02-04 weekday: got %s want Thursday", tm.Weekday()));
    }
}}
