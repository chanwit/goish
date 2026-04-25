// Port of go1.25.5 src/text/tabwriter/tabwriter_test.go — core cases.
//
// Goish's tabwriter is simpler than Go's (no flags, no incremental flush
// to an underlying io.Writer; Flush returns a String). Tests cover the
// alignment / padding shape that actually matches.

#![allow(non_snake_case)]
use goish::prelude::*;
use goish::text::tabwriter;

fn run(input: &str, minwidth: i64, padding: i64, padchar: char) -> string {
    let mut tw = tabwriter::NewWriter(minwidth, 8, padding, padchar, 0);
    tw.WriteString(input);
    tw.Flush()
}

test!{ fn TestAlignTwoColumns(t) {
    let got = run("a\t1\nbb\t22\nccc\t333\n", 0, 1, ' ');
    let want = "a   1\nbb  22\nccc 333\n";
    if got != want {
        t.Errorf(Sprintf!("aligned = %q, want %q", got, want));
    }
}}

test!{ fn TestAlignThreeColumns(t) {
    let got = run("x\ty\tz\naa\tbb\tcc\naaaa\tbbbb\tcc\n", 0, 1, ' ');
    let want = "x    y    z\naa   bb   cc\naaaa bbbb cc\n";
    if got != want {
        t.Errorf(Sprintf!("3-col aligned = %q, want %q", got, want));
    }
}}

test!{ fn TestMinwidthPad(t) {
    let got = run("a\t1\n", 5, 0, '.');
    let want = "a....1\n";
    if got != want {
        t.Errorf(Sprintf!("minwidth = %q, want %q", got, want));
    }
}}

test!{ fn TestNoTrailingNewline(t) {
    let got = run("a\t1\nbb\t22", 0, 1, ' ');
    let want = "a  1\nbb 22";
    if got != want {
        t.Errorf(Sprintf!("no-newline = %q, want %q", got, want));
    }
}}

test!{ fn TestEmpty(t) {
    let mut tw = tabwriter::NewWriter(0, 8, 1, ' ', 0);
    if tw.Flush() != "" {
        t.Errorf(Sprintf!("empty Flush should return \"\""));
    }
}}
