// Port of selected go1.25.5 src/math/*_test.go cases — the ones that
// exercise API correctness without needing tight float bit-for-bit
// equality (Go's math tests use ulp-level tolerances we don't match).

#![allow(non_snake_case)]
use goish::prelude::*;

fn close(a: f64, b: f64, eps: f64) -> bool { (a - b).abs() < eps }

test!{ fn TestConstants(t) {
    if !close(math::Pi, 3.141592653589793, 1e-14) {
        t.Errorf(Sprintf!("math::Pi = %g", math::Pi));
    }
    if !close(math::E, 2.718281828459045, 1e-14) {
        t.Errorf(Sprintf!("math::E = %g", math::E));
    }
    if !close(math::Sqrt2, 1.4142135623730951, 1e-14) {
        t.Errorf(Sprintf!("math::Sqrt2 = %g", math::Sqrt2));
    }
}}

test!{ fn TestAbs(t) {
    let cases = [(1.0, 1.0), (-1.0, 1.0), (0.0, 0.0), (-3.14, 3.14)];
    for (inp, want) in cases {
        let got = math::Abs(inp);
        if !close(got, want, 1e-15) {
            t.Errorf(Sprintf!("Abs(%g) = %g, want %g", inp, got, want));
        }
    }
}}

test!{ fn TestSqrt(t) {
    let cases = [(0.0, 0.0), (1.0, 1.0), (4.0, 2.0), (9.0, 3.0), (2.0, math::Sqrt2)];
    for (inp, want) in cases {
        let got = math::Sqrt(inp);
        if !close(got, want, 1e-14) {
            t.Errorf(Sprintf!("Sqrt(%g) = %g, want %g", inp, got, want));
        }
    }
}}

test!{ fn TestPow(t) {
    let cases = [
        (2.0, 0.0, 1.0),
        (2.0, 1.0, 2.0),
        (2.0, 10.0, 1024.0),
        (3.0, 2.0, 9.0),
        (0.5, 2.0, 0.25),
    ];
    for (x, y, want) in cases {
        let got = math::Pow(x, y);
        if !close(got, want, 1e-12) {
            t.Errorf(Sprintf!("Pow(%g, %g) = %g, want %g", x, y, got, want));
        }
    }
}}

test!{ fn TestTrig(t) {
    if !close(math::Sin(0.0), 0.0, 1e-15) { t.Errorf(Sprintf!("Sin(0) nonzero")); }
    if !close(math::Cos(0.0), 1.0, 1e-15) { t.Errorf(Sprintf!("Cos(0) != 1")); }
    if !close(math::Sin(math::Pi / 2.0), 1.0, 1e-14) { t.Errorf(Sprintf!("Sin(Pi/2) != 1")); }
    if !close(math::Cos(math::Pi), -1.0, 1e-14) { t.Errorf(Sprintf!("Cos(Pi) != -1")); }
    if !close(math::Atan2(1.0, 0.0), math::Pi / 2.0, 1e-14) {
        t.Errorf(Sprintf!("Atan2(1,0) != Pi/2"));
    }
}}

test!{ fn TestLog(t) {
    if !close(math::Log(math::E), 1.0, 1e-14) { t.Errorf(Sprintf!("Log(E) != 1")); }
    if !close(math::Log2(8.0), 3.0, 1e-14) { t.Errorf(Sprintf!("Log2(8) != 3")); }
    if !close(math::Log10(1000.0), 3.0, 1e-14) { t.Errorf(Sprintf!("Log10(1000) != 3")); }
}}

test!{ fn TestIsNaN(t) {
    if !math::IsNaN(math::NaN) { t.Errorf(Sprintf!("IsNaN(NaN) = false")); }
    if math::IsNaN(0.0) { t.Errorf(Sprintf!("IsNaN(0) = true")); }
    if math::IsNaN(math::Inf) { t.Errorf(Sprintf!("IsNaN(Inf) = true")); }
}}

test!{ fn TestIsInf(t) {
    if !math::IsInf(math::Inf, 1) { t.Errorf(Sprintf!("IsInf(Inf, 1) = false")); }
    if !math::IsInf(-math::Inf, -1) { t.Errorf(Sprintf!("IsInf(-Inf, -1) = false")); }
    if math::IsInf(0.0, 0) { t.Errorf(Sprintf!("IsInf(0, 0) = true")); }
    if math::IsInf(math::NaN, 0) { t.Errorf(Sprintf!("IsInf(NaN, 0) = true")); }
}}

test!{ fn TestFloorCeilRoundTrunc(t) {
    let cases = [
        (1.5, 1.0, 2.0, 2.0, 1.0),
        (-1.5, -2.0, -1.0, -2.0, -1.0),
        (1.2, 1.0, 2.0, 1.0, 1.0),
        (-1.2, -2.0, -1.0, -1.0, -1.0),
    ];
    for (x, f, c, r, tr) in cases {
        if math::Floor(x) != f { t.Errorf(Sprintf!("Floor(%g) = %g, want %g", x, math::Floor(x), f)); }
        if math::Ceil(x) != c  { t.Errorf(Sprintf!("Ceil(%g) = %g, want %g", x, math::Ceil(x), c)); }
        if math::Round(x) != r { t.Errorf(Sprintf!("Round(%g) = %g, want %g", x, math::Round(x), r)); }
        if math::Trunc(x) != tr { t.Errorf(Sprintf!("Trunc(%g) = %g, want %g", x, math::Trunc(x), tr)); }
    }
}}

test!{ fn TestMaxMin(t) {
    if math::Max(3.0, 4.0) != 4.0 { t.Errorf(Sprintf!("Max(3, 4) != 4")); }
    if math::Min(3.0, 4.0) != 3.0 { t.Errorf(Sprintf!("Min(3, 4) != 3")); }
}}
