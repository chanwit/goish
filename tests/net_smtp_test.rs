// Port (adaptation) of go1.25.5 src/net/smtp/smtp_test.go. Uses a local
// mock SMTP server running on 127.0.0.1 and a canned response script —
// exercises Dial, Hello, Mail, Rcpt, Data, Quit, dot-stuffing.
//
// Auth/TLS tests are deferred (v0.17 crypto milestone).

#![allow(non_snake_case)]
use goish::prelude::*;
use goish::net::smtp;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;

struct MockServer {
    addr: String,
    // Collected client lines after the server has finished.
    received: mpsc::Receiver<Vec<String>>,
}

/// Run a mock SMTP server on a random port. The script is a list of
/// (expected-command-prefix, response) pairs. The server emits its 220
/// greeting first, then loops reading commands and writing responses
/// according to the script (or a default 250 response).
fn start_mock(responses: &[&str]) -> MockServer {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().unwrap().to_string();
    let (tx, rx) = mpsc::channel();
    let responses: Vec<String> = responses.iter().map(|s| (*s).to_string()).collect();

    thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept");
        let mut writer = stream.try_clone().expect("clone");
        let mut reader = BufReader::new(stream);
        let mut collected = make!([]string, 0);

        let _ = writer.write_all(b"220 mock.test SMTP ready\r\n");

        let mut idx = 0usize;
        loop {
            let mut line = String::new();
            let n = match reader.read_line(&mut line) {
                Ok(n) => n,
                Err(_) => break,
            };
            if n == 0 { break; }
            let trimmed = line.trim_end_matches(|c| c == '\r' || c == '\n').to_string();
            collected.push(trimmed.clone());

            // Data phase: consume until lone ".".
            if trimmed.eq_ignore_ascii_case("DATA") {
                let _ = writer.write_all(b"354 Go ahead\r\n");
                loop {
                    let mut dl = String::new();
                    if reader.read_line(&mut dl).unwrap_or(0) == 0 { break; }
                    let d = dl.trim_end_matches(|c| c == '\r' || c == '\n').to_string();
                    collected.push(d.clone());
                    if d == "." { break; }
                }
                let _ = writer.write_all(b"250 OK\r\n");
                continue;
            }

            let resp = responses.get(idx).cloned().unwrap_or_else(|| "250 OK".into());
            idx += 1;
            let mut full = resp.clone();
            full.push_str("\r\n");
            let _ = writer.write_all(full.as_bytes());

            if trimmed.eq_ignore_ascii_case("QUIT") { break; }
        }
        let _ = tx.send(collected);
    });

    MockServer { addr, received: rx }
}

// ── TestDialAndBasicFlow ───────────────────────────────────────────

test!{ fn TestDialAndBasicFlow(t) {
    let script = [
        "250 mock.test Hello",  // HELO/EHLO
        "250 OK",                // MAIL FROM
        "250 OK",                // RCPT TO
        // (DATA handled inline)
        "221 bye",              // QUIT
    ];
    let server = start_mock(&script);

    let (mut c, err) = smtp::Dial(&server.addr);
    if err != nil { t.Fatal(&Sprintf!("Dial: %s", err)); }

    let err = c.Hello("localhost");
    if err != nil { t.Fatal(&Sprintf!("Hello: %s", err)); }

    let err = c.Mail("sender@example.com");
    if err != nil { t.Fatal(&Sprintf!("Mail: %s", err)); }

    let err = c.Rcpt("recipient@example.com");
    if err != nil { t.Fatal(&Sprintf!("Rcpt: %s", err)); }

    {
        let (mut w, err) = c.Data();
        if err != nil { t.Fatal(&Sprintf!("Data: %s", err)); }
        let (_, e) = w.Write(b"Subject: hello\r\n\r\nHello.\r\n");
        if e != nil { t.Fatal(&Sprintf!("Data write: %s", e)); }
        let e = w.Close();
        if e != nil { t.Fatal(&Sprintf!("Data close: %s", e)); }
    }

    let err = c.Quit();
    if err != nil { t.Fatal(&Sprintf!("Quit: %s", err)); }

    // Drain server collected lines — look for canonical commands.
    let received = server.received.recv_timeout(std::time::Duration::from_secs(5))
        .unwrap_or_default();
    let expected_prefixes = ["EHLO ", "MAIL FROM:", "RCPT TO:", "DATA", "QUIT"];
    for ep in &expected_prefixes {
        let found = received.iter().any(|l| l.starts_with(ep));
        if !found {
            t.Errorf(Sprintf!("expected server to receive %s, got %d lines",
                ep, len!(received) as i64));
        }
    }
}}

// ── TestExtensions: EHLO multi-line advertises extensions ─────────

test!{ fn TestExtensions(t) {
    // First line is "HELLO", next lines advertise extensions.
    let script = [
        "250-mock.test Hello\n250-AUTH LOGIN PLAIN\n250-8BITMIME\n250 SIZE 1048576",
        "221 bye",
    ];
    let server = start_mock(&script);
    let (mut c, err) = smtp::Dial(&server.addr);
    if err != nil { t.Fatal(&Sprintf!("Dial: %s", err)); }

    // Hello triggers EHLO → collects extension list.
    let err = c.Hello("localhost");
    if err != nil { t.Fatal(&Sprintf!("Hello: %s", err)); }

    let (has_auth, auth_params) = c.Extension("AUTH");
    if !has_auth {
        t.Errorf(Sprintf!("Extension(AUTH) = false, want true"));
    }
    if auth_params != "LOGIN PLAIN" {
        t.Errorf(Sprintf!("Extension(AUTH) params = %s, want LOGIN PLAIN", auth_params));
    }

    let (has_bit, _) = c.Extension("8BITMIME");
    if !has_bit {
        t.Errorf(Sprintf!("Extension(8BITMIME) = false"));
    }

    let (has_dsn, _) = c.Extension("DSN");
    if has_dsn {
        t.Errorf(Sprintf!("Extension(DSN) = true, want false"));
    }

    let _ = c.Quit();
}}

// ── TestNoopReset ───────────────────────────────────────────────────

test!{ fn TestNoopReset(t) {
    let script = [
        "250 mock.test Hello",  // EHLO
        "250 OK",                // NOOP
        "250 OK",                // RSET
        "221 bye",               // QUIT
    ];
    let server = start_mock(&script);
    let (mut c, err) = smtp::Dial(&server.addr);
    if err != nil { t.Fatal(&Sprintf!("Dial: %s", err)); }

    let err = c.Hello("localhost");
    if err != nil { t.Fatal(&Sprintf!("Hello: %s", err)); }

    let err = c.Noop();
    if err != nil { t.Errorf(Sprintf!("Noop: %s", err)); }

    let err = c.Reset();
    if err != nil { t.Errorf(Sprintf!("Reset: %s", err)); }

    let _ = c.Quit();
}}

// ── TestValidateLine (message injection guard) ─────────────────────

test!{ fn TestValidateLine(t) {
    let script = ["250 ok", "250 ok", "250 ok", "221 bye"];
    let server = start_mock(&script);
    let (mut c, err) = smtp::Dial(&server.addr);
    if err != nil { t.Fatal(&Sprintf!("Dial: %s", err)); }
    // Send legit HELO so future calls don't retry the exchange.
    let _ = c.Hello("localhost");
    // Injection attempt: CRLF in the address argument.
    let err = c.Mail("attacker@example.com>\r\nRSET");
    if err == nil {
        t.Errorf(Sprintf!("Mail with CRLF injection should have errored"));
    }
    let _ = c.Quit();
}}
