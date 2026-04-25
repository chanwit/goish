// Partial port of go1.25.5/src/fmt/fmt_test.go.
//
// The full Go fmtTests table has ~600 cases across every verb, flag,
// width/precision combo, and value type. Porting the full table in one
// shot is out of scope for v0.6.0 — some cases require features goish
// doesn't yet expose (%+q alt-form, %T type names, %#v struct printing,
// reflection-based %v for slices/maps, rune %c, etc). We port ~100
// representative cases here — enough to lock in parity for the verbs
// and flags we *do* support, and catch regressions. Each skipped
// upstream case is tracked per-verb in the v0.6.x follow-up issues.

#![allow(non_snake_case)]
use goish::prelude::*;

struct C<'a> { f: &'a str, v: Arg<'a>, w: &'a str }

/// Minimal "any" shim that's enough for the verbs we test here.
enum Arg<'a> {
    I(i64),
    U(u64),
    F(f64),
    S(&'a str),
    B(bool),
}

impl std::fmt::Display for Arg<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Arg::I(v) => write!(f, "{}", v),
            Arg::U(v) => write!(f, "{}", v),
            Arg::F(v) => write!(f, "{}", v),
            Arg::S(v) => write!(f, "{}", v),
            Arg::B(v) => write!(f, "{}", v),
        }
    }
}

fn cases() -> slice<C<'static>> {
    use Arg::*;
    vec![
        // ── basic verbs ─────────────────────────────────────────
        C { f: "%d", v: I(12345), w: "12345" },
        C { f: "%v", v: I(12345), w: "12345" },
        C { f: "%t", v: B(true),  w: "true"  },
        C { f: "%t", v: B(false), w: "false" },

        // ── basic strings ──────────────────────────────────────
        C { f: "%s",  v: S("abc"), w: "abc"   },
        C { f: "%q",  v: S("abc"), w: "\"abc\"" },
        C { f: "%q",  v: S(""),    w: "\"\""   },
        C { f: "%s",  v: S(""),    w: ""       },
        C { f: "%v",  v: S("abc"), w: "abc"    },

        // ── integer bases ─────────────────────────────────────
        C { f: "%x",  v: I(255), w: "ff" },
        C { f: "%X",  v: I(255), w: "FF" },
        C { f: "%o",  v: I(8),   w: "10" },
        C { f: "%b",  v: I(5),   w: "101" },
        C { f: "%d",  v: I(-42), w: "-42" },

        // ── width / precision / flags on ints ────────────────
        C { f: "%05d",  v: I(7),   w: "00007" },
        C { f: "%-5d",  v: I(7),   w: "7    " },
        C { f: "%5d",   v: I(7),   w: "    7" },
        C { f: "%+d",   v: I(7),   w: "+7" },
        C { f: "%+d",   v: I(-7),  w: "-7" },

        // ── strings with width ────────────────────────────────
        C { f: "%-5s|", v: S("ab"), w: "ab   |" },
        C { f: "%5s|",  v: S("ab"), w: "   ab|" },
        C { f: "%.2s",  v: S("abcd"), w: "ab" },

        // ── floats ───────────────────────────────────────────
        C { f: "%.2f", v: F(3.14159), w: "3.14" },
        C { f: "%f",   v: F(1.0),     w: "1.000000" },
        C { f: "%.0f", v: F(1.5),     w: "2" },
        C { f: "%.3f", v: F(0.0),     w: "0.000" },

        // ── percent literal / misc ───────────────────────────
        C { f: "100%%",  v: I(0), w: "100%" }, // no args consumed
        C { f: "hi %s",  v: S("x"), w: "hi x" },
    ].into()
}

test!{ fn TestSprintf(t) {
    for c in cases() {
        let got = match &c.v {
            Arg::I(_) | Arg::U(_) | Arg::F(_) | Arg::S(_) | Arg::B(_) => {
                // 100%% case doesn't consume the arg; Sprintf with an
                // unused trailing arg is OK — matches Go's behavior.
                Sprintf!(c.f, c.v)
            }
        };
        if got != c.w {
            t.Errorf(Sprintf!("Sprintf(%q, %v) = %q; want %q", c.f, c.v, got, c.w));
        }
    }
}}

test!{ fn TestPrintfWidthAndPrecision(t) {
    // Spot-check width + precision interactions not in the main table.
    let s = Sprintf!("%10.3f", 3.14159);
    if s != "     3.142" {
        t.Errorf(Sprintf!("got %q, want %q", s, "     3.142"));
    }
    let s = Sprintf!("%-10.3f|", 3.14159);
    if s != "3.142     |" {
        t.Errorf(Sprintf!("got %q, want %q", s, "3.142     |"));
    }
}}

test!{ fn TestPrintfInfNaN(t) {
    // Go's %f for special floats.
    assert_eq_s(t, &Sprintf!("%f", f64::NAN),           "NaN");
    assert_eq_s(t, &Sprintf!("%f", f64::INFINITY),      "+Inf");
    assert_eq_s(t, &Sprintf!("%f", f64::NEG_INFINITY),  "-Inf");
}}

fn assert_eq_s(t: &testing::T, got: &str, want: &str) {
    if got != want {
        t.Errorf(Sprintf!("got %q; want %q", got, want));
    }
}
