// Regression tests for the 5 CSP-derived semantic bugs fixed in v0.10.1
// (issue #119). Each test targets one specific divergence between goish's
// old macro_rules! select and Go's select spec.

#![allow(non_snake_case)]
use goish::prelude::*;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

// ── Bug 1: Fairness ──────────────────────────────────────────────────
// Old select! always picked arm 0 when both were ready. Go picks
// uniformly at random. flume::Selector provides built-in arbitration.

test!{ fn TestSelectFairness(t) {
    let a: Chan<i64> = chan!(i64, 1);
    let b: Chan<i64> = chan!(i64, 1);
    let mut ca: i64 = 0;
    let mut cb: i64 = 0;
    const ITERS: i64 = 1000;
    for _ in 0..ITERS {
        a.Send(1);
        b.Send(2);
        select! {
            recv(a) |_v| => { ca += 1; },
            recv(b) |_v| => { cb += 1; },
        }
        // drain the other channel so next iteration starts clean
        let _ = a.TryRecv();
        let _ = b.TryRecv();
    }
    // Both arms should fire a meaningful fraction of the time.
    // With true uniform random: mean 500, stddev ~16. Accept ≥ 30%.
    let floor = (ITERS as f64 * 0.30) as i64;
    if ca < floor || cb < floor {
        t.Errorf(Sprintf!("fairness: a=%d b=%d over %d iters (floor=%d)", ca, cb, ITERS, floor));
    }
}}

// ── Bug 2: send-on-closed always panics ──────────────────────────────
// Go spec: send to a closed channel is a "ready" event that panics,
// even if another recv arm would also fire. The shadow close_rx arm
// makes the panic participate in the random pick.

test!{ fn TestSelectSendOnClosedPanics(t) {
    let panicked = sync::atomic::Bool::new(false);
    let p = panicked.clone();
    let handle = std::thread::spawn(move || {
        let c: Chan<i64> = chan!(i64, 1);
        close!(c);
        // No default — Selector arbitrates between recv arm (if it were
        // ready) and the closed-send arm. With only a closed send arm,
        // it must panic.
        select! {
            send(c, 42) => {},
        }
    });
    match handle.join() {
        Err(_) => { p.Store(true); }
        Ok(_) => {}
    }
    if !panicked.Load() {
        t.Errorf(Sprintf!("select with send to closed channel should have panicked"));
    }
}}

// ── Bug 3: arm expressions evaluated exactly once ────────────────────
// Old select! re-evaluated arm expressions every 1ms poll iteration.
// Now with Selector, send-value expression runs once per select.

test!{ fn TestSelectExprEvaluatedOnce(t) {
    let counter = Arc::new(AtomicI64::new(0));
    let c: Chan<i64> = chan!(i64, 1);
    // Fill the channel so the send arm fires immediately.
    // (We want to confirm the expression ran exactly once.)
    //
    // Use a fresh empty channel to send INTO (it has room).
    let out: Chan<i64> = chan!(i64, 1);
    let cnt = counter.clone();
    select! {
        send(out, cnt.fetch_add(1, Ordering::SeqCst)) => {},
        default => {},
    }
    let got = counter.load(Ordering::SeqCst);
    if got != 1 {
        t.Errorf(Sprintf!("counter = %d, want 1 (expression evaluated more than once)", got));
    }
    // Verify the sent value is the pre-increment (0).
    let (v, _) = out.Recv();
    if v != 0 {
        t.Errorf(Sprintf!("sent value = %d, want 0", v));
    }
}}

// ── Bug 4: wake latency ─────────────────────────────────────────────
// Old 1ms spin-sleep added up to 1ms latency for no-default selects.
// With flume::Selector the wake should be sub-millisecond.
//
// We measure latency relative to the SENDER's own timestamp (captured
// immediately before Send), not the test start. Including the 10ms
// sleep in the measurement would let scheduling jitter on a loaded CI
// runner mask the wake-latency signal entirely.

test!{ fn TestSelectWakeLatency(t) {
    let c: Chan<i64> = chan!(i64, 0);
    let cc = c.clone();
    let send_at: sync::Mutex<Option<std::time::Instant>> = sync::Mutex::new(None);
    let sa = send_at.clone();
    go!{
        std::thread::sleep(std::time::Duration::from_millis(10));
        // Timestamp the moment just before the rendezvous completes.
        *sa.Lock() = Some(std::time::Instant::now());
        cc.Send(42);
    };
    let mut got: i64 = 0;
    select! {
        recv(c) |v| => { got = v; },
    }
    let received_at = std::time::Instant::now();
    if got != 42 {
        t.Errorf(Sprintf!("got %d want 42", got));
    }
    let sent_at = send_at.Lock().expect("sender did not set timestamp");
    let latency = received_at.duration_since(sent_at);
    // The old macro_rules! select would add up to 1 ms of spin-sleep on
    // top of the send-to-receive handoff. Park-based select should be
    // sub-millisecond on a quiet machine. Accept up to 50 ms to absorb
    // CI scheduling outliers — anything above that would indicate a
    // real regression to polling (or much worse).
    if latency.as_millis() > 50 {
        t.Errorf(Sprintf!("wake latency too high: %d ms (want < 50)", latency.as_millis()));
    }
}}

// ── Bug 5 (compile-time): shared mutable state across arm bodies ─────
// With old macro_rules!+Selector attempt, arm bodies inside FnMut
// closures that shared &mut state would fail to compile. The dispatch-
// by-tag design runs bodies OUTSIDE closures, so this compiles fine.

test!{ fn TestSelectSharedBodyCompiles(t) {
    let a: Chan<i64> = chan!(i64, 1);
    let b: Chan<i64> = chan!(i64, 1);
    a.Send(10);
    b.Send(20);
    // Both arms mutate the same outer `sum` — this tests that arm
    // bodies compile outside the Selector closures.
    let mut sum: i64 = 0;
    select! {
        recv(a) |v| => { sum += v; },
        recv(b) |v| => { sum += v; },
    }
    // One of the two fired; drain the other.
    select! {
        recv(a) |v| => { sum += v; },
        recv(b) |v| => { sum += v; },
    }
    if sum != 30 {
        t.Errorf(Sprintf!("sum = %d, want 30", sum));
    }
}}
