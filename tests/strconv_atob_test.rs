// Direct port of go1.25.5/src/strconv/atob_test.go.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   ParseBool(s) → (bool, error)         strconv::ParseBool(s)
//   FormatBool(b) → string               strconv::FormatBool(b)
//   AppendBool(dst, b) → []byte          strconv::AppendBool(dst, b)
//   ErrSyntax sentinel                   strconv::ErrSyntax()

#![allow(non_snake_case)]
use goish::prelude::*;

struct AtobTest {
    #[allow(dead_code)] // fields are read by the runner
    In: &'static str,
    Out: bool,
    Err: Option<error>, // None == nil, Some(...) == expected sentinel
}

fn atobtests() -> Vec<AtobTest> {
    vec![
        AtobTest { In: "",      Out: false, Err: Some(strconv::ErrSyntax()) },
        AtobTest { In: "asdf",  Out: false, Err: Some(strconv::ErrSyntax()) },
        AtobTest { In: "0",     Out: false, Err: None },
        AtobTest { In: "f",     Out: false, Err: None },
        AtobTest { In: "F",     Out: false, Err: None },
        AtobTest { In: "FALSE", Out: false, Err: None },
        AtobTest { In: "false", Out: false, Err: None },
        AtobTest { In: "False", Out: false, Err: None },
        AtobTest { In: "1",     Out: true,  Err: None },
        AtobTest { In: "t",     Out: true,  Err: None },
        AtobTest { In: "T",     Out: true,  Err: None },
        AtobTest { In: "TRUE",  Out: true,  Err: None },
        AtobTest { In: "true",  Out: true,  Err: None },
        AtobTest { In: "True",  Out: true,  Err: None },
    ]
}

test!{ fn TestParseBool(t) {
    for test in atobtests() {
        let (b, e) = strconv::ParseBool(test.In);
        if let Some(want) = test.Err {
            // expect an error
            if e == nil {
                t.Errorf(Sprintf!("ParseBool(%s) = nil; want %s", test.In, want));
            } else {
                // Go checks e.(*NumError).Err == test.err. We check the
                // error message contains the sentinel's text.
                let want_msg = format!("{}", want);
                let got_msg = format!("{}", e);
                if !got_msg.contains(&want_msg) {
                    t.Errorf(Sprintf!("ParseBool(%s) = %s; want %s", test.In, e, want));
                }
            }
        } else {
            if e != nil {
                t.Errorf(Sprintf!("ParseBool(%s) = %s; want nil", test.In, e));
            }
            if b != test.Out {
                t.Errorf(Sprintf!("ParseBool(%s) = %v; want %v", test.In, b, test.Out));
            }
        }
    }
}}

test!{ fn TestFormatBool(t) {
    // Go: for b, s := range map[bool]string{true:"true", false:"false"}
    let cases = [(true, "true"), (false, "false")];
    for (b, s) in cases {
        let f = strconv::FormatBool(b);
        if f != s {
            t.Errorf(Sprintf!("FormatBool(%v) = %q; want %q", b, f, s));
        }
    }
}}

struct AppendBoolTest {
    B: bool,
    In: &'static [u8],
    Out: &'static [u8],
}

fn appendBoolTests() -> Vec<AppendBoolTest> {
    vec![
        AppendBoolTest { B: true,  In: b"foo ", Out: b"foo true"  },
        AppendBoolTest { B: false, In: b"foo ", Out: b"foo false" },
    ]
}

test!{ fn TestAppendBool(t) {
    for test in appendBoolTests() {
        let b = strconv::AppendBool(test.In.to_vec(), test.B);
        if !bytes::Equal(&b, test.Out) {
            t.Errorf(Sprintf!("AppendBool(%q, %v) = %q; want %q",
                std::str::from_utf8(test.In).unwrap(), test.B,
                std::str::from_utf8(&b).unwrap(),
                std::str::from_utf8(test.Out).unwrap()));
        }
    }
}}
