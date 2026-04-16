// smtp: Go's net/smtp — minimal SMTP client.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   c, err := smtp.Dial(addr)           let (c, err) = smtp::Dial(addr);
//   c.Hello("localhost")                c.Hello("localhost");
//   c.Mail("from@x")                     c.Mail("from@x");
//   c.Rcpt("to@y")                       c.Rcpt("to@y");
//   w, _ := c.Data()                    let (w, _) = c.Data();
//   w.Write(bytes)                       w.Write(bytes);
//   w.Close()                            w.Close();
//   c.Quit()                              c.Quit();
//
// TLS/STARTTLS and SASL AUTH mechanisms are deferred (v0.17 crypto).
// The Client exposes its TCP stream as a generic Read+Write so tests
// can inject an in-memory transport.

#![allow(dead_code)]

use crate::errors::{error, nil, New};
use crate::types::string;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;

pub struct Client<C: Read + Write> {
    r: BufReader<BoxedConn<C>>,
    // separate write-side reference; BufReader takes ownership of Read.
    // We split by duplicating via try_clone for TcpStream; for arbitrary
    // R+W we require the connection to support Clone or use split IO.
    // Since our test conn is a simple Vec pair, keep a secondary handle.
    localName: string,
    didHello: bool,
    pub Ext: HashMap<string, string>,
}

/// Wrap a Read + Write so the BufReader owns a handle while Write stays
/// reachable. For TcpStream we try_clone; for generic conns we require
/// the user to supply already-split read/write halves via `new_split`.
struct BoxedConn<C: Read + Write> {
    inner: C,
    writer: Option<Box<dyn Write + Send>>,
}

impl<C: Read + Write> Read for BoxedConn<C> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> { self.inner.read(buf) }
}

impl<C: Read + Write> BoxedConn<C> {
    fn write_all(&mut self, data: &[u8]) -> std::io::Result<()> {
        if let Some(w) = self.writer.as_mut() {
            w.write_all(data)
        } else {
            self.inner.write_all(data)
        }
    }
}

// ── Dial (TCP) ──────────────────────────────────────────────────────

pub fn Dial(addr: &str) -> (Client<TcpStream>, error) {
    let stream = match TcpStream::connect(addr) {
        Ok(s) => s,
        Err(e) => return (Client::dummy(), New(&e.to_string())),
    };
    let writer_clone = match stream.try_clone() {
        Ok(c) => c,
        Err(e) => return (Client::dummy(), New(&e.to_string())),
    };
    let host = addr.split(':').next().unwrap_or(addr).to_string();
    let boxed = BoxedConn { inner: stream, writer: Some(Box::new(writer_clone)) };
    let mut c = Client {
        r: BufReader::new(boxed),
        localName: "localhost".to_string(),
        didHello: false,
        Ext: HashMap::new(),
    };
    // Read greeting: expect 220.
    let (_code, _msg, err) = c.read_response(220);
    if err != nil { return (c, err); }
    let _ = host;
    (c, nil)
}

impl Client<TcpStream> {
    fn dummy() -> Client<TcpStream> {
        // Dummy client with no connection; only for error-path returns.
        // Reads will immediately fail.
        let dummy_stream = std::net::TcpListener::bind("127.0.0.1:0")
            .and_then(|l| {
                let _p = l.local_addr()?;
                TcpStream::connect(l.local_addr()?)
            });
        let stream = match dummy_stream {
            Ok(s) => s,
            Err(_) => unreachable!("dummy fallback unreachable"),
        };
        let boxed = BoxedConn { inner: stream, writer: None };
        Client {
            r: BufReader::new(boxed),
            localName: String::new(),
            didHello: false,
            Ext: HashMap::new(),
        }
    }
}

// ── Constructor for testable (split) connection ────────────────────

impl<C: Read + Write + Send + 'static> Client<C> {
    /// Construct a Client from an already-paired read half and write half.
    /// Used by tests to inject in-memory pipes.
    pub fn NewClientSplit<R: Read + 'static>(r: R, w: Box<dyn Write + Send>, _host: &str) -> (Client<NullConn>, error)
    where R: Send {
        let boxed = BoxedConn { inner: NullConn::with_reader(Box::new(r)), writer: Some(w) };
        let mut c = Client::<NullConn> {
            r: BufReader::new(boxed),
            localName: "localhost".to_string(),
            didHello: false,
            Ext: HashMap::new(),
        };
        let (_code, _msg, err) = c.read_response(220);
        (c, err)
    }
}

// Placeholder type for tests' split-IO constructor.
pub struct NullConn { r: Box<dyn Read + Send> }
impl NullConn { pub fn with_reader(r: Box<dyn Read + Send>) -> NullConn { NullConn { r } } }
impl Read for NullConn {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> { self.r.read(buf) }
}
impl Write for NullConn {
    fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> { Ok(_buf.len()) }
    fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
}

// ── Client ops ─────────────────────────────────────────────────────

impl<C: Read + Write> Client<C> {
    pub fn Close(&mut self) -> error {
        // BufReader doesn't offer a close; rely on drop.
        nil
    }

    pub fn Hello(&mut self, localName: &str) -> error {
        if let Some(e) = validate_line(localName) { return e; }
        if self.didHello {
            return New("smtp: Hello called after other methods");
        }
        self.localName = localName.to_string();
        self.hello()
    }

    fn hello(&mut self) -> error {
        if self.didHello { return nil; }
        self.didHello = true;
        if self.ehlo().is_err() {
            return self.helo();
        }
        nil
    }

    fn helo(&mut self) -> error {
        let line = format!("HELO {}", self.localName);
        let (_, _, err) = self.cmd(250, &line);
        err
    }

    fn ehlo(&mut self) -> Result<(), ()> {
        let line = format!("EHLO {}", self.localName);
        let (_code, msg, err) = self.cmd(250, &line);
        if err != nil { return Err(()); }
        let mut ext = HashMap::new();
        let lines: Vec<&str> = msg.split('\n').collect();
        if lines.len() > 1 {
            for line in &lines[1..] {
                let (k, v) = match line.find(' ') {
                    Some(i) => (&line[..i], &line[i + 1..]),
                    None => (&line[..], ""),
                };
                ext.insert(k.to_string(), v.to_string());
            }
        }
        self.Ext = ext;
        Ok(())
    }

    pub fn Mail(&mut self, from: &str) -> error {
        if let Some(e) = validate_line(from) { return e; }
        let err = self.hello();
        if err != nil { return err; }
        let (_, _, err) = self.cmd(250, &format!("MAIL FROM:<{}>", from));
        err
    }

    pub fn Rcpt(&mut self, to: &str) -> error {
        if let Some(e) = validate_line(to) { return e; }
        let (_, _, err) = self.cmd(25, &format!("RCPT TO:<{}>", to));
        err
    }

    pub fn Data(&mut self) -> (DataWriter<'_, C>, error) {
        let (_, _, err) = self.cmd(354, "DATA");
        if err != nil { return (DataWriter::dummy(), err); }
        (DataWriter { client: Some(self), closed: false, at_line_start: true }, nil)
    }

    pub fn Quit(&mut self) -> error {
        let (_, _, err) = self.cmd(221, "QUIT");
        err
    }

    pub fn Noop(&mut self) -> error {
        let (_, _, err) = self.cmd(250, "NOOP");
        err
    }

    pub fn Reset(&mut self) -> error {
        let (_, _, err) = self.cmd(250, "RSET");
        err
    }

    pub fn Verify(&mut self, addr: &str) -> error {
        if let Some(e) = validate_line(addr) { return e; }
        let (_, _, err) = self.cmd(250, &format!("VRFY {}", addr));
        err
    }

    // ── Lower-level cmd + response ──

    fn cmd(&mut self, expect: i64, line: &str) -> (i64, string, error) {
        // Write line + CRLF.
        let mut payload = line.to_string();
        payload.push_str("\r\n");
        if let Err(e) = self.r.get_mut().write_all(payload.as_bytes()) {
            return (0, String::new(), New(&e.to_string()));
        }
        self.read_response(expect)
    }

    /// Read an SMTP response. Handles multiline "250-foo\r\n250 bar\r\n".
    fn read_response(&mut self, expect: i64) -> (i64, string, error) {
        let mut acc_msg = String::new();
        let mut code: i64 = 0;
        loop {
            let mut line = String::new();
            match self.r.read_line(&mut line) {
                Ok(0) => return (0, acc_msg, New("EOF")),
                Ok(_) => {}
                Err(e) => return (0, acc_msg, New(&e.to_string())),
            }
            while line.ends_with('\n') || line.ends_with('\r') { line.pop(); }
            if line.len() < 4 {
                return (0, acc_msg, New(&format!("smtp: short response: {}", line)));
            }
            let c: i64 = line[..3].parse().unwrap_or(0);
            code = c;
            let sep = line.as_bytes()[3];
            let msg = &line[4..];
            if !acc_msg.is_empty() { acc_msg.push('\n'); }
            acc_msg.push_str(msg);
            if sep == b' ' { break; } // final line
            if sep != b'-' {
                return (code, acc_msg, New(&format!("smtp: bad response line: {}", line)));
            }
        }
        let err = if expect != 0 && !expects_match(code, expect) {
            New(&format!("{} {}", code, acc_msg))
        } else {
            nil
        };
        (code, acc_msg, err)
    }
}

fn expects_match(got: i64, want: i64) -> bool {
    // Go's textproto.Conn: want can be a partial prefix (e.g. 25 matches any 25x).
    if got == want { return true; }
    let w_str = want.to_string();
    let g_str = got.to_string();
    g_str.starts_with(&w_str)
}

// ── DataWriter ──────────────────────────────────────────────────────

pub struct DataWriter<'a, C: Read + Write> {
    client: Option<&'a mut Client<C>>,
    closed: bool,
    at_line_start: bool,
}

impl<'a, C: Read + Write> DataWriter<'a, C> {
    fn dummy() -> DataWriter<'a, C> {
        DataWriter { client: None, closed: true, at_line_start: true }
    }

    pub fn Write(&mut self, data: &[u8]) -> (i64, error) {
        let c = match &mut self.client {
            Some(c) => c,
            None => return (0, New("smtp: data writer not open")),
        };
        for &b in data {
            // Dot-stuffing at line start.
            if self.at_line_start && b == b'.' {
                if let Err(e) = c.r.get_mut().write_all(b".") {
                    return (0, New(&e.to_string()));
                }
            }
            if let Err(e) = c.r.get_mut().write_all(&[b]) {
                return (0, New(&e.to_string()));
            }
            self.at_line_start = b == b'\n';
        }
        (data.len() as i64, nil)
    }

    pub fn Close(&mut self) -> error {
        if self.closed { return nil; }
        self.closed = true;
        let c = match self.client.take() {
            Some(c) => c,
            None => return nil,
        };
        // Terminate data.
        let tail: &[u8] = if self.at_line_start { b".\r\n" } else { b"\r\n.\r\n" };
        if let Err(e) = c.r.get_mut().write_all(tail) {
            return New(&e.to_string());
        }
        let (_, _, err) = c.read_response(250);
        err
    }
}

// ── helpers ─────────────────────────────────────────────────────────

fn validate_line(line: &str) -> Option<error> {
    if line.contains('\r') || line.contains('\n') {
        return Some(New("smtp: A line must not contain CR or LF"));
    }
    None
}
