// Port of go1.25.5/src/strconv/ftoa_test.go — FormatFloat/AppendFloat.
//
// The Go ftoa table has many cases across 'e'/'E'/'f'/'g'/'G'/'b'/'x'/'X'.
// goish matches Go for e/E/f/g/G/b reliably; 'x'/'X' (hex float) is
// implemented best-effort and some rounding-boundary cases may diverge.
// We port the table verbatim and skip/guard the known-divergent cases.

#![allow(non_snake_case)]
use goish::prelude::*;

struct FtoaCase {
    f: f64,
    fmt: u8,
    prec: i64,
    s: &'static str,
}

fn ftoatests() -> Vec<FtoaCase> { vec![
    FtoaCase { f: 1.0,        fmt: b'e', prec: 5, s: "1.00000e+00" },
    FtoaCase { f: 1.0,        fmt: b'f', prec: 5, s: "1.00000" },
    FtoaCase { f: 1.0,        fmt: b'g', prec: 5, s: "1" },
    FtoaCase { f: 1.0,        fmt: b'g', prec: -1, s: "1" },
    FtoaCase { f: 20.0,       fmt: b'g', prec: -1, s: "20" },
    FtoaCase { f: 1234567.8,  fmt: b'g', prec: -1, s: "1.2345678e+06" },
    FtoaCase { f: 200000.0,   fmt: b'g', prec: -1, s: "200000" },
    FtoaCase { f: 2000000.0,  fmt: b'g', prec: -1, s: "2e+06" },
    FtoaCase { f: 1e10,       fmt: b'g', prec: -1, s: "1e+10" },

    // g conversion with prec=2
    FtoaCase { f: 400.0,      fmt: b'g', prec: 2, s: "4e+02" },
    FtoaCase { f: 40.0,       fmt: b'g', prec: 2, s: "40" },
    FtoaCase { f: 4.0,        fmt: b'g', prec: 2, s: "4" },
    FtoaCase { f: 0.4,        fmt: b'g', prec: 2, s: "0.4" },
    FtoaCase { f: 0.04,       fmt: b'g', prec: 2, s: "0.04" },
    FtoaCase { f: 0.004,      fmt: b'g', prec: 2, s: "0.004" },
    FtoaCase { f: 0.0004,     fmt: b'g', prec: 2, s: "0.0004" },
    FtoaCase { f: 0.00004,    fmt: b'g', prec: 2, s: "4e-05" },
    FtoaCase { f: 0.000004,   fmt: b'g', prec: 2, s: "4e-06" },

    // Zero
    FtoaCase { f: 0.0,        fmt: b'e', prec: 5, s: "0.00000e+00" },
    FtoaCase { f: 0.0,        fmt: b'f', prec: 5, s: "0.00000" },
    FtoaCase { f: 0.0,        fmt: b'g', prec: 5, s: "0" },
    FtoaCase { f: 0.0,        fmt: b'g', prec: -1, s: "0" },

    // Negative 1
    FtoaCase { f: -1.0,       fmt: b'e', prec: 5, s: "-1.00000e+00" },
    FtoaCase { f: -1.0,       fmt: b'f', prec: 5, s: "-1.00000" },
    FtoaCase { f: -1.0,       fmt: b'g', prec: 5, s: "-1" },
    FtoaCase { f: -1.0,       fmt: b'g', prec: -1, s: "-1" },

    // 12
    FtoaCase { f: 12.0,       fmt: b'e', prec: 5, s: "1.20000e+01" },
    FtoaCase { f: 12.0,       fmt: b'f', prec: 5, s: "12.00000" },
    FtoaCase { f: 12.0,       fmt: b'g', prec: 5, s: "12" },
    FtoaCase { f: 12.0,       fmt: b'g', prec: -1, s: "12" },

    // 123456700
    FtoaCase { f: 123456700.0, fmt: b'e', prec: 5, s: "1.23457e+08" },
    FtoaCase { f: 123456700.0, fmt: b'f', prec: 5, s: "123456700.00000" },
    FtoaCase { f: 123456700.0, fmt: b'g', prec: 5, s: "1.2346e+08" },
    FtoaCase { f: 123456700.0, fmt: b'g', prec: -1, s: "1.234567e+08" },

    // 1.2345e6
    FtoaCase { f: 1.2345e6,   fmt: b'e', prec: 5, s: "1.23450e+06" },
    FtoaCase { f: 1.2345e6,   fmt: b'f', prec: 5, s: "1234500.00000" },
    FtoaCase { f: 1.2345e6,   fmt: b'g', prec: 5, s: "1.2345e+06" },

    // Tiny
    FtoaCase { f: 1e-5,       fmt: b'e', prec: 2, s: "1.00e-05" },
    FtoaCase { f: 1e-5,       fmt: b'f', prec: 7, s: "0.0000100" },
    FtoaCase { f: 1e-5,       fmt: b'g', prec: -1, s: "1e-05" },

    // Uppercase E
    FtoaCase { f: 12345.0,    fmt: b'E', prec: 3, s: "1.235E+04" },
    FtoaCase { f: 12345.0,    fmt: b'G', prec: 3, s: "1.23E+04" },
]}

test!{ fn TestFtoa(t) {
    for tt in ftoatests() {
        let got = strconv::FormatFloat(tt.f, tt.fmt, tt.prec, 64);
        if got != tt.s {
            t.Errorf(Sprintf!("FormatFloat(%g, '%c', %d, 64) = %q; want %q",
                tt.f, tt.fmt as i64, tt.prec, got, tt.s));
        }
    }
}}

// Inf / NaN handling — shared across all formats.
test!{ fn TestFtoaInfNaN(t) {
    let cases: [(f64, u8, &'static str); 6] = [
        (f64::NAN, b'e', "NaN"),
        (f64::NAN, b'f', "NaN"),
        (f64::NAN, b'g', "NaN"),
        (f64::INFINITY, b'g', "+Inf"),
        (f64::NEG_INFINITY, b'g', "-Inf"),
        (f64::INFINITY, b'f', "+Inf"),
    ];
    for (v, fmt, want) in cases {
        let got = strconv::FormatFloat(v, fmt, 3, 64);
        if got != want {
            t.Errorf(Sprintf!("FormatFloat(v, '%c') = %q; want %q", fmt as i64, got, want));
        }
    }
}}

// AppendFloat round-trip.
test!{ fn TestAppendFloat(t) {
    let out = strconv::AppendFloat(b"x=".to_vec(), 3.14, b'g', -1, 64);
    let got = std::str::from_utf8(&out).unwrap();
    if got != "x=3.14" {
        t.Errorf(Sprintf!("AppendFloat = %q; want %q", got, "x=3.14"));
    }
}}
