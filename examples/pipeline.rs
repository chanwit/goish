// pipeline: showcases sync.WaitGroup, sort, math, filepath, log, context.
//
//   $ cargo run --example pipeline
//
// Simulates computing sqrt + log on a set of numbers across parallel
// workers, collects results in a shared map under sync::Mutex, sorts the
// output, writes a summary file, and runs under a context::WithTimeout
// budget so a stray worker can't block the whole pipeline forever.

use goish::prelude::*;

#[derive(Clone, Default, Debug)]
struct Metric {
    n: float64,
    sqrt: float64,
    log: float64,
}

fn main() {
    let ctx = context::Background();
    let (ctx, cancel) = context::WithTimeout(ctx, time::Second * 2i64);
    defer!{ cancel.call(); }

    let inputs = slice!([]float64{1.0, 4.0, 9.0, 16.0, 25.0, 36.0, 49.0, 64.0, 81.0, 100.0});
    log::Printf!("starting pipeline over %d inputs", len!(inputs));

    let results: sync::Mutex<map<int, Metric>> = sync::Mutex::new(make!(map[int]Metric));
    let wg = sync::WaitGroup::new();

    // Fan out: one goroutine per input.
    for (i, n) in range!(inputs) {
        wg.Add(1);
        let wg = wg.clone();
        let results = results.clone();
        let ctx = ctx.clone();
        let i = i as int;
        let n = *n;
        let _ = go!{
            defer!{ wg.Done(); }
            if ctx.Err() != nil {
                log::Printf!("worker %d: skipping (ctx cancelled)", i);
                return;
            }
            time::Sleep(time::Millisecond * 5i64); // simulated work
            let m = Metric {
                n,
                sqrt: math::Sqrt(n),
                log: math::Log(n),
            };
            let mut g = results.Lock();
            g.insert(i, m);
        };
    }

    wg.Wait();

    // Collect + sort by input value.
    let collected: slice<Metric> = {
        let g = results.Lock();
        let mut v: slice<Metric> = slice::new();
        for m in g.values() {
            v.push(m.clone());
        }
        sort::Slice(&mut v, |a, b| a.n < b.n);
        v
    };

    // Summarise.
    log::Printf!("computed %d metrics", len!(collected));
    fmt::Println!();
    fmt::Printf!("%-8s %-10s %-10s\n", "n", "sqrt", "log");
    fmt::Println!(strings::Repeat("-", 30));
    for (_, m) in range!(collected) {
        fmt::Printf!("%-8.0f %-10.4f %-10.4f\n", m.n, m.sqrt, m.log);
    }

    // Write a summary under a temp path built with filepath::Join.
    let out_path = filepath::Join((os::TempDir(), "goish_pipeline_summary.txt"));
    let mut buf = bytes::Buffer::new();
    fmt::Fprintf!(&mut buf, "pipeline summary — %d metrics\n", len!(collected));
    for (_, m) in range!(collected) {
        fmt::Fprintf!(&mut buf, "%.0f -> sqrt=%.4f log=%.4f\n", m.n, m.sqrt, m.log);
    }
    let err = os::WriteFile(&out_path, buf.Bytes(), 0o644);
    if err != nil {
        log::Fatalf!("writing summary: %s", err);
    }
    log::Printf!("wrote summary to %s (%d bytes)", out_path, len!(buf));

    let (_, _) = os::ReadFile(&out_path); // round-trip demo
    let _ = os::Remove(&out_path);
}
