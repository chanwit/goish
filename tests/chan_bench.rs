// Channel engine bake-off benchmarks.
//
// Run with:
//   cargo test --release --no-default-features --features chan-flume \
//              --test chan_bench -- --nocapture --test-threads=1 Benchmark
//   cargo test --release --no-default-features --features chan-async \
//              --test chan_bench -- --nocapture --test-threads=1 Benchmark
//
// Both runs print the engine name + per-benchmark ns/op. Compare side by side.

#![allow(non_snake_case)]
use goish::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

// ── B0: engine identifier ─────────────────────────────────────────────

test!{ fn Benchmark_Z_EngineName(t) {
    t.Logf(Sprintf!("chan engine = %s", goish::chan::ENGINE));
    eprintln!(">>> chan engine = {} <<<", goish::chan::ENGINE);
}}

// ── B1: Ping-pong latency ─────────────────────────────────────────────
// Two channels, one worker thread bouncing values. Measures round-trip.

benchmark!{ fn BenchmarkPingPong(b) {
    let req = chan!(i64, 1);
    let rsp = chan!(i64, 1);
    let rc = req.clone();
    let sc = rsp.clone();
    let worker = thread::spawn(move || {
        loop {
            let (v, ok) = rc.Recv();
            if !ok { break; }
            sc.Send(v);
        }
    });
    while b.Loop() {
        req.Send(42);
        let _ = rsp.Recv();
    }
    req.Close();
    worker.join().unwrap();
}}

// ── B2: Producer/consumer throughput (buffered cap=100, 1P:1C) ────────

benchmark!{ fn BenchmarkChanProdCons_cap100(b) {
    let c = chan!(i64, 100);
    let cp = c.clone();
    let n = b.N;
    let producer = thread::spawn(move || {
        for i in 0..n { let _ = cp.Send(i); }
    });
    let mut total = 0i64;
    for _ in 0..n {
        let (v, _) = c.Recv();
        total += v;
    }
    producer.join().unwrap();
    // Drain loop counter so report reflects iterations.
    while b.Loop() {}
    std::hint::black_box(total);
}}

// ── B3: Unbuffered rendezvous throughput ──────────────────────────────

benchmark!{ fn BenchmarkChanProdCons_cap0(b) {
    let c = chan!(i64, 0);
    let cp = c.clone();
    let n = b.N;
    let producer = thread::spawn(move || {
        for i in 0..n { let _ = cp.Send(i); }
    });
    let mut total = 0i64;
    for _ in 0..n {
        let (v, _) = c.Recv();
        total += v;
    }
    producer.join().unwrap();
    while b.Loop() {}
    std::hint::black_box(total);
}}

// ── B4: Multi-producer (4P:1C, cap=100) ──────────────────────────────

benchmark!{ fn BenchmarkChanProdCons_4P1C(b) {
    let c = chan!(i64, 100);
    let per_producer = b.N / 4;
    let mut producers = Vec::new();
    for _ in 0..4 {
        let cp = c.clone();
        producers.push(thread::spawn(move || {
            for i in 0..per_producer { let _ = cp.Send(i); }
        }));
    }
    let total_iters = per_producer * 4;
    let mut sum = 0i64;
    for _ in 0..total_iters {
        let (v, _) = c.Recv();
        sum += v;
    }
    for h in producers { h.join().unwrap(); }
    while b.Loop() {}
    std::hint::black_box(sum);
}}

// ── B5a: std::thread::spawn + send ────────────────────────────────────
// Baseline: plain OS thread spawn per iteration.

benchmark!{ fn BenchmarkSpawnThreadAndSend(b) {
    while b.Loop() {
        let c = chan!(i64, 1);
        let cp = c.clone();
        let h = thread::spawn(move || { cp.Send(1); });
        let _ = c.Recv();
        h.join().unwrap();
    }
}}

// ── B5b: go!{} + send ────────────────────────────────────────────────
// goish goroutines via tokio's spawn_blocking pool. Should be materially
// cheaper than plain thread::spawn thanks to thread reuse.

benchmark!{ fn BenchmarkGoroutineSpawnAndSend(b) {
    while b.Loop() {
        let c = chan!(i64, 1);
        let cp = c.clone();
        let h = goish::go!{ cp.Send(1); };
        let _ = c.Recv();
        let _ = h.Wait();
    }
}}

// ── B5c: go!{} spawn only (no channel) ───────────────────────────────
// Pure spawn cost; subtracted from B5b gives channel round-trip cost.

benchmark!{ fn BenchmarkGoroutineSpawnOnly(b) {
    while b.Loop() {
        let h = goish::go!{ let _ = std::hint::black_box(1 + 1); };
        let _ = h.Wait();
    }
}}

// ── B6: Memory footprint — 100k channels parked with 1 value each ─────
// Not a timed benchmark; prints approximate RSS. Run under /usr/bin/time -v
// for external measurement.

test!{ fn Benchmark_Z_MemoryFootprint(t) {
    const N: usize = 100_000;
    let mut keep: Vec<goish::chan::Chan<i64>> = Vec::with_capacity(N);
    let start = Instant::now();
    for _ in 0..N {
        let c = chan!(i64, 1);
        let _ = c.Send(42);
        keep.push(c);
    }
    let dt = start.elapsed();
    let rss = approx_rss_kb();
    eprintln!("[{}] created {} cap=1 channels with 1 val each in {:?}; RSS={}KB",
        goish::chan::ENGINE, N, dt, rss);
    std::hint::black_box(keep);
}}

fn approx_rss_kb() -> i64 {
    if let Ok(s) = std::fs::read_to_string("/proc/self/status") {
        for line in s.lines() {
            if let Some(rest) = line.strip_prefix("VmRSS:") {
                let nums: String = rest.chars().filter(|c| c.is_ascii_digit()).collect();
                return nums.parse().unwrap_or(-1);
            }
        }
    }
    -1
}

// ── B7: Select-contended (2 goroutines racing on 2 channels) ──────────
//
// We don't have goish select! yet. Skip — this benchmark lands when the
// select! macro does in v0.5.1+. Placeholder so the suite has the slot
// explicitly reserved in docs.

// ── B8: Select-fairness distribution (requires select!) ───────────────
// Same: reserved placeholder.

// ── B9: N tight producer-consumer pairs in parallel ───────────────────
// Tests contention between independent channel operations.

benchmark!{ fn BenchmarkParallelProdCons_16pairs(b) {
    const PAIRS: usize = 16;
    let per_pair = b.N / PAIRS as i64;
    let acked = Arc::new(AtomicUsize::new(0));
    let mut pairs = Vec::new();
    for _ in 0..PAIRS {
        let c = chan!(i64, 16);
        let cp = c.clone();
        let ack = acked.clone();
        let producer = thread::spawn(move || {
            for i in 0..per_pair { let _ = cp.Send(i); }
        });
        let consumer = thread::spawn(move || {
            for _ in 0..per_pair {
                let _ = c.Recv();
            }
            ack.fetch_add(1, Ordering::SeqCst);
        });
        pairs.push((producer, consumer));
    }
    for (p, c) in pairs { p.join().unwrap(); c.join().unwrap(); }
    while b.Loop() {}
    assert_eq!(acked.load(Ordering::SeqCst), PAIRS);
}}
