// 1,000,000 goroutines — prove we're on a lightweight async task scheduler,
// not an OS-thread pool. Equivalent to Go programs that spawn 1M goroutines.
//
// Run with:
//   cargo test --release --test million_goroutines -- --nocapture --test-threads=1
//
// Expected: completes in a few seconds, RSS stays under ~500 MB.

#![allow(non_snake_case)]
use goish::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

#[test]
fn spawn_million_goroutines() {
    const N: usize = 1_000_000;
    let counter = Arc::new(AtomicUsize::new(0));

    let start = Instant::now();
    let mut handles = Vec::with_capacity(N);
    for _ in 0..N {
        let c = counter.clone();
        handles.push(go!{
            c.fetch_add(1, Ordering::Relaxed);
        });
    }
    let spawned_at = start.elapsed();

    for h in handles { let _ = h.Wait(); }
    let done_at = start.elapsed();

    let got = counter.load(Ordering::SeqCst);
    eprintln!("spawned {} goroutines in {:?}", N, spawned_at);
    eprintln!("joined  {} goroutines in {:?}", N, done_at);
    eprintln!("RSS after = {} KB", rss_kb());
    assert_eq!(got, N, "expected {} increments, got {}", N, got);
}

#[test]
fn hundred_k_producers_one_consumer() {
    // Every goroutine sends one value to a shared channel. A single consumer
    // drains N. Proves the runtime can park senders waiting for buffer space
    // without exhausting a thread pool.
    //
    // Instead of sequentially calling `h.Wait()` from main (each block_on
    // has overhead), use a "done" channel — consumer signals completion and
    // all producers' tasks naturally finish on their own once sends settle.
    // 1M producers all contending for a single small-buffer channel is
    // pathological: every producer has to park in the channel's wait queue,
    // and flume's O(N) wake-up hurts at that scale. 100k stays realistic.
    const N: usize = 100_000;
    let ch: goish::chan::Chan<i64> = chan!(i64, 1024);
    let done = chan!(bool, 1);

    let start = Instant::now();

    // Consumer goroutine — drains N values.
    let cc = ch.clone();
    let done_c = done.clone();
    let consumer = go!{
        let mut sum = 0i64;
        for _ in 0..N {
            let (v, _) = cc.recv().await;
            sum += v;
        }
        let _ = done_c.send(true).await;
        let _ = sum;
    };

    // Producers — fire-and-forget; no joins.
    for i in 0..N {
        let c = ch.clone();
        let _ = go!{
            c.send(i as i64).await;
        };
    }

    // Single Wait on the "done" signal.
    let (_, _) = done.Recv();
    let _ = consumer.Wait();

    eprintln!("{}k producer/single consumer: {:?}, RSS={}KB", N / 1000, start.elapsed(), rss_kb());
}

/// Realistic "1M concurrent clients" pattern — each client has its own
/// tiny reply channel and waits on IT rather than all contending on one
/// global queue. This is how Go RPC servers work, and it's the pattern
/// that justifies M:N scheduling.
#[test]
fn million_goroutines_per_request_channels() {
    const N: usize = 1_000_000;
    let counter = Arc::new(AtomicUsize::new(0));

    let start = Instant::now();

    // Each "request" has its own reply channel. Main spawns N goroutines
    // each with its own reply channel, then spawns N "server" goroutines
    // that respond. 2M goroutines total, minimal contention per channel.
    let mut replies = Vec::with_capacity(N);
    for i in 0..N {
        let reply = chan!(i64, 1);
        let reply_server = reply.clone();
        // Server goroutine — sends i back on its private reply channel.
        let _ = go!{
            reply_server.send(i as i64).await;
        };
        replies.push(reply);
    }

    // Client-aggregator — consumes all replies, tallying.
    let counter_c = counter.clone();
    let aggregator = go!{
        for reply in replies {
            let (_, _) = reply.recv().await;
            counter_c.fetch_add(1, Ordering::Relaxed);
        }
    };

    let _ = aggregator.Wait();

    let got = counter.load(Ordering::SeqCst);
    let elapsed = start.elapsed();
    eprintln!("{}k per-request goroutines: {:?}, RSS={}KB",
        N / 1000, elapsed, rss_kb());
    assert_eq!(got, N);
}

fn rss_kb() -> i64 {
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
