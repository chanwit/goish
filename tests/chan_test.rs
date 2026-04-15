// port of go/src/runtime/chan_test.go subset
//
// Correctness fixture for the v0.5 channel engine bake-off. The same test
// file must pass with BOTH chan-flume and chan-async features.
//
// Covered subset of Go's TestChan:
//   - buffered/unbuffered send + recv across capacities 0..N
//   - non-blocking recv on empty via TryRecv
//   - non-blocking send on full via TrySend
//   - close + drain + (zero, false) signal
//   - send on closed returns error
//   - MPMC: multiple producers + multiple consumers

#![allow(non_camel_case_types, non_snake_case)]
use goish::prelude::*;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;

test!{ fn TestChan_BufferedSmall(t) {
    // Analogous to Go's TestChan inner block 1: recv from empty blocks.
    for cap in [0i64, 1, 2, 4, 8, 16, 32] {
        let c = chan!(i64, cap as usize);

        // Non-blocking recv does not block.
        let (_, ok) = c.TryRecv();
        if ok {
            t.Errorf(Sprintf!("chan[%d]: TryRecv on empty returned ok", cap));
        }

        // For buffered channels, we can send up to cap.
        if cap > 0 {
            for i in 0..cap {
                let err = c.Send(i);
                if err != nil {
                    t.Errorf(Sprintf!("chan[%d]: Send #%d failed", cap, i));
                }
            }
            // Non-blocking send on full.
            let ok = c.TrySend(99);
            if ok {
                t.Errorf(Sprintf!("chan[%d]: TrySend on full returned ok", cap));
            }
        }
    }
}}

test!{ fn TestChan_CloseDrains(t) {
    // Matches Go's close+drain: after Close, remaining items still recv ok,
    // then (zero, false).
    let c = chan!(i64, 4);
    c.Send(1);
    c.Send(2);
    c.Send(3);
    c.Close();

    for expect in [1i64, 2, 3] {
        let (v, ok) = c.Recv();
        if !ok {
            t.Fatalf(Sprintf!("expected ok recv of %d; got !ok", expect));
        }
        if v != expect {
            t.Errorf(Sprintf!("recv got %d, want %d", v, expect));
        }
    }
    // Drained + closed → (0, false)
    let (v, ok) = c.Recv();
    if ok {
        t.Errorf(Sprintf!("expected !ok after drain; got (%d, true)", v));
    }
    if v != 0 {
        t.Errorf(Sprintf!("expected zero value after close; got %d", v));
    }
}}

test!{ fn TestChan_SendOnClosed(t) {
    let c = chan!(i64, 1);
    c.Close();
    let err = c.Send(42);
    if err == nil {
        t.Error("send on closed channel should return error, got nil");
    }
}}

test!{ fn TestChan_MPMC(t) {
    // Multiple producers, multiple consumers, same channel.
    const PRODUCERS: i32 = 4;
    const CONSUMERS: i32 = 4;
    const PER_PRODUCER: i32 = 250;

    let c = chan!(i32, 16);
    let total_recv = Arc::new(AtomicI32::new(0));
    let sum_recv = Arc::new(AtomicI32::new(0));

    let mut producers = Vec::new();
    for p in 0..PRODUCERS {
        let cp = c.clone();
        producers.push(std::thread::spawn(move || {
            for i in 0..PER_PRODUCER {
                let _ = cp.Send(p * PER_PRODUCER + i);
            }
        }));
    }

    let mut consumers = Vec::new();
    for _ in 0..CONSUMERS {
        let cc = c.clone();
        let total = total_recv.clone();
        let sum = sum_recv.clone();
        consumers.push(std::thread::spawn(move || {
            loop {
                let (v, ok) = cc.Recv();
                if !ok { return; }
                sum.fetch_add(v, Ordering::SeqCst);
                total.fetch_add(1, Ordering::SeqCst);
            }
        }));
    }

    for h in producers { h.join().unwrap(); }
    // All producers done; close so consumers can drain and exit.
    c.Close();
    for h in consumers { h.join().unwrap(); }

    let got = total_recv.load(Ordering::SeqCst);
    let want = PRODUCERS * PER_PRODUCER;
    if got != want {
        t.Errorf(Sprintf!("recv count: got %d, want %d", got, want));
    }

    // Sum 0..PRODUCERS*PER_PRODUCER = N*(N-1)/2 where N = PRODUCERS*PER_PRODUCER
    let n = PRODUCERS * PER_PRODUCER;
    let expected_sum = n * (n - 1) / 2;
    let got_sum = sum_recv.load(Ordering::SeqCst);
    if got_sum != expected_sum {
        t.Errorf(Sprintf!("recv sum: got %d, want %d", got_sum, expected_sum));
    }
}}

test!{ fn TestChan_UnbufferedRendezvous(t) {
    // cap=0 (or cap=1 spirit for async-channel which can't do true cap=0).
    let c = chan!(i32);
    let cp = c.clone();
    let h = std::thread::spawn(move || {
        // Send should synchronize with receiver.
        cp.Send(42);
    });
    let (v, ok) = c.Recv();
    h.join().unwrap();
    if !ok || v != 42 {
        t.Errorf(Sprintf!("unbuffered recv: got (%d, %v); want (42, true)", v, ok));
    }
}}

test!{ fn TestChan_CloseWakesReceiver(t) {
    // Receiver blocked on empty channel; close must wake it with (0, false).
    let c = chan!(i32);
    let cp = c.clone();
    let h = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(30));
        cp.Close();
    });
    let (v, ok) = c.Recv();
    h.join().unwrap();
    if ok {
        t.Errorf(Sprintf!("expected !ok after close-while-blocked; got (%d, true)", v));
    }
    if v != 0 {
        t.Errorf(Sprintf!("expected zero value; got %d", v));
    }
}}

test!{ fn TestChan_EngineReport(t) {
    // Not a real test — just surface the engine name for benchmark logs.
    let name = goish::chan::ENGINE;
    t.Logf(Sprintf!("chan engine: %s", name));
}}
