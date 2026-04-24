// Port of go1.25.5/src/strings/reader_test.go — strings.Reader exercised
// through Read/Seek/ReadAt. Skips Go's concurrency + WriteTo + RuneReader
// sub-cases (tracked for v0.6.x follow-up).

#![allow(non_snake_case)]
use goish::prelude::*;

// SeekStart/Current/End match Go's io package constants.
const SEEK_START:   i64 = 0;
const SEEK_CURRENT: i64 = 1;
const SEEK_END:     i64 = 2;

struct ReadCase {
    off: i64,
    seek: i64,
    n: usize,
    want: &'static str,
    wantpos: i64,
    read_eof: bool,
    seek_err: &'static str,
}

test!{ fn TestReader(t) {
    let mut r = strings::NewReader("0123456789");
    let cases: Vec<ReadCase> = vec![
        ReadCase { seek: SEEK_START,   off: 0, n: 20, want: "0123456789", wantpos: 0, read_eof: false, seek_err: "" },
        ReadCase { seek: SEEK_START,   off: 1, n: 1,  want: "1",         wantpos: 0, read_eof: false, seek_err: "" },
        ReadCase { seek: SEEK_CURRENT, off: 1, n: 2,  want: "34",        wantpos: 3, read_eof: false, seek_err: "" },
        ReadCase { seek: SEEK_START,   off: -1, n: 0, want: "",          wantpos: 0, read_eof: false, seek_err: "strings.Reader.Seek: negative position" },
        ReadCase { seek: SEEK_START,   off: 1 << 33, n: 0, want: "",     wantpos: 1 << 33, read_eof: true,  seek_err: "" },
        ReadCase { seek: SEEK_CURRENT, off: 1, n: 0, want: "",           wantpos: (1 << 33) + 1, read_eof: true, seek_err: "" },
        ReadCase { seek: SEEK_START,   off: 0, n: 5, want: "01234",      wantpos: 0, read_eof: false, seek_err: "" },
        ReadCase { seek: SEEK_CURRENT, off: 0, n: 5, want: "56789",      wantpos: 0, read_eof: false, seek_err: "" },
        ReadCase { seek: SEEK_END,     off: -1, n: 1, want: "9",         wantpos: 9, read_eof: false, seek_err: "" },
    ];

    for (i, tt) in cases.iter().enumerate() {
        let (pos, err) = r.Seek(tt.off, tt.seek);
        if !tt.seek_err.is_empty() {
            if err == nil {
                t.Errorf(Sprintf!("%d. want seek error %q", i, tt.seek_err));
                continue;
            }
            if Sprintf!("%v", err) != tt.seek_err {
                t.Errorf(Sprintf!("%d. seek error = %q; want %q", i, err, tt.seek_err));
                continue;
            }
            continue;
        }
        if err != nil {
            t.Errorf(Sprintf!("%d. seek err = %s; want nil", i, err));
            continue;
        }
        if tt.wantpos != 0 && tt.wantpos != pos {
            t.Errorf(Sprintf!("%d. pos = %d, want %d", i, pos, tt.wantpos));
        }
        let mut buf = vec![0u8; tt.n];
        let (n, err) = r.Read(&mut buf);
        let got_eof = Sprintf!("%v", err) == "EOF";
        if tt.read_eof && !got_eof {
            t.Errorf(Sprintf!("%d. read err = %v; want EOF", i, err));
            continue;
        }
        if !tt.read_eof && err != nil {
            t.Errorf(Sprintf!("%d. read err = %s; want nil", i, err));
            continue;
        }
        let got = std::str::from_utf8(&buf[..n as usize]).unwrap();
        if got != tt.want {
            t.Errorf(Sprintf!("%d. got %q; want %q", i, got, tt.want));
        }
    }
}}

test!{ fn TestReadAfterBigSeek(t) {
    let mut r = strings::NewReader("0123456789");
    let (_, err) = r.Seek((1i64 << 31) + 5, SEEK_START);
    if err != nil { t.Fatal(Sprintf!("seek: %s", err)); }
    let mut buf = [0u8; 10];
    let (n, err) = r.Read(&mut buf);
    if n != 0 || Sprintf!("%v", err) != "EOF" {
        t.Errorf(Sprintf!("Read = %d, %v; want 0, EOF", n, err));
    }
}}

test!{ fn TestReaderAt(t) {
    let r = strings::NewReader("0123456789");
    struct AtCase { off: i64, n: usize, want: &'static str, want_err: &'static str }
    let cases: Vec<AtCase> = vec![
        AtCase { off: 0,  n: 10, want: "0123456789", want_err: "" },
        AtCase { off: 1,  n: 10, want: "123456789",  want_err: "EOF" },
        AtCase { off: 1,  n: 9,  want: "123456789",  want_err: "" },
        AtCase { off: 11, n: 10, want: "",           want_err: "EOF" },
        AtCase { off: 0,  n: 0,  want: "",           want_err: "" },
        AtCase { off: -1, n: 0,  want: "",           want_err: "strings.Reader.ReadAt: negative offset" },
    ];
    for (i, tt) in cases.iter().enumerate() {
        let mut b = vec![0u8; tt.n];
        let (rn, err) = r.ReadAt(&mut b, tt.off);
        let got = std::str::from_utf8(&b[..rn as usize]).unwrap();
        if got != tt.want {
            t.Errorf(Sprintf!("%d. got %q; want %q", i, got, tt.want));
        }
        let got_err = if err == nil { "".to_string() } else { format!("{}", err) };
        if got_err != tt.want_err {
            t.Errorf(Sprintf!("%d. err = %q; want %q", i, got_err, tt.want_err));
        }
    }
}}
