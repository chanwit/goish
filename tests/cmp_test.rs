// Port of go1.25.5 src/cmp/cmp_test.go — Compare, Less, Or with
// integer, string, and NaN float cases.

#![allow(non_snake_case)]
use goish::prelude::*;

test!{ fn TestLessInt(t) {
    if !cmp::Less(&1i64, &2i64) { t.Errorf(Sprintf!("Less(1, 2) = false")); }
    if cmp::Less(&2i64, &1i64) { t.Errorf(Sprintf!("Less(2, 1) = true")); }
    if cmp::Less(&1i64, &1i64) { t.Errorf(Sprintf!("Less(1, 1) = true")); }
}}

test!{ fn TestLessString(t) {
    if !cmp::Less(&"abc".to_string(), &"abd".to_string()) {
        t.Errorf(Sprintf!("Less(abc, abd) = false"));
    }
}}

test!{ fn TestLessNaN(t) {
    // Go spec: NaN < any non-NaN; NaN !< NaN.
    if !cmp::Less(&f64::NAN, &0.0) { t.Errorf(Sprintf!("Less(NaN, 0) = false")); }
    if cmp::Less(&0.0, &f64::NAN) { t.Errorf(Sprintf!("Less(0, NaN) = true")); }
    if cmp::Less(&f64::NAN, &f64::NAN) { t.Errorf(Sprintf!("Less(NaN, NaN) = true")); }
}}

test!{ fn TestCompareInt(t) {
    for (a, b, want) in [(1i64, 2i64, -1i64), (2, 1, 1), (5, 5, 0)] {
        let got = cmp::Compare(&a, &b);
        if got != want {
            t.Errorf(Sprintf!("Compare(%d, %d) = %d, want %d", a, b, got, want));
        }
    }
}}

test!{ fn TestCompareNaN(t) {
    // Go spec: NaN == NaN, NaN < all non-NaN.
    if cmp::Compare(&f64::NAN, &f64::NAN) != 0 {
        t.Errorf(Sprintf!("Compare(NaN, NaN) != 0"));
    }
    if cmp::Compare(&f64::NAN, &1.0) != -1 {
        t.Errorf(Sprintf!("Compare(NaN, 1) != -1"));
    }
    if cmp::Compare(&1.0, &f64::NAN) != 1 {
        t.Errorf(Sprintf!("Compare(1, NaN) != 1"));
    }
}}

test!{ fn TestOr(t) {
    if cmp::Or(&[0i64, 0, 42, 100]) != 42 {
        t.Errorf(Sprintf!("Or first nonzero"));
    }
    if cmp::Or::<i64>(&[0, 0, 0]) != 0 {
        t.Errorf(Sprintf!("Or all zero"));
    }
    let s = cmp::Or::<string>(&[string::default(), "first".into(), "second".into()]);
    if s != "first" {
        t.Errorf(Sprintf!("Or string: %q", s));
    }
}}
