// Port of go1.25.5/src/encoding/json/decode_test.go — Value-level subset.
//
// Go's decode tests rely overwhelmingly on reflection-based struct
// unmarshaling, which goish doesn't have. This port covers the subset
// that maps onto json::Value (equivalent to Go's `any`):
//   - TestEscape            — control/unicode escape sequences
//   - TestEmptyString       — empty JSON strings parse
//   - TestNullString        — null value
//   - TestUnmarshalSyntax   — bad JSON fails
//   - TestSkipArrayObjects  — deep structures parse
//   - TestNumberAccessors   — ints vs floats
//   - TestLargeByteSlice    — base64-ish not applicable; we do big-string
//   - Primitive round-trips, nested round-trips

#![allow(non_snake_case)]
use goish::prelude::*;
use goish::encoding::json::{self, Value};

test!{ fn TestUnmarshalPrimitives(t) {
    // null
    let mut v = Value::Null;
    let err = json::Unmarshal(b"null", &mut v);
    if err != nil || !v.IsNull() {
        t.Errorf(Sprintf!("Unmarshal(null): err=%s, IsNull=%v", err, v.IsNull()));
    }
    // true/false
    let mut v = Value::Null;
    json::Unmarshal(b"true", &mut v);
    if v.Bool() != true { t.Errorf(Sprintf!("true parse = %v", v.Bool())); }
    json::Unmarshal(b"false", &mut v);
    if v.Bool() != false { t.Errorf(Sprintf!("false parse = %v", v.Bool())); }
    // number
    json::Unmarshal(b"42", &mut v);
    if v.Int() != 42 { t.Errorf(Sprintf!("42 parse = %d", v.Int())); }
    json::Unmarshal(b"3.14", &mut v);
    if (v.Float() - 3.14).abs() > 1e-9 { t.Errorf(Sprintf!("3.14 parse = %f", v.Float())); }
    // string
    json::Unmarshal(b"\"hello\"", &mut v);
    if v.String() != "hello" { t.Errorf(Sprintf!("string parse = %q", v.String())); }
}}

test!{ fn TestEmptyString(t) {
    let mut v = Value::Null;
    let err = json::Unmarshal(b"\"\"", &mut v);
    if err != nil { t.Errorf(Sprintf!("empty string: %s", err)); }
    if !v.IsString() || v.String() != "" {
        t.Errorf(Sprintf!("empty string parsed wrong: %v", v.IsString()));
    }
}}

test!{ fn TestNullString(t) {
    let mut v = Value::String("existing".into());
    let err = json::Unmarshal(b"null", &mut v);
    if err != nil { t.Errorf(Sprintf!("null: %s", err)); }
    if !v.IsNull() {
        t.Errorf(Sprintf!("after null unmarshal, IsNull = false"));
    }
}}

struct EscT { r#in: &'static [u8], want: &'static str }

test!{ fn TestEscape(t) {
    let tests = vec![
        EscT { r#in: b"\"\\u0026\"",             want: "&" },
        EscT { r#in: b"\"\\u003c\"",             want: "<" },
        EscT { r#in: b"\"a\\tb\"",               want: "a\tb" },
        EscT { r#in: b"\"\\n\"",                 want: "\n" },
        EscT { r#in: b"\"\\\"quoted\\\"\"",      want: "\"quoted\"" },
        EscT { r#in: b"\"\\/slash\"",            want: "/slash" },
        EscT { r#in: b"\"\\u2028\"",             want: "\u{2028}" },
        EscT { r#in: b"\"hello \\ud83d\\ude00\"", want: "hello \u{1F600}" },
    ];
    for tt in &tests {
        let mut v = Value::Null;
        let err = json::Unmarshal(tt.r#in, &mut v);
        if err != nil {
            t.Errorf(Sprintf!("Unmarshal(%s) err: %s", String::from_utf8_lossy(tt.r#in), err));
            continue;
        }
        if v.String() != tt.want {
            t.Errorf(Sprintf!("escape: got %q want %q", v.String(), tt.want));
        }
    }
}}

test!{ fn TestUnmarshalSyntax(t) {
    let bad = vec![
        &b"{"[..], &b"}"[..], &b"["[..], &b"]"[..],
        &b"{\"a\":}"[..], &b"{1:2}"[..], &b"[1 2 3]"[..],
        &b"\"unterminated"[..], &b"not json"[..], &b""[..],
        &b"{\"a\":1,}"[..], &b"[1,2,]"[..],
    ];
    for b in bad {
        let mut v = Value::Null;
        let err = json::Unmarshal(b, &mut v);
        if err == nil {
            t.Errorf(Sprintf!("Unmarshal(%s) = nil, want error", String::from_utf8_lossy(b)));
        }
    }
}}

test!{ fn TestValid(t) {
    let good = vec![
        &b"null"[..], &b"true"[..], &b"false"[..], &b"42"[..],
        &b"3.14"[..], &b"\"x\""[..], &b"[1,2,3]"[..], &b"{\"a\":1}"[..],
        &b"{}"[..], &b"[]"[..], &b"{\"nested\":{\"k\":[1,2,3]}}"[..],
    ];
    for b in good {
        if !json::Valid(b) {
            t.Errorf(Sprintf!("Valid(%s) = false, want true", String::from_utf8_lossy(b)));
        }
    }
    let bad = vec![
        &b"not json"[..], &b"{"[..], &b"[1,2"[..], &b"nulll"[..],
    ];
    for b in bad {
        if json::Valid(b) {
            t.Errorf(Sprintf!("Valid(%s) = true, want false", String::from_utf8_lossy(b)));
        }
    }
}}

test!{ fn TestUnmarshalNestedArray(t) {
    let mut v = Value::Null;
    let err = json::Unmarshal(b"[[1,2],[3,4],[5]]", &mut v);
    if err != nil { t.Errorf(Sprintf!("err: %s", err)); return; }
    if !v.IsArray() || v.Len() != 3 {
        t.Errorf(Sprintf!("expected 3-element array, got %d", v.Len()));
        return;
    }
    if v.Index(0).Len() != 2 || v.Index(0).Index(0).Int() != 1 || v.Index(0).Index(1).Int() != 2 {
        t.Errorf(Sprintf!("inner 0 mismatch"));
    }
    if v.Index(2).Len() != 1 || v.Index(2).Index(0).Int() != 5 {
        t.Errorf(Sprintf!("inner 2 mismatch"));
    }
}}

test!{ fn TestUnmarshalNestedObject(t) {
    let input = "{\"outer\":{\"inner\":{\"k\":\"v\",\"n\":42},\"list\":[1,2,3]},\"other\":true}";
    let mut v = Value::Null;
    json::Unmarshal(input.as_bytes(), &mut v);
    if v.Get("outer").Get("inner").Get("k").String() != "v" {
        t.Errorf(Sprintf!("outer.inner.k mismatch"));
    }
    if v.Get("outer").Get("inner").Get("n").Int() != 42 {
        t.Errorf(Sprintf!("outer.inner.n mismatch"));
    }
    if v.Get("outer").Get("list").Len() != 3 {
        t.Errorf(Sprintf!("outer.list len mismatch"));
    }
    if v.Get("other").Bool() != true {
        t.Errorf(Sprintf!("other mismatch"));
    }
}}

test!{ fn TestUnmarshalRoundtrip(t) {
    let originals = vec![
        "null",
        "true",
        "false",
        "42",
        "-17",
        "0.5",
        "\"hello\"",
        "[1,2,3]",
        "{\"a\":1,\"b\":\"x\"}",
        "{\"nested\":[{\"k\":true},{\"k\":false}]}",
    ];
    for s in originals {
        let mut v = Value::Null;
        let err = json::Unmarshal(s.as_bytes(), &mut v);
        if err != nil { t.Errorf(Sprintf!("Unmarshal(%q): %s", s, err)); continue; }
        let (b, err) = json::Marshal(&v);
        if err != nil { t.Errorf(Sprintf!("Marshal after Unmarshal(%q): %s", s, err)); continue; }
        let got = String::from_utf8(b).unwrap();
        if got != s {
            t.Errorf(Sprintf!("roundtrip: %q → %q", s, got));
        }
    }
}}

test!{ fn TestNumberAccessors(t) {
    // Go's json.Number has Int64/Float64/String. We expose Int/Float/String on Value.
    let mut v = Value::Null;
    json::Unmarshal(b"1e2", &mut v);
    if v.Float() != 100.0 { t.Errorf(Sprintf!("1e2 → %f, want 100.0", v.Float())); }
    json::Unmarshal(b"-123", &mut v);
    if v.Int() != -123 { t.Errorf(Sprintf!("-123 → %d", v.Int())); }
}}

test!{ fn TestSkipArrayObjects(t) {
    // Parse deeply nested arrays/objects.
    let input = "[[{},[]],[[[{\"x\":1}]]]]";
    let mut v = Value::Null;
    let err = json::Unmarshal(input.as_bytes(), &mut v);
    if err != nil { t.Errorf(Sprintf!("err: %s", err)); }
    if !v.IsArray() || v.Len() != 2 {
        t.Errorf(Sprintf!("skip: top-level Len = %d", v.Len()));
    }
}}

test!{ fn TestLargeString(t) {
    let big = "a".repeat(10000);
    let input = format!("\"{}\"", big);
    let mut v = Value::Null;
    json::Unmarshal(input.as_bytes(), &mut v);
    if v.String().len() != 10000 {
        t.Errorf(Sprintf!("large string len = %d, want 10000", v.String().len()));
    }
}}

test!{ fn TestObjectGetAbsent(t) {
    let mut v = Value::Null;
    json::Unmarshal(b"{\"a\":1}", &mut v);
    // Looking up missing keys should return Null, not panic.
    let missing = v.Get("nope");
    if !missing.IsNull() {
        t.Errorf(Sprintf!("Get absent key should be Null"));
    }
    // Chained misses.
    let chained = v.Get("nope").Get("also").Index(0);
    if !chained.IsNull() {
        t.Errorf(Sprintf!("chained missing should be Null"));
    }
}}

test!{ fn TestUnmarshalMarshalRoundTrip(t) {
    // Build a complex Value, Marshal it, Unmarshal it back, compare.
    let original = Value::Object(vec![
        ("a".into(), Value::Number(1.0)),
        ("b".into(), Value::Array(vec![
            Value::String("x".into()),
            Value::Bool(true),
            Value::Null,
        ])),
        ("c".into(), Value::Object(vec![
            ("d".into(), Value::Number(-42.0)),
        ])),
    ]);
    let (bytes_, err) = json::Marshal(&original);
    if err != nil { t.Errorf(Sprintf!("Marshal: %s", err)); return; }
    let mut decoded = Value::Null;
    let err = json::Unmarshal(&bytes_, &mut decoded);
    if err != nil { t.Errorf(Sprintf!("Unmarshal: %s", err)); return; }
    if original != decoded {
        t.Errorf(Sprintf!("round-trip mismatch"));
    }
}}
