// csv: Go's encoding/csv package — RFC 4180 CSV Reader and Writer.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   r := csv.NewReader(strings.NewReader(s))
//                                       let mut r = csv::NewReader(s);
//   rec, err := r.Read()                let (rec, err) = r.Read();
//   all, err := r.ReadAll()             let (all, err) = r.ReadAll();
//
//   var buf bytes.Buffer
//   w := csv.NewWriter(&buf)            let mut w = csv::NewWriter();
//   w.Write([]string{"a","b"})          w.Write(&slice!([]string{"a","b"}));
//   w.Flush()                           let s = w.Flush();
//
// The writer intentionally targets an internal buffer (retrieved via Flush())
// rather than an arbitrary io::Write, to keep the Go-shaped call site clean.
// If you want to stream directly to a Buffer/File, combine `Write()` with
// `.Bytes()` and write yourself — see the tests.

use crate::errors::{error, nil, New};
use crate::types::{byte, int, slice, string};

// ── Reader ────────────────────────────────────────────────────────────

pub struct Reader {
    data: Vec<byte>,
    pos: usize,
    pub Comma: char,
    pub Comment: Option<char>,
    pub TrimLeadingSpace: bool,
    /// 0 = dynamic (set by first record); otherwise must match.
    pub FieldsPerRecord: int,
    line: int,
}

impl Reader {
    pub fn Read(&mut self) -> (slice<string>, error) {
        loop {
            if self.pos >= self.data.len() {
                return (Vec::new(), crate::io::EOF());
            }
            // Skip blank lines at the start of a record? Go keeps them.
            // Skip comment lines if Comment set.
            if let Some(cc) = self.Comment {
                if self.at_line_start() && self.data[self.pos] == cc as u8 {
                    self.consume_until_newline();
                    continue;
                }
            }
            let (rec, err) = self.read_record();
            if err != nil { return (rec, err); }
            if self.FieldsPerRecord == 0 {
                self.FieldsPerRecord = rec.len() as int;
            } else if rec.len() as int != self.FieldsPerRecord {
                return (rec, New(&format!(
                    "csv: record on line {}: wrong number of fields", self.line
                )));
            }
            return (rec, nil);
        }
    }

    pub fn ReadAll(&mut self) -> (Vec<slice<string>>, error) {
        let mut out = Vec::new();
        loop {
            let (rec, err) = self.Read();
            if err == crate::io::EOF() { return (out, nil); }
            if err != nil { return (out, err); }
            out.push(rec);
        }
    }

    fn at_line_start(&self) -> bool {
        self.pos == 0 || self.data[self.pos.saturating_sub(1)] == b'\n'
    }

    fn consume_until_newline(&mut self) {
        while self.pos < self.data.len() && self.data[self.pos] != b'\n' {
            self.pos += 1;
        }
        if self.pos < self.data.len() {
            self.pos += 1;
            self.line += 1;
        }
    }

    fn read_record(&mut self) -> (slice<string>, error) {
        let mut fields: slice<string> = Vec::new();
        let mut field = String::new();
        let comma = self.Comma as u8;
        loop {
            if self.TrimLeadingSpace {
                while self.pos < self.data.len()
                    && (self.data[self.pos] == b' ' || self.data[self.pos] == b'\t')
                {
                    self.pos += 1;
                }
            }
            if self.pos < self.data.len() && self.data[self.pos] == b'"' {
                // quoted field
                self.pos += 1;
                loop {
                    if self.pos >= self.data.len() {
                        return (fields, New(&format!(
                            "csv: unterminated quoted field on line {}",
                            self.line
                        )));
                    }
                    let b = self.data[self.pos];
                    if b == b'"' {
                        if self.pos + 1 < self.data.len() && self.data[self.pos + 1] == b'"' {
                            field.push('"');
                            self.pos += 2;
                        } else {
                            self.pos += 1;
                            break;
                        }
                    } else {
                        field.push(b as char);
                        self.pos += 1;
                    }
                }
            } else {
                while self.pos < self.data.len() {
                    let b = self.data[self.pos];
                    if b == comma || b == b'\n' || b == b'\r' { break; }
                    field.push(b as char);
                    self.pos += 1;
                }
            }
            fields.push(std::mem::take(&mut field));
            if self.pos >= self.data.len() { return (fields, nil); }
            let b = self.data[self.pos];
            if b == comma { self.pos += 1; continue; }
            if b == b'\r' {
                self.pos += 1;
                if self.pos < self.data.len() && self.data[self.pos] == b'\n' { self.pos += 1; }
                self.line += 1;
                return (fields, nil);
            }
            if b == b'\n' { self.pos += 1; self.line += 1; return (fields, nil); }
        }
    }
}

#[allow(non_snake_case)]
pub fn NewReader(s: impl AsRef<str>) -> Reader {
    Reader {
        data: s.as_ref().as_bytes().to_vec(),
        pos: 0,
        Comma: ',',
        Comment: None,
        TrimLeadingSpace: false,
        FieldsPerRecord: 0,
        line: 1,
    }
}

// ── Writer ────────────────────────────────────────────────────────────

pub struct Writer {
    buf: String,
    pub Comma: char,
    pub UseCRLF: bool,
}

impl Writer {
    pub fn Write(&mut self, record: &[impl AsRef<str>]) -> error {
        for (i, f) in record.iter().enumerate() {
            if i > 0 {
                self.buf.push(self.Comma);
            }
            let field = f.as_ref();
            let needs_quote = field.contains(self.Comma)
                || field.contains('"')
                || field.contains('\n')
                || field.contains('\r');
            if needs_quote {
                self.buf.push('"');
                for c in field.chars() {
                    if c == '"' { self.buf.push('"'); }
                    self.buf.push(c);
                }
                self.buf.push('"');
            } else {
                self.buf.push_str(field);
            }
        }
        if self.UseCRLF {
            self.buf.push_str("\r\n");
        } else {
            self.buf.push('\n');
        }
        nil
    }

    pub fn WriteAll(&mut self, records: &[slice<string>]) -> error {
        for r in records {
            let err = self.Write(r);
            if err != nil { return err; }
        }
        nil
    }

    /// Returns the buffered output, clears the internal buffer, and returns nil error.
    pub fn Flush(&mut self) -> string {
        std::mem::take(&mut self.buf)
    }

    pub fn Bytes(&self) -> &[u8] {
        self.buf.as_bytes()
    }

    pub fn String(&self) -> string {
        self.buf.clone()
    }

    pub fn Error(&self) -> error { nil }
}

#[allow(non_snake_case)]
pub fn NewWriter() -> Writer {
    Writer { buf: String::new(), Comma: ',', UseCRLF: false }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_simple() {
        let mut r = NewReader("a,b,c\n1,2,3\n");
        let (rec, err) = r.Read();
        assert_eq!(err, nil);
        assert_eq!(rec, vec!["a", "b", "c"]);
        let (rec, err) = r.Read();
        assert_eq!(err, nil);
        assert_eq!(rec, vec!["1", "2", "3"]);
        let (_, err) = r.Read();
        assert!(err != nil);
    }

    #[test]
    fn read_quoted_fields() {
        let mut r = NewReader("\"hello, world\",\"she said \"\"hi\"\"\"\n");
        let (rec, err) = r.Read();
        assert_eq!(err, nil);
        assert_eq!(rec, vec!["hello, world", "she said \"hi\""]);
    }

    #[test]
    fn read_all_collects() {
        let mut r = NewReader("a,b\n1,2\n3,4\n");
        let (all, err) = r.ReadAll();
        assert_eq!(err, nil);
        assert_eq!(all.len(), 3);
        assert_eq!(all[0], vec!["a", "b"]);
        assert_eq!(all[2], vec!["3", "4"]);
    }

    #[test]
    fn read_crlf_line_endings() {
        let mut r = NewReader("a,b\r\n1,2\r\n");
        let (all, err) = r.ReadAll();
        assert_eq!(err, nil);
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn read_with_comments() {
        let mut r = NewReader("# header\na,b\n1,2\n");
        r.Comment = Some('#');
        let (all, _) = r.ReadAll();
        assert_eq!(all, vec![vec!["a", "b"], vec!["1", "2"]]);
    }

    #[test]
    fn read_enforces_field_count() {
        let mut r = NewReader("a,b\n1,2,3\n");
        let _ = r.Read();
        let (_, err) = r.Read();
        assert!(err != nil);
    }

    #[test]
    fn write_simple() {
        let mut w = NewWriter();
        w.Write(&["a", "b", "c"]);
        w.Write(&["1", "2", "3"]);
        let s = w.Flush();
        assert_eq!(s, "a,b,c\n1,2,3\n");
    }

    #[test]
    fn write_quotes_special_chars() {
        let mut w = NewWriter();
        w.Write(&["hello, world", "she said \"hi\"", "normal"]);
        let s = w.Flush();
        assert_eq!(s, "\"hello, world\",\"she said \"\"hi\"\"\",normal\n");
    }

    #[test]
    fn round_trip() {
        let mut w = NewWriter();
        let records: Vec<Vec<String>> = vec![
            vec!["name".into(), "age".into()],
            vec!["Alice, CEO".into(), "30".into()],
            vec!["Bob".into(), "25".into()],
        ];
        w.WriteAll(&records);
        let s = w.Flush();
        let mut r = NewReader(&s);
        let (all, err) = r.ReadAll();
        assert_eq!(err, nil);
        assert_eq!(all, records);
    }
}
