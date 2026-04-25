// json: Go's encoding/json — Marshal / Unmarshal / Value.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   data, err := json.Marshal(v)        let (data, err) = json::Marshal(&v);
//   err := json.Unmarshal(data, &v)     let err = json::Unmarshal(&data, &mut v);
//   var m map[string]interface{}        let mut v = json::Value::Null;
//   json.Unmarshal(d, &m)               json::Unmarshal(&d, &mut v);
//   s := v["name"].(string)             let s = v.Get("name").String();
//
// Go's type-directed marshalling (based on struct tags) doesn't map cleanly
// to Rust without code generation, so goish's json is value-centered — it
// maps directly to Go's `map[string]interface{}` style, which is idiomatic
// Go for dynamic JSON and covers the 90% case.
//
// For strongly typed structs, you can still write your own (T → Value)
// conversion and then Marshal the Value.

use crate::errors::{error, nil, New};
use crate::types::{byte, int, slice, string};

pub use serde_json::Value as RawValue;

/// json.RawMessage — a byte slice representing raw JSON, used to defer
/// parsing of a sub-tree.
pub type RawMessage = crate::types::slice<byte>;

/// json.Value — a JSON value. Equivalent to Go's `interface{}` after an
/// Unmarshal into a non-typed destination.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(string),
    Array(Vec<Value>),
    Object(Vec<(string, Value)>),
}

impl Default for Value {
    fn default() -> Self { Value::Null }
}

impl Value {
    pub fn IsNull(&self) -> bool { matches!(self, Value::Null) }
    pub fn IsBool(&self) -> bool { matches!(self, Value::Bool(_)) }
    pub fn IsNumber(&self) -> bool { matches!(self, Value::Number(_)) }
    pub fn IsString(&self) -> bool { matches!(self, Value::String(_)) }
    pub fn IsArray(&self) -> bool { matches!(self, Value::Array(_)) }
    pub fn IsObject(&self) -> bool { matches!(self, Value::Object(_)) }

    /// Value.String() — returns the string inside or "" if not a string.
    pub fn String(&self) -> string {
        match self {
            Value::String(s) => s.clone(),
            _ => "".into(),
        }
    }

    /// Value.Int() — returns the number as int64 (0 if not a number).
    pub fn Int(&self) -> crate::types::int64 {
        match self {
            Value::Number(n) => *n as i64,
            _ => 0,
        }
    }

    pub fn Float(&self) -> f64 {
        match self {
            Value::Number(n) => *n,
            _ => 0.0,
        }
    }

    pub fn Bool(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            _ => false,
        }
    }

    /// Value.Get(k) — looks up a key in an object, returns Null if absent.
    pub fn Get(&self, key: impl AsRef<str>) -> Value {
        if let Value::Object(m) = self {
            for (k, v) in m {
                if k == key.as_ref() { return v.clone(); }
            }
        }
        Value::Null
    }

    pub fn Index(&self, i: int) -> Value {
        if let Value::Array(a) = self {
            if i >= 0 && (i as usize) < a.len() { return a[i as usize].clone(); }
        }
        Value::Null
    }

    /// Value.Len() — length for array/object/string, else 0.
    pub fn Len(&self) -> int {
        match self {
            Value::Array(a) => a.len() as int,
            Value::Object(o) => o.len() as int,
            Value::String(s) => s.len() as int,
            _ => 0,
        }
    }

    /// Set a key on an object (inserts if missing). No-op for non-objects.
    pub fn Set(&mut self, key: impl AsRef<str>, value: Value) {
        if let Value::Object(m) = self {
            for (k, v) in m.iter_mut() {
                if k == key.as_ref() { *v = value; return; }
            }
            m.push((key.as_ref().into(), value));
        }
    }
}

// ── Conversion to/from serde_json::Value ─────────────────────────────

fn to_raw(v: &Value) -> RawValue {
    match v {
        Value::Null => RawValue::Null,
        Value::Bool(b) => RawValue::Bool(*b),
        Value::Number(n) => {
            // If the number has no fractional part and fits in i64, emit it as
            // an integer — matches Go's `json.Marshal(int(5))` which writes "5"
            // not "5.0".
            if n.is_finite() && n.fract() == 0.0 && *n >= i64::MIN as f64 && *n <= i64::MAX as f64 {
                RawValue::Number(serde_json::Number::from(*n as i64))
            } else {
                serde_json::Number::from_f64(*n)
                    .map(RawValue::Number)
                    .unwrap_or(RawValue::Null)
            }
        }
        Value::String(s) => RawValue::String(s.as_str().into()),
        Value::Array(a) => RawValue::Array(a.iter().map(to_raw).collect()),
        Value::Object(m) => {
            let mut map = serde_json::Map::new();
            for (k, v) in m {
                map.insert(k.as_str().into(), to_raw(v));
            }
            RawValue::Object(map)
        }
    }
}

fn from_raw(v: &RawValue) -> Value {
    match v {
        RawValue::Null => Value::Null,
        RawValue::Bool(b) => Value::Bool(*b),
        RawValue::Number(n) => Value::Number(n.as_f64().unwrap_or(0.0)),
        RawValue::String(s) => Value::String(s.as_str().into()),
        RawValue::Array(a) => Value::Array(a.iter().map(from_raw).collect()),
        RawValue::Object(m) => Value::Object(
            m.iter().map(|(k, v)| (string::from(k.as_str()), from_raw(v))).collect()
        ),
    }
}

// ── Marshal / Unmarshal ──────────────────────────────────────────────

#[allow(non_snake_case)]
pub fn Marshal(v: &Value) -> (crate::types::slice<byte>, error) {
    match serde_json::to_vec(&to_raw(v)) {
        Ok(b) => (b.into(), nil),
        Err(e) => (crate::types::slice::new(), New(&e.to_string())),
    }
}

#[allow(non_snake_case)]
pub fn MarshalIndent(v: &Value, _prefix: impl AsRef<str>, indent: impl AsRef<str>) -> (crate::types::slice<byte>, error) {
    // serde_json doesn't directly expose prefix support; emit with to_string_pretty
    // using the given indent width (spaces only; tab if indent=="\t").
    let indent = indent.as_ref();
    let mut buf: Vec<byte> = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(indent.as_bytes());
    let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
    use serde::Serialize;
    match to_raw(v).serialize(&mut ser) {
        Ok(()) => (buf.into(), nil),
        Err(e) => (crate::types::slice::new(), New(&e.to_string())),
    }
}

#[allow(non_snake_case)]
pub fn Unmarshal(data: impl AsRef<[byte]>, v: &mut Value) -> error {
    match serde_json::from_slice::<RawValue>(data.as_ref()) {
        Ok(raw) => {
            *v = from_raw(&raw);
            nil
        }
        Err(e) => New(&e.to_string()),
    }
}

/// json.Valid(data) — returns true if data is valid JSON.
#[allow(non_snake_case)]
pub fn Valid(data: impl AsRef<[byte]>) -> bool {
    serde_json::from_slice::<RawValue>(data.as_ref()).is_ok()
}

// ── HTMLEscape / Compact / Indent ────────────────────────────────────

/// HTMLEscape writes to dst the JSON-encoded src with <, >, &, U+2028, U+2029
/// characters replaced by their Unicode-escape equivalents inside JSON strings.
#[allow(non_snake_case)]
pub fn HTMLEscape(dst: &mut crate::bytes::Buffer, src: impl AsRef<[byte]>) {
    let src = src.as_ref();
    let n = src.len();
    let mut i = 0;
    let mut buf: Vec<byte> = Vec::with_capacity(n);
    while i < n {
        let c = src[i];
        if c == b'<' || c == b'>' || c == b'&' {
            buf.extend_from_slice(&[b'\\', b'u', b'0', b'0']);
            const HEX: &[u8] = b"0123456789abcdef";
            buf.push(HEX[(c >> 4) as usize]);
            buf.push(HEX[(c & 0xf) as usize]);
            i += 1;
            continue;
        }
        // U+2028 = E2 80 A8, U+2029 = E2 80 A9 — emit \u2028 / \u2029.
        if i + 2 < n && src[i] == 0xe2 && src[i+1] == 0x80 && (src[i+2] == 0xa8 || src[i+2] == 0xa9) {
            buf.extend_from_slice(b"\\u202");
            buf.push(if src[i+2] == 0xa8 { b'8' } else { b'9' });
            i += 3;
            continue;
        }
        buf.push(c);
        i += 1;
    }
    let _ = dst.Write(&buf);
}

/// Compact appends to dst the JSON-encoded src with insignificant whitespace elided.
#[allow(non_snake_case)]
pub fn Compact(dst: &mut crate::bytes::Buffer, src: impl AsRef<[byte]>) -> error {
    match serde_json::from_slice::<RawValue>(src.as_ref()) {
        Ok(raw) => match serde_json::to_vec(&raw) {
            Ok(bs) => { let _ = dst.Write(&bs); nil }
            Err(e) => New(&e.to_string()),
        },
        Err(e) => New(&e.to_string()),
    }
}

/// Indent appends to dst a pretty-printed JSON value read from src, using
/// `prefix` and `indent` for each newline-initiated line.
#[allow(non_snake_case)]
pub fn Indent(dst: &mut crate::bytes::Buffer, src: impl AsRef<[byte]>, _prefix: impl AsRef<str>, indent: impl AsRef<str>) -> error {
    let indent_bytes = indent.as_ref().as_bytes().to_vec();
    match serde_json::from_slice::<RawValue>(src.as_ref()) {
        Ok(raw) => {
            let formatter = serde_json::ser::PrettyFormatter::with_indent(&indent_bytes);
            let mut ser = serde_json::Serializer::with_formatter(Vec::<u8>::new(), formatter);
            use serde::Serialize;
            match raw.serialize(&mut ser) {
                Ok(()) => { let _ = dst.Write(&ser.into_inner()); nil }
                Err(e) => New(&e.to_string()),
            }
        }
        Err(e) => New(&e.to_string()),
    }
}

// ── Convenience builders ─────────────────────────────────────────────

/// Build a JSON object Value from (key, value) pairs.
#[allow(non_snake_case)]
pub fn Object(pairs: slice<(string, Value)>) -> Value {
    Value::Object(pairs.into_vec())
}

/// Build a JSON array Value.
#[allow(non_snake_case)]
pub fn Array(values: slice<Value>) -> Value {
    Value::Array(values.into_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marshal_simple_object() {
        let mut v = Value::Object(Vec::new());
        v.Set("name", Value::String("alice".into()));
        v.Set("age", Value::Number(30.0));
        let (data, err) = Marshal(&v);
        assert_eq!(err, nil);
        let s = crate::bytes::String(&data);
        assert!(s.contains("\"name\":\"alice\""));
        assert!(s.contains("\"age\":30"));
    }

    #[test]
    fn unmarshal_simple_object() {
        let s = r#"{"name":"alice","age":30,"active":true,"tags":["a","b"]}"#;
        let mut v = Value::Null;
        let err = Unmarshal(s.as_bytes(), &mut v);
        assert_eq!(err, nil);
        assert_eq!(v.Get("name").String(), "alice");
        assert_eq!(v.Get("age").Int(), 30);
        assert_eq!(v.Get("active").Bool(), true);
        assert_eq!(v.Get("tags").Len(), 2);
        assert_eq!(v.Get("tags").Index(0).String(), "a");
    }

    #[test]
    fn unmarshal_invalid_returns_error() {
        let mut v = Value::Null;
        let err = Unmarshal(b"{broken}", &mut v);
        assert!(err != nil);
    }

    #[test]
    fn marshal_indent_formats() {
        let mut v = Value::Object(Vec::new());
        v.Set("x", Value::Number(1.0));
        let (data, err) = MarshalIndent(&v, "", "  ");
        assert_eq!(err, nil);
        let s = crate::bytes::String(&data);
        assert!(s.contains('\n'));
        assert!(s.contains("  \"x\""));
    }

    #[test]
    fn valid_detects_good_and_bad() {
        assert!(Valid(br#"{"a":1}"#));
        assert!(!Valid(b"not json"));
    }

    #[test]
    fn round_trip_nested() {
        let original = Value::Object(vec![
            ("outer".into(), Value::Object(vec![
                ("inner".into(), Value::Array(vec![
                    Value::Number(1.0), Value::Number(2.0), Value::Number(3.0)
                ])),
            ])),
        ]);
        let (data, _) = Marshal(&original);
        let mut decoded = Value::Null;
        Unmarshal(&data, &mut decoded);
        assert_eq!(decoded.Get("outer").Get("inner").Len(), 3);
        assert_eq!(decoded.Get("outer").Get("inner").Index(2).Int(), 3);
    }

    #[test]
    fn null_defaults() {
        let v: Value = Value::Null;
        assert_eq!(v.String(), "");
        assert_eq!(v.Int(), 0);
        assert_eq!(v.Bool(), false);
    }
}
