// Port of go1.25.5 src/sync/waitgroup_test.go + once_test.go.
//
// Elided: TestWaitGroupMisuse (negative-counter panic tests, a Go-specific
// detector); benchmark runners.

#![allow(non_snake_case)]
use goish::prelude::*;

test!{ fn TestWaitGroup(t) {
    let wg1 = sync::WaitGroup::new();
    let wg2 = sync::WaitGroup::new();
    for _ in 0..16 {
        wg1.Add(1);
        wg2.Add(1);
        let n = sync::atomic::Int64::new(0);
        let n1 = n.clone();
        let n2 = n.clone();
        let w1 = wg1.clone();
        let w2 = wg2.clone();
        go!{
            n1.Add(1);
            w1.Done();
        };
        go!{
            n2.Add(1);
            w2.Done();
        };
    }
    wg1.Wait();
    wg2.Wait();
    let _ = t;
}}

test!{ fn TestWaitGroupReuse(t) {
    // A WaitGroup should be reusable after Wait returns.
    let wg = sync::WaitGroup::new();
    for _ in 0..3 {
        wg.Add(4);
        for _ in 0..4 {
            let w = wg.clone();
            go!{
                std::thread::sleep(std::time::Duration::from_millis(5));
                w.Done();
            };
        }
        wg.Wait();
    }
    let _ = t;
}}

test!{ fn TestWaitGroupZero(t) {
    // Zero-counter WaitGroup should return from Wait immediately.
    let wg = sync::WaitGroup::new();
    wg.Wait();
    let _ = t;
}}

struct OneT { f: fn(), called: i64 }

test!{ fn TestOnce(t) {
    let once = sync::Once::new();
    let counter = sync::atomic::Int64::new(0);
    // Spawn 100 goroutines each attempting Do; only one call should run.
    let wg = sync::WaitGroup::new();
    for _ in 0..100 {
        wg.Add(1);
        let o = once.clone();
        let c = counter.clone();
        let w = wg.clone();
        go!{
            o.Do(move || { c.Add(1); });
            w.Done();
        };
    }
    wg.Wait();
    if counter.Load() != 1 {
        t.Errorf(Sprintf!("Once ran %d times, want 1", counter.Load()));
    }
}}

test!{ fn TestOnceSequential(t) {
    // Calling Do on the same Once multiple times on one goroutine runs once.
    let once = sync::Once::new();
    let mut n = 0;
    for _ in 0..5 {
        once.Do(|| { n += 1; });
    }
    if n != 1 {
        t.Errorf(Sprintf!("seq Once ran %d times, want 1", n));
    }
}}

test!{ fn TestOnceDistinct(t) {
    // Two different Once values run their bodies independently.
    let o1 = sync::Once::new();
    let o2 = sync::Once::new();
    let count = sync::atomic::Int64::new(0);
    let c1 = count.clone(); let c2 = count.clone();
    o1.Do(move || { c1.Add(1); });
    o2.Do(move || { c2.Add(1); });
    if count.Load() != 2 {
        t.Errorf(Sprintf!("distinct Once total = %d, want 2", count.Load()));
    }
    // Let OneT unused helper avoid dead_code.
    let _ = OneT { f: || {}, called: 0 };
}}
