// Port of go1.25.5 src/iter/iter_test.go and iter.Pull semantics —
// covers the core Seq / Seq2 shapes that goish exposes.

#![allow(non_snake_case)]
use goish::prelude::*;
use goish::iter::{Seq, Seq2};

test!{ fn TestFromIteratorCollect(t) {
    let seq = iter::FromIterator(vec![10i64, 20, 30, 40]);
    let out = iter::Collect(seq);
    if out != vec![10i64, 20, 30, 40] {
        t.Errorf(Sprintf!("FromIterator→Collect mismatch"));
    }
}}

test!{ fn TestSeqEarlyStop(t) {
    // An iter.Seq that yields 1,2,3,... until yield returns false.
    let mut counter = 0i64;
    let mut seq = move |yield_: &mut dyn FnMut(i64) -> bool| {
        loop {
            counter += 1;
            if !yield_(counter) { return; }
        }
    };
    let mut collected: slice<int> = slice::new();
    seq.for_each(|v| {
        collected.push(v);
        v < 3
    });
    if collected != vec![1i64, 2, 3] {
        t.Errorf(Sprintf!("early-stop mismatch: %d items", collected.len() as i64));
    }
}}

test!{ fn TestSeq2Pairs(t) {
    let mut seq = |yield_: &mut dyn FnMut(i64, i64) -> bool| {
        for (k, v) in &[(1i64, 10i64), (2, 20), (3, 30)] {
            if !yield_(*k, *v) { return; }
        }
    };
    let mut keys: slice<int> = slice::new();
    let mut vals: slice<int> = slice::new();
    seq.for_each(|k, v| { keys.push(k); vals.push(v); true });
    if keys != vec![1i64, 2, 3] { t.Errorf(Sprintf!("Seq2 keys mismatch")); }
    if vals != vec![10i64, 20, 30] { t.Errorf(Sprintf!("Seq2 vals mismatch")); }
}}

test!{ fn TestSeqEmpty(t) {
    let seq = iter::FromIterator::<slice<int>>(slice::new());
    let out = iter::Collect(seq);
    if !out.is_empty() { t.Errorf(Sprintf!("empty Seq collected %d items", out.len() as i64)); }
}}

test!{ fn TestCollect2(t) {
    let seq = |yield_: &mut dyn FnMut(&'static str, i64) -> bool| {
        for (k, v) in &[("a", 1i64), ("b", 2), ("c", 3)] {
            if !yield_(*k, *v) { return; }
        }
    };
    let out = iter::Collect2(seq);
    if out != vec![("a", 1i64), ("b", 2), ("c", 3)] {
        t.Errorf(Sprintf!("Collect2 mismatch"));
    }
}}
