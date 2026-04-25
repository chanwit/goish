// Port of go1.25.5 src/context/context_test.go — core semantics.
//
// Elided: the enormous propagation table (TestChildFinishesFirst,
// TestCancelRemoves, TestParentFinishesChild) that simulates hundreds of
// context trees and measures runtime stats; Cause/WithCancelCause (not
// ported yet). We cover the contract every context user actually relies
// on: Background, WithCancel, WithTimeout, WithValue, Done/Err.

#![allow(non_snake_case)]
use goish::prelude::*;
use std::time::Duration;

test!{ fn TestBackground(t) {
    let ctx = context::Background();
    if ctx.Err() != nil {
        t.Errorf(Sprintf!("Background.Err() = %s, want nil", ctx.Err()));
    }
}}

test!{ fn TestWithCancel(t) {
    let (ctx, cancel) = context::WithCancel(context::Background());
    if ctx.Err() != nil {
        t.Errorf(Sprintf!("fresh WithCancel.Err() = %s", ctx.Err()));
    }
    cancel.call();
    // Wait briefly for the propagation.
    for _ in 0..100 {
        if ctx.Err() != nil { break; }
        std::thread::sleep(Duration::from_millis(1));
    }
    let es = Sprintf!("%v", ctx.Err());
    if !strings::Contains(&es, "canceled") && !strings::Contains(&es, "Canceled") {
        t.Errorf(Sprintf!("WithCancel.Err() = %s, want 'canceled'", es));
    }
}}

test!{ fn TestWithCancelDone(t) {
    let (ctx, cancel) = context::WithCancel(context::Background());
    let done = ctx.Done();
    let flag = sync::atomic::Bool::new(false);
    let f = flag.clone();
    go!{
        let _ = done.Recv();
        f.Store(true);
    };
    std::thread::sleep(Duration::from_millis(10));
    cancel.call();
    std::thread::sleep(Duration::from_millis(50));
    if !flag.Load() {
        t.Errorf(Sprintf!("Done channel did not fire after cancel"));
    }
}}

test!{ fn TestWithTimeout(t) {
    let start = time::Now();
    let (ctx, _cancel) = context::WithTimeout(context::Background(), time::Millisecond * 50i64);
    ctx.Wait();
    let elapsed = time::Since(start);
    if elapsed.Milliseconds() < 40 {
        t.Errorf(Sprintf!("WithTimeout fired too early: %d ms", elapsed.Milliseconds()));
    }
    if elapsed.Milliseconds() > 1000 {
        t.Errorf(Sprintf!("WithTimeout fired too late: %d ms", elapsed.Milliseconds()));
    }
    if ctx.Err() == nil {
        t.Errorf(Sprintf!("WithTimeout Err() = nil after deadline"));
    }
}}

test!{ fn TestWithDeadline(t) {
    let deadline = time::Now().Add(time::Millisecond * 40i64);
    let (ctx, _cancel) = context::WithDeadline(context::Background(), deadline);
    ctx.Wait();
    if ctx.Err() == nil {
        t.Errorf(Sprintf!("WithDeadline Err() = nil after deadline"));
    }
}}

test!{ fn TestWithValue(t) {
    let ctx = context::Background();
    let ctx = context::WithValue(ctx, "user", string::from("alice"));
    let got: Option<string> = ctx.Value("user");
    if got.as_deref() != Some("alice") {
        t.Errorf(Sprintf!("Value(user) = %v", got.is_some()));
    }
    // Missing key returns None.
    let missing: Option<string> = ctx.Value("missing");
    if missing.is_some() {
        t.Errorf(Sprintf!("missing key got Some"));
    }
}}

test!{ fn TestWithValueChained(t) {
    let ctx = context::Background();
    let ctx = context::WithValue(ctx, "a", 1i64);
    let ctx = context::WithValue(ctx, "b", 2i64);
    let a: Option<i64> = ctx.Value("a");
    let b: Option<i64> = ctx.Value("b");
    if a != Some(1) || b != Some(2) {
        t.Errorf(Sprintf!("Chained values: a=%v b=%v", a.is_some(), b.is_some()));
    }
}}

test!{ fn TestCancelPropagatesToChild(t) {
    let (parent, pcancel) = context::WithCancel(context::Background());
    let (child, _ccancel) = context::WithCancel(parent);
    pcancel.call();
    // Wait briefly for propagation.
    for _ in 0..100 {
        if child.Err() != nil { break; }
        std::thread::sleep(Duration::from_millis(1));
    }
    if child.Err() == nil {
        t.Errorf(Sprintf!("child did not propagate parent cancellation"));
    }
}}

test!{ fn TestMultipleCancelCalls(t) {
    // Calling cancel twice should not panic.
    let (ctx, cancel) = context::WithCancel(context::Background());
    cancel.call();
    cancel.call();
    let _ = ctx;
}}

test!{ fn TestBackgroundNeverDone(t) {
    // Background.Err() stays nil; its Done channel doesn't fire.
    let ctx = context::Background();
    if ctx.Err() != nil { t.Errorf(Sprintf!("Background Err non-nil")); }
    // Wait a bit to make sure nothing fires.
    std::thread::sleep(Duration::from_millis(20));
    if ctx.Err() != nil { t.Errorf(Sprintf!("Background Err changed")); }
}}
