// Port of go1.25.5 src/sync/atomic/atomic_test.go — subset.
//
// Elided: the MASSIVE generated table of SwapInt32/Add* benchmarks that
// hammer every integer width at 10_000+ iterations; pointer-atomic tests
// (unsafe.Pointer); alignment tests (Go-specific ABI). We cover the
// semantic API surface.

#![allow(non_snake_case)]
use goish::prelude::*;
use goish::sync::atomic;
use std::sync::Arc;

test!{ fn TestSwapInt32(t) {
    let x = atomic::Int32::new(0);
    for i in 1i32..100 {
        let old = x.Swap(i);
        if old != i - 1 {
            t.Fatalf(Sprintf!("SwapInt32: old = %d, want %d", old, i - 1));
        }
    }
}}

test!{ fn TestSwapInt64(t) {
    let x = atomic::Int64::new(0);
    for i in 1i64..100 {
        let old = x.Swap(i);
        if old != i - 1 {
            t.Fatalf(Sprintf!("SwapInt64: old = %d, want %d", old, i - 1));
        }
    }
}}

test!{ fn TestSwapUint32(t) {
    let x = atomic::Uint32::new(0);
    for i in 1u32..100 {
        let old = x.Swap(i);
        if old != i - 1 {
            t.Fatalf(Sprintf!("SwapUint32: old = %u, want %u", old, i - 1));
        }
    }
}}

test!{ fn TestAddInt32(t) {
    let x = atomic::Int32::new(0);
    for i in 1i32..100 {
        let v = x.Add(1);
        if v != i {
            t.Fatalf(Sprintf!("AddInt32: v = %d, want %d", v, i));
        }
    }
    if x.Load() != 99 {
        t.Errorf(Sprintf!("Load() = %d, want 99", x.Load()));
    }
}}

test!{ fn TestAddInt64(t) {
    let x = atomic::Int64::new(0);
    let sum: i64 = (1..=10).sum();
    for i in 1i64..=10 {
        x.Add(i);
    }
    if x.Load() != sum {
        t.Errorf(Sprintf!("AddInt64: Load() = %d, want %d", x.Load(), sum));
    }
}}

test!{ fn TestCompareAndSwapInt32(t) {
    let x = atomic::Int32::new(1);
    if !x.CompareAndSwap(1, 2) { t.Errorf(Sprintf!("CAS 1→2 failed")); }
    if x.Load() != 2 { t.Errorf(Sprintf!("Load after CAS = %d", x.Load())); }
    if x.CompareAndSwap(1, 3) {
        t.Errorf(Sprintf!("CAS 1→3 succeeded when value is 2"));
    }
    if x.Load() != 2 { t.Errorf(Sprintf!("Load after failed CAS = %d", x.Load())); }
}}

test!{ fn TestCompareAndSwapInt64(t) {
    let x = atomic::Int64::new(-1);
    if !x.CompareAndSwap(-1, 42) { t.Errorf(Sprintf!("CAS -1→42 failed")); }
    if x.Load() != 42 { t.Errorf(Sprintf!("Load = %d", x.Load())); }
}}

test!{ fn TestLoadInt64(t) {
    let x = atomic::Int64::new(0);
    for i in 0i64..100 {
        x.Store(i);
        if x.Load() != i { t.Errorf(Sprintf!("Load %d", i)); }
    }
}}

test!{ fn TestStoreInt32(t) {
    let x = atomic::Int32::new(0);
    for i in 0i32..100 {
        x.Store(i);
        if x.Load() != i { t.Errorf(Sprintf!("Store Load %d", i)); }
    }
}}

test!{ fn TestBoolAtomic(t) {
    let b = atomic::Bool::new(false);
    if b.Load() { t.Errorf(Sprintf!("initial Load = true")); }
    b.Store(true);
    if !b.Load() { t.Errorf(Sprintf!("Load after Store(true) = false")); }
    if b.Swap(false) != true {
        t.Errorf(Sprintf!("Swap old != true"));
    }
    if b.Load() { t.Errorf(Sprintf!("Load after Swap = true")); }
    if !b.CompareAndSwap(false, true) {
        t.Errorf(Sprintf!("CAS false→true failed"));
    }
}}

test!{ fn TestConcurrentIncrement(t) {
    // 100 goroutines each adding 100 to a shared Int64 → final 10_000.
    let x = atomic::Int64::new(0);
    let wg = sync::WaitGroup::new();
    for _ in 0..100 {
        wg.Add(1);
        let n = x.clone();
        let w = wg.clone();
        std::thread::spawn(move || {
            for _ in 0..100 { n.Add(1); }
            w.Done();
        });
    }
    wg.Wait();
    if x.Load() != 10_000 {
        t.Errorf(Sprintf!("Load = %d, want 10000", x.Load()));
    }
}}

test!{ fn TestConcurrentCAS(t) {
    // 10 goroutines racing to CAS 0→1. Only one should succeed.
    let x = atomic::Int32::new(0);
    let wins = Arc::new(std::sync::atomic::AtomicI64::new(0));
    let wg = sync::WaitGroup::new();
    for _ in 0..10 {
        wg.Add(1);
        let n = x.clone();
        let w = wg.clone();
        let c = wins.clone();
        std::thread::spawn(move || {
            if n.CompareAndSwap(0, 1) {
                c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
            w.Done();
        });
    }
    wg.Wait();
    if wins.load(std::sync::atomic::Ordering::SeqCst) != 1 {
        t.Errorf(Sprintf!("CAS winners = %d, want 1",
            wins.load(std::sync::atomic::Ordering::SeqCst)));
    }
}}
