// Port of go1.25.5 src/text/template/exec_test.go patterns — focused
// on the subset goish implements (field lookup, if/else, range, define/
// template, comments). Full Go template language features (pipelines,
// funcs, with/block, variables) are deferred.

#![allow(non_snake_case)]
use goish::prelude::*;
use goish::text::template;
use serde_json::json;

fn exec(src: &str, data: &serde_json::Value) -> string {
    let (t, err) = template::New("t").Parse(src);
    if err != nil { panic!("parse error: {}", err); }
    let mut out = bytes::Buffer::new();
    let e = t.Execute(&mut out, data);
    if e != nil { panic!("exec error: {}", e); }
    out.String()
}

test!{ fn TestPlainText(t) {
    if exec("hello world", &json!({})) != "hello world" {
        t.Errorf(Sprintf!("plain text"));
    }
}}

test!{ fn TestFieldLookup(t) {
    if exec("Hi {{.name}}!", &json!({"name": "alice"})) != "Hi alice!" {
        t.Errorf(Sprintf!("field lookup"));
    }
}}

test!{ fn TestNestedField(t) {
    if exec("{{.user.name}} ({{.user.age}})",
            &json!({"user": {"name": "bob", "age": 42}})) != "bob (42)" {
        t.Errorf(Sprintf!("nested field"));
    }
}}

test!{ fn TestIf(t) {
    let src = "{{if .ok}}YES{{else}}NO{{end}}";
    if exec(src, &json!({"ok": true})) != "YES" { t.Errorf(Sprintf!("if true")); }
    if exec(src, &json!({"ok": false})) != "NO" { t.Errorf(Sprintf!("if false")); }
    if exec(src, &json!({})) != "NO" { t.Errorf(Sprintf!("if missing")); }
}}

test!{ fn TestRangeArray(t) {
    let src = "{{range .}}[{{.}}]{{end}}";
    if exec(src, &json!([1, 2, 3])) != "[1][2][3]" {
        t.Errorf(Sprintf!("range array"));
    }
}}

test!{ fn TestRangeEmpty(t) {
    // Go: range over empty slice produces no output.
    if exec("{{range .}}X{{end}}", &json!([])) != "" {
        t.Errorf(Sprintf!("range empty"));
    }
}}

test!{ fn TestDefineAndTemplate(t) {
    let src = r#"{{define "row"}}<{{.}}>{{end}}{{range .}}{{template "row"}}{{end}}"#;
    if exec(src, &json!(["a", "b", "c"])) != "<a><b><c>" {
        t.Errorf(Sprintf!("define+template"));
    }
}}

test!{ fn TestCommentStripped(t) {
    if exec("A{{/* ignored */}}B", &json!({})) != "AB" {
        t.Errorf(Sprintf!("comment"));
    }
}}

test!{ fn TestDotSelf(t) {
    if exec("value={{.}}", &json!("hello")) != "value=hello" {
        t.Errorf(Sprintf!("dot self"));
    }
}}

test!{ fn TestMissingFieldRendersEmpty(t) {
    // Go: {{.missing}} on an object without that field renders <no value>
    // or "", depending on settings. Goish renders empty (matching Null
    // value behavior).
    if exec("|{{.missing}}|", &json!({})) != "||" {
        t.Errorf(Sprintf!("missing field"));
    }
}}
