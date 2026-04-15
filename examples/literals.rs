// literals: tour of slice!, map!, and chan! composite literal macros.
//
//   $ cargo run --example literals
//
// Goal: read like Go.
//
//   Go                                       goish
//   ───────────────────────────────────      ──────────────────────────────────
//   names := []string{"a","b","c"}            slice!([]string{"a","b","c"})
//   ports := []int{80, 443, 8080}             slice!([]int{80, 443, 8080})
//   env   := map[string]int{"PORT": 5432}     map!([string]int{"PORT" => 5432})
//   jobs  := make(chan int, 10)               chan!(int, 10)
//   done  := make(chan bool)                  chan!(bool)

use goish::prelude::*;

fn main() {
    // ── slice literals ────────────────────────────────────────────────
    let names: slice<string> = slice!([]string{"alpha", "beta", "gamma"});
    let ports: slice<int>    = slice!([]int{80, 443, 8080});

    fmt::Printf!("names = [%s]\n", strings::Join(&names, ", "));
    fmt::Printf!("ports = ");
    for p in &ports {
        fmt::Printf!("%d ", p);
    }
    fmt::Println!();

    // ── map literal ───────────────────────────────────────────────────
    let env: map<string, int> = map!([string]int{
        "PORT"    => 5432,
        "TTL"     => 60,
        "RETRIES" => 3,
    });

    let opts = map!([string]string{
        "host" => "db.local",
        "user" => "goish",
    });

    fmt::Println!("env entries:", env.len());
    // Map iteration order is unspecified — sort for stable demo output.
    let mut env_keys: slice<&string> = env.keys().collect();
    env_keys.sort();
    for k in &env_keys {
        fmt::Printf!("  %-8s = %d\n", k, env[*k]);
    }

    fmt::Println!("opts.host =", opts["host"]);

    // ── channel literals + cross-thread send/recv ─────────────────────
    let jobs: Chan<int> = chan!(int, 4);
    let done: Chan<bool> = chan!(bool, 1);

    // Producer thread (stand-in for `go func() { ... }()`).
    let producer = jobs.clone();
    let signaller = done.clone();
    std::thread::spawn(move || {
        for i in 1..=4 {
            producer.Send(i * 10);
        }
        signaller.Send(true);
    });

    // Consumer (main).
    for _ in 0..4 {
        let (v, _ok) = jobs.Recv();
        fmt::Printf!("got: %d\n", v);
    }
    let (_, _) = done.Recv();
    fmt::Println!("producer done");

    // ── len() and append() builtins ───────────────────────────────────
    let nums = slice!([]int{1, 2, 3});
    let nums = append!(nums, 4, 5, 6);
    fmt::Printf!("nums      = %d items: ", len!(nums));
    for n in &nums { fmt::Printf!("%d ", n); }
    fmt::Println!();

    let words = append!(slice!([]string{"go"}), "ro", "lib");
    fmt::Printf!("words     = %d items: %s\n", len!(words), strings::Join(&words, " "));

    fmt::Printf!("env len   = %d\n", len!(env));
    fmt::Printf!("greeting  = %d chars\n", len!("hello, world"));

    // ── delete() builtin ──────────────────────────────────────────────
    let mut counters: map<string, int> = map!([string]int{
        "a" => 1, "b" => 2, "c" => 3,
    });
    fmt::Printf!("before    = %d entries\n", len!(counters));
    delete!(counters, "b");
    delete!(counters, "missing");  // no-op, like Go
    fmt::Printf!("after     = %d entries (b removed)\n", len!(counters));
}
