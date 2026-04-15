// worker: a small worker-pool demo that exercises most of goish.
//
//   $ cargo run --example worker
//
// Go-idiomatic features in use:
//   - go!{...}      goroutines (spawn worker threads)
//   - chan!(...)    channels for jobs and results
//   - defer!{...}   scope-end cleanup
//   - time::Sleep   simulated work
//   - time::Since   measure elapsed duration
//   - range!(...)   Go-style for k, v := range m
//   - strings::    text helpers
//   - os::Args    command-line arguments
//   - fmt::Println, Printf, Errorf, Stringer

use goish::prelude::*;

// A job: an integer to square.
#[derive(Clone, Default)]
struct Job { id: int, n: int }

fmt::stringer! {
    impl Job {
        fn String(&self) -> string {
            fmt::Sprintf!("Job#%d(n=%d)", self.id, self.n)
        }
    }
}

// A result the worker emits back.
#[derive(Clone, Default)]
#[allow(dead_code)]
struct Result { job_id: int, square: int, worker: int }

fn worker(id: int, jobs: Chan<Job>, results: Chan<Result>) {
    defer!{ fmt::Printf!("worker %d: shutting down\n", id); }
    loop {
        let (job, _) = jobs.Recv();
        if job.id < 0 { return; }  // sentinel: id<0 → stop

        // Simulated work.
        time::Sleep(time::Millisecond * 20i64);
        let r = Result { job_id: job.id, square: job.n * job.n, worker: id };
        results.Send(r);
    }
}

fn main() {
    // Optional CLI: `cargo run --example worker 8` → 8 jobs.
    let args = os::Args();
    let n_jobs: int = if args.len() >= 2 {
        let (n, err) = strconv::Atoi(&args[1]);
        if err != nil {
            fmt::Println!("bad arg:", err);
            os::Exit(1);
        }
        n
    } else { 6 };

    fmt::Printf!("running %d jobs across 3 workers...\n", n_jobs);
    let start = time::Now();

    let jobs: Chan<Job> = chan!(Job, 16);
    let results: Chan<Result> = chan!(Result, 16);

    // Fan out: 3 workers.
    let mut handles: slice<Goroutine> = make!([]Goroutine, 0, 3);
    for w in 1i64..=3 {
        let j = jobs.clone();
        let r = results.clone();
        handles.push(go!{ worker(w, j, r); });
    }

    // Feed jobs, then one sentinel per worker to signal "stop".
    // (close!(&jobs) also works — Go programs use either pattern; sentinels
    // compose best when producers outnumber the consumers.)
    for i in 1i64..=n_jobs {
        jobs.Send(Job { id: i, n: i + 10 });
    }
    for _ in 0..3 {
        jobs.Send(Job { id: -1, n: 0 });  // sentinel
    }

    // Collect results.
    let mut by_worker: map<int, slice<int>> = make!(map[int]slice<int>);
    for _ in 0..n_jobs {
        let (r, _) = results.Recv();
        let entry = by_worker.entry(r.worker).or_insert_with(|| slice::<int>::new());
        entry.push(r.square);
    }

    // Wait for workers to finish (they'll exit after their sentinel job).
    range!(handles, |g| { let _ = g.Wait(); });

    // Summary.
    fmt::Println!();
    fmt::Println!("results by worker:");
    let mut worker_ids: slice<&int> = by_worker.keys().collect();
    worker_ids.sort();
    range!(&worker_ids, |_i, wid| {
        let squares = &by_worker[*wid];
        let as_strs: slice<string> = squares.iter().map(|n| strconv::Itoa(*n)).collect();
        fmt::Printf!("  worker %d: %d squares [%s]\n",
            **wid, len!(squares), strings::Join(&as_strs, ", "));
    });

    let elapsed = time::Since(start);
    fmt::Printf!("\nfinished in %s\n", elapsed);
}
