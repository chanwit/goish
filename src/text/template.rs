//! text/template: Go's template engine — focused subset.
//!
//!   Go                                  goish
//!   ─────────────────────────────────   ──────────────────────────────────
//!   t, err := template.New("n").Parse(src)
//!                                       let (mut t, err) = template::New("n").Parse(src);
//!   err := t.Execute(&buf, data)        let err = t.Execute(&mut buf, &data);
//!
//! Scope (v0.14): the core actions goish users actually need for output
//! templating.
//!
//!   {{.}}              → current scope value
//!   {{.Field}}         → map/struct field lookup (JSON-like paths)
//!   {{.a.b.c}}         → dotted lookup chains
//!   {{if .x}}...{{end}}/ {{else}}
//!   {{range .list}}...{{end}}
//!   {{define "name"}}...{{end}}
//!   {{template "name" .}}
//!   {{/* comment */}}  → stripped
//!
//! Not supported: user-defined funcs (`FuncMap`), pipelines (`{{.x | y}}`),
//! `with`, `block`, variables (`$x := ...`), comparison operators,
//! numeric/string literals. These are tracked on a follow-up issue.
//!
//! Data values are `serde_json::Value` — gives JSON-shaped structures
//! with field lookup, boolean truthiness, and range-iterability without
//! adding reflection.

use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone)]
enum Node {
    Text(String),
    Field(Vec<String>),          // {{.a.b.c}} → ["a","b","c"]; empty = {{.}}
    If(Vec<String>, Vec<Node>, Vec<Node>),  // cond, then, else
    Range(Vec<String>, Vec<Node>),
    Template(String),            // {{template "name" .}}
}

pub struct Template {
    name: String,
    root: Vec<Node>,
    defines: HashMap<String, Vec<Node>>,
}

#[allow(non_snake_case)]
pub fn New(name: impl Into<String>) -> Template {
    Template { name: name.into(), root: Vec::new(), defines: HashMap::new() }
}

impl Template {
    /// `t.Parse(src)` — parse src into this template. Returns the same
    /// template + error so the Go-idiomatic chain works.
    pub fn Parse(mut self, src: impl AsRef<str>) -> (Self, crate::errors::error) {
        let src = src.as_ref();
        match parse(src) {
            Ok((root, defines)) => {
                self.root = root;
                for (k, v) in defines { self.defines.insert(k, v); }
                (self, crate::errors::nil)
            }
            Err(e) => (self, crate::errors::New(&e)),
        }
    }

    pub fn Name(&self) -> &str { &self.name }

    /// `t.Execute(w, data)` — render the template against `data`,
    /// writing the rendered output to `w`. Mirrors Go's
    /// `(*Template).Execute(io.Writer, any) error`. Goish accepts any
    /// `std::io::Write`, so `bytes::Buffer`, `os::Stdout`, and
    /// `Vec<u8>` all work directly.
    pub fn Execute<W: std::io::Write>(&self, out: &mut W, data: &Value) -> crate::errors::error {
        let mut buf = String::new();
        let e = render(&self.root, &mut buf, data, self);
        if e != crate::errors::nil { return e; }
        match out.write_all(buf.as_bytes()) {
            Ok(()) => crate::errors::nil,
            Err(e) => crate::errors::New(&format!("template: write: {}", e)),
        }
    }

    pub fn ExecuteTemplate<W: std::io::Write>(&self, out: &mut W, name: &str, data: &Value) -> crate::errors::error {
        let mut buf = String::new();
        let e = match self.defines.get(name) {
            Some(nodes) => render(nodes, &mut buf, data, self),
            None => return crate::errors::New(&format!("template: no template {:?} associated with template {:?}", name, self.name)),
        };
        if e != crate::errors::nil { return e; }
        match out.write_all(buf.as_bytes()) {
            Ok(()) => crate::errors::nil,
            Err(e) => crate::errors::New(&format!("template: write: {}", e)),
        }
    }
}

// ── Parser ─────────────────────────────────────────────────────────────

fn parse(src: &str) -> Result<(Vec<Node>, HashMap<String, Vec<Node>>), String> {
    let tokens = tokenize(src)?;
    let mut p = Parser { toks: tokens, pos: 0, defines: HashMap::new() };
    let root = p.parse_nodes(&[])?;
    Ok((root, p.defines))
}

#[derive(Debug, Clone)]
enum Token {
    Text(String),
    Action(String),  // inner content of {{ ... }}
}

fn tokenize(src: &str) -> Result<Vec<Token>, String> {
    let mut out = Vec::new();
    let mut i = 0;
    let bytes = src.as_bytes();
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'{' && bytes[i + 1] == b'{' {
            // find matching }}
            if let Some(end) = src[i + 2..].find("}}") {
                let inner = src[i + 2..i + 2 + end].trim();
                i = i + 2 + end + 2;
                if inner.starts_with("/*") && inner.ends_with("*/") {
                    continue;
                }
                out.push(Token::Action(inner.to_string()));
                continue;
            } else {
                return Err("unclosed action".to_string());
            }
        }
        // Text: consume until next {{
        let start = i;
        while i < bytes.len() && !(i + 1 < bytes.len() && bytes[i] == b'{' && bytes[i + 1] == b'{') {
            i += 1;
        }
        if i > start {
            out.push(Token::Text(src[start..i].to_string()));
        }
    }
    Ok(out)
}

struct Parser {
    toks: Vec<Token>,
    pos: usize,
    defines: HashMap<String, Vec<Node>>,
}

impl Parser {
    /// Parse nodes until we hit one of the stop keywords (e.g. "end", "else").
    fn parse_nodes(&mut self, stop: &[&str]) -> Result<Vec<Node>, String> {
        let mut out = Vec::new();
        while self.pos < self.toks.len() {
            match &self.toks[self.pos] {
                Token::Text(s) => {
                    out.push(Node::Text(s.clone()));
                    self.pos += 1;
                }
                Token::Action(a) => {
                    let kw = a.split_whitespace().next().unwrap_or("");
                    if stop.contains(&kw) { return Ok(out); }
                    let action = a.clone();
                    self.pos += 1;
                    if let Some(node) = self.parse_action(&action)? {
                        out.push(node);
                    }
                }
            }
        }
        if !stop.is_empty() {
            return Err(format!("unexpected end; looking for {:?}", stop));
        }
        Ok(out)
    }

    fn parse_action(&mut self, a: &str) -> Result<Option<Node>, String> {
        let a = a.trim();
        // {{end}} and {{else}} are handled by callers — here they shouldn't reach.
        if a == "end" || a == "else" {
            return Err(format!("unexpected {{{{{}}}}}", a));
        }
        // {{if ...}}
        if let Some(rest) = a.strip_prefix("if ") {
            let path = parse_path(rest.trim())?;
            let then = self.parse_nodes(&["end", "else"])?;
            let mut else_ = Vec::new();
            if self.pos < self.toks.len() {
                if let Token::Action(kw) = &self.toks[self.pos] {
                    if kw.trim() == "else" {
                        self.pos += 1;
                        else_ = self.parse_nodes(&["end"])?;
                    }
                }
            }
            self.expect_action("end")?;
            return Ok(Some(Node::If(path, then, else_)));
        }
        // {{range ...}}
        if let Some(rest) = a.strip_prefix("range ") {
            let path = parse_path(rest.trim())?;
            let body = self.parse_nodes(&["end"])?;
            self.expect_action("end")?;
            return Ok(Some(Node::Range(path, body)));
        }
        // {{define "name"}}
        if let Some(rest) = a.strip_prefix("define ") {
            let name = parse_string_lit(rest.trim())?;
            let body = self.parse_nodes(&["end"])?;
            self.expect_action("end")?;
            self.defines.insert(name, body);
            return Ok(None);  // define produces no output at its position
        }
        // {{template "name" .x}}
        if let Some(rest) = a.strip_prefix("template ") {
            let rest = rest.trim();
            let name = parse_string_lit(rest)?;
            // argument (optional) after the string literal — accept
            // trailing `. `/path but we currently ignore it (always pass
            // current scope). Improvements tracked in the v0.14 follow-up.
            return Ok(Some(Node::Template(name)));
        }
        // Field: `.`, `.a`, `.a.b.c`
        let path = parse_path(a)?;
        Ok(Some(Node::Field(path)))
    }

    fn expect_action(&mut self, kw: &str) -> Result<(), String> {
        if self.pos >= self.toks.len() {
            return Err(format!("expected {{{{{}}}}} at end of input", kw));
        }
        match &self.toks[self.pos] {
            Token::Action(a) if a.trim() == kw => { self.pos += 1; Ok(()) }
            other => Err(format!("expected {{{{{}}}}}, got {:?}", kw, other)),
        }
    }
}

fn parse_path(s: &str) -> Result<Vec<String>, String> {
    let s = s.trim();
    if s == "." { return Ok(vec![]); }
    if !s.starts_with('.') {
        return Err(format!("expected path starting with '.', got {:?}", s));
    }
    Ok(s[1..].split('.').map(|p| p.to_string()).collect())
}

fn parse_string_lit(s: &str) -> Result<String, String> {
    let s = s.trim();
    // Take leading quoted string.
    if !s.starts_with('"') {
        return Err(format!("expected \"name\", got {:?}", s));
    }
    let end = s[1..].find('"').ok_or("unterminated string literal")?;
    Ok(s[1..1 + end].to_string())
}

// ── Renderer ───────────────────────────────────────────────────────────

fn render(nodes: &[Node], out: &mut String, scope: &Value, tmpl: &Template) -> crate::errors::error {
    for node in nodes {
        match node {
            Node::Text(s) => out.push_str(s),
            Node::Field(path) => {
                let v = resolve(scope, path);
                write_value(out, &v);
            }
            Node::If(path, then, else_) => {
                let v = resolve(scope, path);
                if truthy(&v) {
                    let e = render(then, out, scope, tmpl);
                    if e != crate::errors::nil { return e; }
                } else {
                    let e = render(else_, out, scope, tmpl);
                    if e != crate::errors::nil { return e; }
                }
            }
            Node::Range(path, body) => {
                let v = resolve(scope, path);
                match &v {
                    Value::Array(items) => {
                        for item in items {
                            let e = render(body, out, item, tmpl);
                            if e != crate::errors::nil { return e; }
                        }
                    }
                    Value::Object(map) => {
                        for (_k, val) in map {
                            let e = render(body, out, val, tmpl);
                            if e != crate::errors::nil { return e; }
                        }
                    }
                    Value::Null => { /* no iterations */ }
                    _ => return crate::errors::New(&format!("range over non-collection: {}", v)),
                }
            }
            Node::Template(name) => {
                match tmpl.defines.get(name) {
                    Some(body) => {
                        let e = render(body, out, scope, tmpl);
                        if e != crate::errors::nil { return e; }
                    }
                    None => return crate::errors::New(&format!("template: no template {:?} associated", name)),
                }
            }
        }
    }
    crate::errors::nil
}

fn resolve(v: &Value, path: &[String]) -> Value {
    let mut cur = v.clone();
    for p in path {
        cur = match cur {
            Value::Object(map) => map.get(p).cloned().unwrap_or(Value::Null),
            _ => Value::Null,
        };
    }
    cur
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

fn write_value(out: &mut String, v: &Value) {
    match v {
        Value::Null => {}
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&n.to_string()),
        Value::String(s) => out.push_str(s),
        Value::Array(_) | Value::Object(_) => out.push_str(&v.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn exec(src: &str, data: &Value) -> String {
        let (t, err) = New("t").Parse(src);
        assert!(err == crate::errors::nil, "parse error: {}", err);
        let mut out: Vec<u8> = Vec::new();
        let e = t.Execute(&mut out, data);
        assert!(e == crate::errors::nil, "exec error: {}", e);
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn plain_text() {
        assert_eq!(exec("hello", &json!({})), "hello");
    }

    #[test]
    fn field_lookup() {
        assert_eq!(exec("Hi {{.name}}!", &json!({"name": "alice"})), "Hi alice!");
    }

    #[test]
    fn nested_field() {
        assert_eq!(exec("{{.user.name}}", &json!({"user": {"name": "bob"}})), "bob");
    }

    #[test]
    fn if_truthy() {
        assert_eq!(exec("{{if .x}}Y{{else}}N{{end}}", &json!({"x": true})), "Y");
        assert_eq!(exec("{{if .x}}Y{{else}}N{{end}}", &json!({"x": false})), "N");
        assert_eq!(exec("{{if .x}}Y{{else}}N{{end}}", &json!({})), "N");
    }

    #[test]
    fn range_over_array() {
        assert_eq!(exec("{{range .}}[{{.}}]{{end}}", &json!([1, 2, 3])), "[1][2][3]");
    }

    #[test]
    fn define_and_template() {
        let src = r#"{{define "item"}}<{{.}}>{{end}}{{range .}}{{template "item"}}{{end}}"#;
        assert_eq!(exec(src, &json!(["a", "b"])), "<a><b>");
    }

    #[test]
    fn comment_stripped() {
        assert_eq!(exec("A{{/* skip */}}B", &json!({})), "AB");
    }
}
