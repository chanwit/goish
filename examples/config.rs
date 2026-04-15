// config: parse a "key=value,key=value" config string into a typed struct.
//
//   $ cargo run --example config
//
// Showcases strings::Split/SplitN/TrimSpace/Contains, strconv::Atoi/ParseBool,
// and Go-style (val, err) error propagation through `if err != nil`.

use goish::prelude::*;

struct Config {
    port: int,
    host: string,
    debug: bool,
}

fmt::stringer! {
    impl Config {
        fn String(&self) -> string {
            fmt::Sprintf!("Config{port=%d host=%s debug=%t}", self.port, self.host, self.debug)
        }
    }
}

fn parse_config(input: impl AsRef<str>) -> (Config, error) {
    let mut cfg = Config { port: 0, host: string::new(), debug: false };

    for raw in strings::Split(input, ",") {
        let pair = strings::TrimSpace(&raw);
        if pair.is_empty() {
            continue;
        }
        if !strings::Contains(&pair, "=") {
            return (cfg, fmt::Errorf!("missing '=' in pair: %s", pair));
        }
        let parts = strings::SplitN(&pair, "=", 2);
        let key = strings::TrimSpace(&parts[0]);
        let val = strings::TrimSpace(&parts[1]);

        if key == "port" {
            let (n, err) = strconv::Atoi(&val);
            if err != nil {
                return (cfg, fmt::Errorf!("port: %s", err));
            }
            cfg.port = n;
        } else if key == "host" {
            cfg.host = val;
        } else if key == "debug" {
            let (b, err) = strconv::ParseBool(&val);
            if err != nil {
                return (cfg, fmt::Errorf!("debug: %s", err));
            }
            cfg.debug = b;
        } else {
            return (cfg, fmt::Errorf!("unknown key: %s", key));
        }
    }
    (cfg, nil)
}

fn main() {
    // ── happy path ─────────────────────────────────────────────────────
    let input = "port=8080, host=db.local, debug=true";
    let (cfg, err) = parse_config(input);
    if err != nil {
        fmt::Println!("error:", err);
        return;
    }
    fmt::Printf!("input  = %s\n", input);
    fmt::Println!("parsed:", cfg);

    // ── various error paths ────────────────────────────────────────────
    let bad_inputs = slice!([]string{"port=abc", "host", "debug=maybe", "color=red"});
    for bad in &bad_inputs {
        let (_, err) = parse_config(bad);
        fmt::Printf!("bad %-15s -> %s\n", strconv::Quote(bad), err);
    }

    // ── strings/strconv quick tour ─────────────────────────────────────
    let words = strings::Fields("  hello   world   go  ");
    fmt::Printf!("fields  = [%s]\n", strings::Join(&words, "|"));
    fmt::Printf!("upper   = %s\n", strings::ToUpper("goish"));
    fmt::Printf!("repeat  = %s\n", strings::Repeat("-", 10));
    fmt::Printf!("count a = %d\n", strings::Count("banana", "a"));

    fmt::Printf!("hex 255 = %s\n", strconv::FormatInt(255, 16));
    fmt::Printf!("itoa    = %s\n", strconv::Itoa(-42));

    let (n, _) = strconv::ParseInt("0xff", 0, 64);
    fmt::Printf!("0xff    = %d\n", n);
}
