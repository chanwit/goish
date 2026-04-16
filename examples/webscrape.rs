// webscrape: showcases v0.3 stdlib — url, regexp, json, hash/sha256, csv, path.
//
//   $ cargo run --example webscrape
//
// Starts from a list of "user agent" log lines, normalizes them with
// regexp + strings, extracts hosts via url.Parse, builds a checksum with
// sha256, emits a JSON summary + CSV table. No network — all local.

use goish::prelude::*;

fn main() {
    let lines = slice!([]string{
        "GET https://example.com/index.html 200",
        "POST https://api.example.com/v1/login?next=/home 204",
        "GET https://static.cdn.example.com/a/b/c.png 304",
        "GET   https://example.com/page?x=1&y=2  200  ",
        "GET https://alt.example.com/path 500",
    });

    // Normalize with strings helpers.
    let compact = regexp::MustCompile(r"\s+");
    let normalized: slice<string> = lines.iter()
        .map(|l| compact.ReplaceAllString(strings::TrimSpace(l), " "))
        .collect();

    // Parse out method / url / status.
    let parts_re = regexp::MustCompile(r"^(\w+)\s+(\S+)\s+(\d+)$");
    #[derive(Clone)]
    struct Req { method: string, host: string, path: string, status: int }
    let mut reqs: slice<Req> = slice::new();
    for l in &normalized {
        let caps = parts_re.FindStringSubmatch(l);
        if caps.is_empty() { continue; }
        let (u, err) = url::Parse(&caps[2]);
        if err != nil { continue; }
        reqs.push(Req {
            method: caps[1].clone(),
            host: u.Hostname(),
            path: u.Path.clone(),
            status: {
                let (n, _) = strconv::Atoi(&caps[3]);
                n
            },
        });
    }

    // Group counts by host.
    let mut by_host = make!(map[string]int);
    for r in &reqs {
        *by_host.entry(r.host.clone()).or_insert(0) += 1;
    }

    // Print a table.
    fmt::Printf!("%-32s %s\n", "host", "count");
    fmt::Println!(strings::Repeat("-", 40));
    let mut entries: slice<(string, int)> = by_host.iter()
        .map(|(k, v)| (k.clone(), *v)).collect();
    sort::Slice(&mut entries, |a, b| a.0 < b.0);
    for (h, c) in &entries {
        fmt::Printf!("%-32s %d\n", h, c);
    }

    // Build a JSON summary.
    let mut summary = json::Value::Object(Vec::new());
    summary.Set("total", json::Value::Number(reqs.len() as f64));
    let hosts_arr: slice<json::Value> = entries.iter()
        .map(|(h, c)| {
            let mut obj = json::Value::Object(Vec::new());
            obj.Set("host", json::Value::String(h.clone()));
            obj.Set("count", json::Value::Number(*c as f64));
            obj
        }).collect();
    summary.Set("hosts", json::Array(hosts_arr));
    let (out, _) = json::MarshalIndent(&summary, "", "  ");
    fmt::Println!();
    fmt::Println!("json:");
    fmt::Println!(String::from_utf8(out.clone()).unwrap());

    // Checksum the normalized input.
    let joined = strings::Join(&normalized, "\n");
    let digest = crypto::sha256::Sum256(joined.as_bytes());
    let hex_hash: string = digest.iter().map(|b| format!("{:02x}", b)).collect::<String>().into();
    fmt::Printf!("\nsha256 of input: %s\n", hex_hash);

    // Emit a CSV table of the parsed requests.
    let mut w = csv::NewWriter();
    w.Write(&["method", "host", "path", "status"]);
    for r in &reqs {
        w.Write(&[r.method.clone(), r.host.clone(), r.path.clone(), strconv::Itoa(r.status)]);
    }
    let csv_out = w.Flush();
    fmt::Println!();
    fmt::Println!("csv:");
    fmt::Println!(csv_out);

    // Compose a path using the slash-only path package (URL-friendly).
    let resource = path::Join(&["/api", "v1", "requests.json"]);
    fmt::Printf!("\nwould POST to: %s\n", resource);

    // Cleanly drop anything defer'd.
    defer!{ fmt::Println!("done."); }
}
