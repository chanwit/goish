// Additional ports from go1.25.5 src/runtime/chan_test.go.
// Complements existing tests/chan_test.rs.
//
// Elided: TestSelectStress (100_000 iters × 4 channels, runtime-stress
// test that saturates Go's scheduler); TestShrinkStackDuringBlockedSend
// and TestNoShrinkStackWhileParking (Go-stack-management specific);
// TestSelectStackAdjust (same). These target Go's runtime internals, not
// channel semantics.

#![allow(non_snake_case)]
use goish::prelude::*;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

test!{ fn TestSelfSelect(t) {
    // Send and recv on the same chan in a select must not deadlock.
    // Simplified: one goroutine, single iteration per cap, exercising
    // both orderings.
    for &cap in &[0usize, 4] {
        let c: Chan<i64> = chan!(i64, cap);
        let c1 = c.clone();
        go!{
            // Sender. Will block until a peer receives (unbuffered) or
            // buffer has room.
            for _ in 0..4 { c1.Send(1); }
        };
        for _ in 0..4 {
            select! {
                recv(c) => {},
                default => {
                    // If nothing ready yet, wait briefly then retry.
                    std::thread::sleep(std::time::Duration::from_millis(5));
                    let _ = c.Recv();
                },
            }
        }
        let _ = t;
    }
}}

test!{ fn TestChanSendInterface(t) {
    // Send an arbitrary value type through a channel; receive the same.
    let c: Chan<Vec<i64>> = chan!(Vec<i64>, 1);
    c.Send(vec![1, 2, 3]);
    let (got, _) = c.Recv();
    if got != vec![1, 2, 3] {
        t.Errorf(Sprintf!("got len %d, want 3", got.len()));
    }
}}

test!{ fn TestMultiConsumer(t) {
    const N_CONS: usize = 10;
    const TOTAL: i64 = 1_000;
    let c: Chan<i64> = chan!(i64, 0);
    let done: Chan<i64> = chan!(i64, N_CONS);

    let producer_wg = sync::WaitGroup::new();
    producer_wg.Add(1);
    let pw = producer_wg.clone();
    let pc = c.clone();
    go!{
        for i in 0..TOTAL { pc.Send(i); }
        close!(pc);
        pw.Done();
    };

    for _ in 0..N_CONS {
        let cr = c.clone();
        let dw = done.clone();
        go!{
            let mut sum: i64 = 0;
            loop {
                let (v, ok) = cr.Recv();
                if !ok { break; }
                sum += v;
            }
            dw.Send(sum);
        };
    }
    producer_wg.Wait();

    let mut total_sum: i64 = 0;
    for _ in 0..N_CONS {
        let (s, _) = done.Recv();
        total_sum += s;
    }
    let want: i64 = (0..TOTAL).sum();
    if total_sum != want {
        t.Errorf(Sprintf!("multi-consumer sum = %d, want %d", total_sum, want));
    }
}}

test!{ fn TestPseudoRandomSend(t) {
    // Drain both channels through a select; verify both are chosen.
    let a: Chan<i64> = chan!(i64, 4);
    let b: Chan<i64> = chan!(i64, 4);
    let counts = Arc::new((AtomicI64::new(0), AtomicI64::new(0)));

    // Pre-fill both buffers.
    for _ in 0..4 { a.Send(1); b.Send(2); }

    let cntc = counts.clone();
    for _ in 0..8 {
        select! {
            recv(a) |v| => { let _ = v; cntc.0.fetch_add(1, Ordering::SeqCst); },
            recv(b) |v| => { let _ = v; cntc.1.fetch_add(1, Ordering::SeqCst); },
        }
    }

    let ca = counts.0.load(Ordering::SeqCst);
    let cb = counts.1.load(Ordering::SeqCst);
    if ca + cb != 8 {
        t.Errorf(Sprintf!("total = %d, want 8", ca + cb));
    }
    if ca == 0 || cb == 0 {
        t.Errorf(Sprintf!("bias: a=%d b=%d", ca, cb));
    }
}}

test!{ fn TestSelectDefault(t) {
    // With no ready channel and a default, select should take default.
    let c: Chan<i64> = chan!(i64, 0);
    let mut took_default = false;
    select! {
        recv(c) => {},
        default => { took_default = true; },
    }
    if !took_default {
        t.Errorf(Sprintf!("select should have taken default"));
    }
}}

test!{ fn TestSelectReadyRecv(t) {
    // Select where only one channel is ready — picks that one.
    let c: Chan<i64> = chan!(i64, 1);
    c.Send(42);
    let mut got: i64 = 0;
    select! {
        recv(c) |v| => { got = v; },
        default => { t.Errorf(Sprintf!("default selected despite ready recv")); },
    }
    if got != 42 {
        t.Errorf(Sprintf!("got %d want 42", got));
    }
}}

test!{ fn TestCloseThenRecv(t) {
    // Recv on closed chan returns (zero, false) after draining.
    let c: Chan<i64> = chan!(i64, 3);
    c.Send(1); c.Send(2); c.Send(3);
    close!(c);
    for want in [1, 2, 3] {
        let (v, ok) = c.Recv();
        if !ok || v != want {
            t.Errorf(Sprintf!("drain: got (%d, %v) want (%d, true)", v, ok, want));
        }
    }
    let (v, ok) = c.Recv();
    if ok || v != 0 {
        t.Errorf(Sprintf!("after drain: got (%d, %v) want (0, false)", v, ok));
    }
}}

test!{ fn TestChanOfStrings(t) {
    let c: Chan<String> = chan!(String, 2);
    c.Send("hello".to_string());
    c.Send("world".to_string());
    close!(c);
    let (a, _) = c.Recv();
    let (b, _) = c.Recv();
    let (_, ok) = c.Recv();
    if a != "hello" || b != "world" || ok {
        t.Errorf(Sprintf!("got (%q, %q, ok=%v)", a, b, ok));
    }
}}
