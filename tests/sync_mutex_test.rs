// Port of go1.25.5 src/sync/mutex_test.go (+ rwmutex basics).
//
// goish's `sync::Mutex<T>` is RAII — `Lock()` returns a guard that
// auto-unlocks on drop, which is semantically identical to Go's
// `mu.Lock(); defer mu.Unlock()`. We port Go's tests by scoping the
// guard; the Go `m.Unlock()` call site corresponds to the guard
// going out of scope.
//
// Elided: tests that drive into Go's runtime semaphore (TestSemaphore,
// benchmarks) and misuse-panic tests that rely on Go's mutex detector
// for double-unlock / fatal-error messages.

#![allow(non_snake_case)]
use goish::prelude::*;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

fn hammer_mutex(m: sync::Mutex<()>, loops: i64) {
    for i in 0..loops {
        if i % 3 == 0 {
            if let Some(g) = m.TryLock() { drop(g); }
            continue;
        }
        let _g = m.Lock();
    }
}

test!{ fn TestMutex(t) {
    let m = sync::Mutex::new(());
    // Lock should succeed.
    let g = m.Lock();
    // TryLock should fail while held.
    if m.TryLock().is_some() {
        t.Fatal(Sprintf!("TryLock succeeded with mutex locked"));
    }
    drop(g);
    // After release, TryLock should succeed.
    let g2 = m.TryLock();
    if g2.is_none() {
        t.Fatal(Sprintf!("TryLock failed with mutex unlocked"));
    }
    drop(g2);

    // Stress test — spawn 10 threads doing 1000 iters each.
    let wg = sync::WaitGroup::new();
    for _ in 0..10 {
        wg.Add(1);
        let mu = m.clone();
        let w = wg.clone();
        std::thread::spawn(move || {
            hammer_mutex(mu, 1000);
            w.Done();
        });
    }
    wg.Wait();
    let _ = t;
}}

test!{ fn TestMutexConcurrentCounter(t) {
    // 10 goroutines incrementing a shared counter under a Mutex must
    // yield exactly 10_000.
    let m = sync::Mutex::new(0i64);
    let wg = sync::WaitGroup::new();
    for _ in 0..10 {
        wg.Add(1);
        let mu = m.clone();
        let w = wg.clone();
        std::thread::spawn(move || {
            for _ in 0..1000 {
                let mut g = mu.Lock();
                *g += 1;
            }
            w.Done();
        });
    }
    wg.Wait();
    let final_ = *m.Lock();
    if final_ != 10_000 {
        t.Errorf(Sprintf!("counter = %d, want 10000", final_));
    }
}}

test!{ fn TestMutexTryLock(t) {
    let m = sync::Mutex::new(());
    if m.TryLock().is_none() {
        t.Fatal(Sprintf!("TryLock on new mutex failed"));
    }
    let g = m.Lock();
    if m.TryLock().is_some() {
        t.Errorf(Sprintf!("TryLock on held mutex succeeded"));
    }
    drop(g);
}}

test!{ fn TestRWMutexReaders(t) {
    let rw = sync::RWMutex::new(0i64);
    let wg = sync::WaitGroup::new();
    let reads = Arc::new(AtomicI64::new(0));
    for _ in 0..20 {
        wg.Add(1);
        let w = wg.clone();
        let r = rw.clone();
        let c = reads.clone();
        std::thread::spawn(move || {
            let g = r.RLock();
            let _ = *g;
            c.fetch_add(1, Ordering::SeqCst);
            w.Done();
        });
    }
    wg.Wait();
    if reads.load(Ordering::SeqCst) != 20 {
        t.Errorf(Sprintf!("reads = %d", reads.load(Ordering::SeqCst)));
    }
}}

test!{ fn TestRWMutexWriter(t) {
    let rw = sync::RWMutex::new(0i64);
    {
        let mut g = rw.Lock();
        *g = 42;
    }
    let r = rw.RLock();
    if *r != 42 {
        t.Errorf(Sprintf!("writer then reader = %d, want 42", *r));
    }
}}
