// Port of go1.25.5 src/maps/maps_test.go — Keys, Values, Equal, Clone,
// Copy, DeleteFunc.

#![allow(non_snake_case)]
use goish::prelude::*;
use goish::_map::map;

test!{ fn TestKeys(t) {
    let mut m: map<i64, String> = map::new();
    m.insert(1, "a".into());
    m.insert(2, "b".into());
    m.insert(3, "c".into());
    let mut ks = maps::Keys(&m);
    ks.sort();
    if ks != vec![1i64, 2, 3] {
        t.Errorf(Sprintf!("Keys mismatch: got %d items", ks.len() as i64));
    }
}}

test!{ fn TestValues(t) {
    let mut m: map<&str, i64> = map::new();
    m.insert("x", 10);
    m.insert("y", 20);
    m.insert("z", 30);
    let mut vs = maps::Values(&m);
    vs.sort();
    if vs != vec![10i64, 20, 30] {
        t.Errorf(Sprintf!("Values mismatch"));
    }
}}

test!{ fn TestEqual(t) {
    let mut a: map<i64, i64> = map::new();
    a.insert(1, 10);
    a.insert(2, 20);
    let b = maps::Clone(&a);
    if !maps::Equal(&a, &b) {
        t.Errorf(Sprintf!("Equal cloned = false"));
    }
    let mut c = a.clone();
    c.insert(3, 30);
    if maps::Equal(&a, &c) {
        t.Errorf(Sprintf!("Equal different sizes = true"));
    }
    let mut d = a.clone();
    d.insert(1, 99);  // same key, different value
    if maps::Equal(&a, &d) {
        t.Errorf(Sprintf!("Equal different values = true"));
    }
}}

test!{ fn TestCopy(t) {
    let mut dst: map<i64, i64> = map::new();
    dst.insert(1, 100);
    let mut src: map<i64, i64> = map::new();
    src.insert(1, 999);  // overwrites
    src.insert(2, 200);
    maps::Copy(&mut dst, &src);
    if dst.len() != 2 { t.Errorf(Sprintf!("Copy len = %d, want 2", dst.len() as i64)); }
    if dst.get(&1) != Some(&999) { t.Errorf(Sprintf!("Copy did not overwrite")); }
    if dst.get(&2) != Some(&200) { t.Errorf(Sprintf!("Copy did not insert new")); }
}}

test!{ fn TestDeleteFunc(t) {
    let mut m: map<i64, i64> = map::new();
    for i in 1..=10 { m.insert(i, i * 10); }
    maps::DeleteFunc(&mut m, |k, _v| *k > 5);
    if m.len() != 5 { t.Errorf(Sprintf!("DeleteFunc len = %d, want 5", m.len() as i64)); }
    for k in 1..=5 {
        if !m.contains_key(&k) {
            t.Errorf(Sprintf!("DeleteFunc removed %d by mistake", k));
        }
    }
}}

test!{ fn TestClone(t) {
    let mut a: map<String, i64> = map::new();
    a.insert("one".into(), 1);
    a.insert("two".into(), 2);
    let b = maps::Clone(&a);
    // Mutating the clone does not affect the original.
    let mut c = b;
    c.insert("three".into(), 3);
    if a.contains_key("three") {
        t.Errorf(Sprintf!("Clone was shallow"));
    }
}}
