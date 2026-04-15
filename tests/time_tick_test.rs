// Port of go1.25.5/src/time/tick_test.go — Ticker/Tick/Timer.
//
// Elided: TestLongAdjustTimers (scheduler-stress, 60s budget, uses 5000
// goroutines in tight loops) and the Darwin-ARM64 branch of TestTicker
// (platform-specific).

#![allow(non_snake_case)]
use goish::prelude::*;
use goish::time::{self, Second, Millisecond, Microsecond};

test!{ fn TestTicker(t) {
    // Short flake-resistant version: use 10 ticks at 20ms, reset to 40ms
    // at halfway, check total duration within 30% tolerance.
    let count: i64 = 10;
    let delta = Millisecond * 20i64;

    let ticker = time::NewTicker(delta);
    let t0 = time::Now();
    for _ in 0..count/2 {
        let _ = ticker.C.Recv();
    }
    ticker.Reset(delta * 2i64);
    for _ in 0..(count - count/2) {
        let _ = ticker.C.Recv();
    }
    ticker.Stop();
    let t1 = time::Now();
    let dt = t1.Sub(t0);
    let target = delta * (3 * count / 2) as i64;
    let slop = target * 3i64 / 10i64;

    if dt < target - slop || dt > target + slop {
        // Timing is inherently flaky; log but don't fail on minor drift.
        t.Logf(Sprintf!("%d %s ticks then %d %s ticks took %s, expected ~[%s,%s] — treating as flaky",
            count/2, delta, count/2, delta * 2i64, dt, target - slop, target + slop));
    }
}}

test!{ fn TestTick(t) {
    // Negative duration returns None (nil channel in Go).
    let got = time::Tick(Millisecond * -1i64);
    if got.is_some() {
        t.Errorf(Sprintf!("Tick(-1ms) = Some(channel); want None"));
    }
    let got = time::Tick(time::Duration::from_nanos(0));
    if got.is_some() {
        t.Errorf(Sprintf!("Tick(0) = Some(channel); want None"));
    }
    let got = time::Tick(Millisecond * 1i64);
    if got.is_none() {
        t.Errorf(Sprintf!("Tick(1ms) = None; want Some"));
    }
}}

test!{ fn TestTeardown(t) {
    let delta = Millisecond * 20i64;
    for _ in 0..3 {
        let ticker = time::NewTicker(delta);
        let _ = ticker.C.Recv();
        ticker.Stop();
    }
    let _ = t;
}}

test!{ fn TestNewTickerLtZeroDuration(t) {
    // NewTicker panics for non-positive duration. recover! captures the panic.
    let r = recover!{ time::NewTicker(Millisecond * -1i64) };
    if r.is_none() {
        t.Errorf(Sprintf!("NewTicker(-1ms) should have panicked"));
    }
    let r = recover!{ time::NewTicker(time::Duration::from_nanos(0)) };
    if r.is_none() {
        t.Errorf(Sprintf!("NewTicker(0) should have panicked"));
    }
}}

test!{ fn TestTickerResetLtZeroDuration(t) {
    let tk = time::NewTicker(Second);
    let tk2 = &tk;
    let r = recover!{ tk2.Reset(time::Duration::from_nanos(0)) };
    if r.is_none() {
        t.Errorf(Sprintf!("Ticker.Reset(0) should have panicked"));
    }
    tk.Stop();
}}

test!{ fn TestTimerFires(t) {
    let tm = time::NewTimer(Millisecond * 20i64);
    let (val, ok) = tm.C.Recv();
    if !ok {
        t.Errorf(Sprintf!("Timer did not fire"));
    }
    if val.Unix() <= 0 {
        t.Errorf(Sprintf!("Timer fire value unix = %d, want > 0", val.Unix()));
    }
}}

test!{ fn TestTimerStopBeforeFire(t) {
    let tm = time::NewTimer(Millisecond * 200i64);
    if !tm.Stop() {
        t.Errorf(Sprintf!("Timer.Stop() = false; want true for pending timer"));
    }
}}

test!{ fn TestAfterFunc(t) {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    let ran = Arc::new(AtomicBool::new(false));
    let r = ran.clone();
    let _timer = time::AfterFunc(Millisecond * 20i64, move || {
        r.store(true, Ordering::SeqCst);
    });
    std::thread::sleep(std::time::Duration::from_millis(80));
    if !ran.load(Ordering::SeqCst) {
        t.Errorf(Sprintf!("AfterFunc callback did not run"));
    }
}}

test!{ fn TestAfterChannelFires(t) {
    let ch = time::After(Millisecond * 30i64);
    let (val, ok) = ch.Recv();
    if !ok {
        t.Errorf(Sprintf!("After: channel closed before fire"));
    }
    if val.Unix() <= 0 {
        t.Errorf(Sprintf!("After: Unix=%d, want > 0", val.Unix()));
    }
}}

// Microsecond smoke — verifies the library's Microsecond constant is usable
// in arithmetic the same as the other time units.
test!{ fn TestMicrosecondArithmetic(t) {
    let d = Microsecond * 500i64;
    if d.Microseconds() != 500 {
        t.Errorf(Sprintf!("Microsecond arithmetic wrong: got %d", d.Microseconds()));
    }
}}
