# Goroutine runtime: the tokio + flume decision

**Status:** Shipped in goish v0.4.2. This document records the decision so
future contributors don't relitigate it.

## The bar

Go programs routinely spawn 1,000,000 goroutines. Examples: a worker-per-
connection HTTP server handling a burst, a fan-out/fan-in numerical
pipeline, a test suite that shards its fixtures concurrently. Any goish
runtime that caps out at a few thousand goroutines cannot honestly claim
"write Rust using Go idioms."

## Options considered

### 1. `std::thread::spawn` — the v0.3 starting point

Each goroutine is a live OS thread.

- Stack: ~2 MB per thread on Linux (the distro default).
- Switch cost: kernel context switch, ~1 µs.
- Ceiling: practical limit ~10k threads per process before VM / kernel
  tables hurt.

Verdict: fine for modest fan-out (pipelines with ≤100 workers) but fails
the 1M bar by three orders of magnitude.

### 2. `tokio::task::spawn_blocking` — our first tokio attempt

Runs sync-shaped bodies on tokio's pooled blocking threads. Tokio reuses
threads, which dodges the constant-creation overhead.

- Pool size: 512 threads by default.
- Each pool thread has a normal OS stack.

Verdict: the pool's hard cap kills any "N producers → 1 consumer" test
where N > 512. We ran straight into this with `TestChan_ManyGoroutines`
— producers filled the pool, the consumer couldn't get scheduled, and
the whole test deadlocked.

### 3. A bespoke M:N scheduler

Build our own work-stealing runtime à la Go's. Correct in principle but
a 6+ month project, not a milestone. Skipped for now; reconsidered if
tokio+flume ever hits a ceiling we can't work around.

### 4. `async-std` — alternate async runtime

Shape-compatible with tokio but smaller ecosystem, less maintenance
velocity in 2025. No clear win.

### 5. **tokio::task::spawn (async, not blocking)** — chosen

Each goroutine is a future scheduled on tokio's multi-threaded async
runtime.

- State size: ~200 B per task (a boxed future + bookkeeping).
- Switch cost: cooperative yield at `.await` points, no kernel involved.
- Ceiling: proven at 1,000,000 concurrent tasks in
  `tests/million_goroutines.rs` (~601 ms to spawn + increment + join
  1M tasks on commodity hardware).

The catch is that bodies must be async. We solve the surface-syntax
problem with `goish-macros`, which rewrites `c.Send(v)` / `c.Recv()` /
`g.Wait()` calls inside `go!{}` bodies into their `.send(v).await`
forms at compile time. The user writes code that looks identical to
non-goroutine code; the `.await` is invisible.

## Channel backend: flume vs async-channel

For MPMC channels with bounded/unbounded modes and both sync and async
receivers, the two real candidates in the Rust ecosystem were `flume`
and `async-channel`. We ran a 6-metric bake-off (commit `b996384`):

| metric                       | flume           | async-channel   |
|------------------------------|-----------------|-----------------|
| ping-pong latency            | **winner**      | —               |
| prod/cons (buf=1)            | **winner**      | —               |
| prod/cons (buf=1024)         | **winner**      | —               |
| prod/cons (unbounded)        | **winner**      | —               |
| 16 parallel sender/receiver  | **winner**      | —               |
| RSS for 100k channels        | **winner** 3.5× | —               |

flume was 2-127% faster on every metric and used 3.5× less memory per
channel for dense channel-graph workloads. We kept flume, deleted the
async-channel engine.

### Close semantics

Go's `close(c)` wakes every parked receiver (each then fires with
`(zero, false)` after the buffer drains). Flume doesn't have an
explicit "close" that wakes async receivers — dropping the last sender
closes it, but we can't do that because `Chan<T>` is a `Clone` handle
and every goroutine holds one.

Our fix: pair the flume channel with a `tokio::sync::Semaphore` used as
a closable wait-gate. On `Close()`, we close the semaphore. Every
parked async receiver sits in a `tokio::select!` against both the
flume recv future and the semaphore's `acquire()`; closing the
semaphore wakes all of them in a single scheduler cycle.

For v0.5.0 we also added Go's runtime-panic semantics: `send` on a
closed channel panics, `close` of a closed channel panics.

## What's still open

- **Pathological fan-in.** If 1M senders all park on one small-buffer
  channel, flume's wake-up is O(N) and the scheduler chokes. Realistic
  patterns (per-request channels, bounded fan-in) scale fine; we
  document the caveat. If a user hits it, the answer today is a
  sharded channel tree, same as Go.
- **Fairness in `select!`.** Our current `select!` fires cases in
  source order when multiple are ready. Go's spec says uniform-random.
  Not a correctness issue for typical code, but a known simplification.
- **Graceful server shutdown.** `http::ListenAndServe` has no
  `Shutdown()` method yet — the server goroutine lives until process
  exit. Tracking as follow-up.

## What would make us reconsider

- A benchmark-validated M:N runtime crate emerges that beats tokio+flume
  on goroutine-heavy workloads. (glommio exists but is io-uring-centric
  and less general.)
- Tokio gains a true rendezvous MPMC channel in `tokio::sync`. Today
  `tokio::sync::mpsc` is SPMC with no rendezvous mode.
- A user reports a workload where flume's O(N) contention is the actual
  bottleneck and sharding isn't an option.

Until one of those, the stack is stable.
