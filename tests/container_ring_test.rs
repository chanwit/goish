// Port of go1.25.5 src/container/ring/ring_test.go.
//
// Mapping Go's `var r Ring` (zero-value 1-element ring) to
// `Ring::new_single()` in goish; `var r0 *Ring = nil` to `Ring::nil()`.
// `r.Value = x` → `r.SetValue(x)`; `r != s` → `!r.ptr_eq(&s)`.

#![allow(non_snake_case)]
use goish::prelude::*;
use goish::container::ring::{self, Ring};

fn verify(t: &goish::testing::T, r: &Ring<i64>, N: i64, want_sum: i64) {
    let n = r.Len();
    if n != N {
        t.Errorf(Sprintf!("r.Len() == %d; expected %d", n, N));
    }
    // Iteration via Do.
    let mut count = 0i64;
    let mut sum = 0i64;
    r.Do(|v| {
        count += 1;
        if let Some(x) = v { sum += *x; }
    });
    if count != N {
        t.Errorf(Sprintf!("forward iterations == %d; expected %d", count, N));
    }
    if want_sum >= 0 && sum != want_sum {
        t.Errorf(Sprintf!("forward ring sum = %d; expected %d", sum, want_sum));
    }

    if N == 0 { return; }

    // Next / Prev consistency (walk forward N steps returns to r).
    let mut p = r.Next();
    for _ in 1..N { p = p.Next(); }
    if !p.ptr_eq(r) {
        t.Errorf(Sprintf!("walking forward %d steps did not return to r", N));
    }

    // Move(0), Move(N), Move(-N) all equal r.
    if !r.Move(0).ptr_eq(r) { t.Errorf(Sprintf!("r.Move(0) != r")); }
    if !r.Move(N).ptr_eq(r) { t.Errorf(Sprintf!("r.Move(%d) != r", N)); }
    if !r.Move(-N).ptr_eq(r) { t.Errorf(Sprintf!("r.Move(%d) != r", -N)); }

    // Move(N+i) == Move(i) for i in 0..10.
    for i in 0..10i64 {
        let ni = N + i;
        let mi = ni % N;
        if !r.Move(ni).ptr_eq(&r.Move(mi)) {
            t.Errorf(Sprintf!("r.Move(%d) != r.Move(%d)", ni, mi));
        }
    }
}

fn makeN(n: i64) -> Ring<i64> {
    let r = ring::New::<i64>(n);
    if n == 0 { return r; }
    r.SetValue(1);
    let mut cur = r.Next();
    for i in 2..=n {
        cur.SetValue(i);
        cur = cur.Next();
    }
    r
}

fn sumN(n: i64) -> i64 { (n * n + n) / 2 }

test!{ fn TestCornerCases(t) {
    let r0 = Ring::<i64>::nil();
    let r1 = Ring::<i64>::new_single();
    verify(t, &r0, 0, 0);
    verify(t, &r1, 1, 0);
    // Link nil is a no-op.
    r1.Link(&r0);
    verify(t, &r0, 0, 0);
    verify(t, &r1, 1, 0);
    r1.Link(&r0);
    verify(t, &r1, 1, 0);
    // Unlink(0) is a no-op.
    r1.Unlink(0);
    verify(t, &r1, 1, 0);
}}

test!{ fn TestNew(t) {
    for i in 0..10i64 {
        let r = ring::New::<i64>(i);
        verify(t, &r, i, -1);
    }
    for i in 0..10i64 {
        let r = makeN(i);
        verify(t, &r, i, sumN(i));
    }
}}

test!{ fn TestLink1(t) {
    let r1a = makeN(1);
    let r1b = Ring::<i64>::new_single();
    let r2a = r1a.Link(&r1b);
    verify(t, &r2a, 2, 1);
    if !r2a.ptr_eq(&r1a) {
        t.Errorf(Sprintf!("a) 2-element link failed"));
    }
}}

test!{ fn TestLink2(t) {
    // Three separate rings; link them together, verify sum + count.
    let r1a = Ring::<i64>::new_single();
    r1a.SetValue(42);
    let r1b = Ring::<i64>::new_single();
    r1b.SetValue(77);
    let r10 = makeN(10);

    r1a.Link(&Ring::<i64>::nil());
    verify(t, &r1a, 1, 42);

    r1a.Link(&r1b);
    verify(t, &r1a, 2, 42 + 77);

    r10.Link(&Ring::<i64>::nil());
    verify(t, &r10, 10, sumN(10));

    r10.Link(&r1a);
    verify(t, &r10, 12, sumN(10) + 42 + 77);
}}

test!{ fn TestUnlink(t) {
    let r10 = makeN(10);
    let s10 = r10.Move(6);
    let sum10 = sumN(10);

    verify(t, &r10, 10, sum10);
    verify(t, &s10, 10, sum10);

    let r0 = r10.Unlink(0);
    verify(t, &r0, 0, 0);

    let r1 = r10.Unlink(1);
    verify(t, &r1, 1, 2);
    verify(t, &r10, 9, sum10 - 2);
}}

test!{ fn TestMoveEmptyRing(t) {
    // `var r Ring` in Go is a 1-element ring (zero-value struct with self-
    // refs after first method call). goish's equivalent is `new_single()`.
    let r = Ring::<i64>::new_single();
    let r2 = r.Move(1);
    // In a 1-element ring, Move(1) returns r itself.
    if !r2.ptr_eq(&r) {
        t.Errorf(Sprintf!("Move(1) on 1-element ring should return self"));
    }
    verify(t, &r, 1, 0);
}}
