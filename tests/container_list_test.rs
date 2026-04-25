// Port of go1.25.5 src/container/list/list_test.go — core semantics.
//
// goish's container/list exposes a slab/index API (Element is just an
// index handle, methods on List take &mut self), so the Go pattern
// `l.Front().Next().Value` becomes `list.Next(list.Front().unwrap())`.
// Tests check invariants (Len, iteration, Remove) rather than the
// pointer-exact API.

#![allow(non_snake_case)]
use goish::prelude::*;
use goish::container::list;

fn values(l: &list::List<i64>) -> Vec<i64> {
    l.Iter().cloned().collect()
}

test!{ fn TestNewEmpty(t) {
    let l = make!(list[i64]);
    if l.Len() != 0 { t.Errorf(Sprintf!("empty Len = %d", l.Len())); }
    if l.Front().is_some() { t.Errorf(Sprintf!("empty Front should be None")); }
    if l.Back().is_some() { t.Errorf(Sprintf!("empty Back should be None")); }
}}

test!{ fn TestPushBack(t) {
    let mut l = make!(list[i64]);
    l.PushBack(1); l.PushBack(2); l.PushBack(3);
    if l.Len() != 3 { t.Errorf(Sprintf!("Len = %d, want 3", l.Len())); }
    let got = values(&l);
    if got != vec![1, 2, 3] {
        t.Errorf(Sprintf!("PushBack order: got %d items", got.len() as i64));
    }
}}

test!{ fn TestPushFront(t) {
    let mut l = make!(list[i64]);
    l.PushFront(1); l.PushFront(2); l.PushFront(3);
    let got = values(&l);
    if got != vec![3, 2, 1] {
        t.Errorf(Sprintf!("PushFront: got %d items", got.len() as i64));
    }
}}

test!{ fn TestRemove(t) {
    let mut l = make!(list[i64]);
    let a = l.PushBack(10);
    l.PushBack(20);
    l.PushBack(30);
    let v = l.Remove(a);
    if v != Some(10) { t.Errorf(Sprintf!("Remove returned %v", v.is_some())); }
    if l.Len() != 2 { t.Errorf(Sprintf!("after Remove Len = %d, want 2", l.Len())); }
    let got = values(&l);
    if got != vec![20, 30] {
        t.Errorf(Sprintf!("after Remove: got %d items", got.len() as i64));
    }
}}

test!{ fn TestFrontBack(t) {
    let mut l = make!(list[i64]);
    l.PushBack(1);
    l.PushBack(2);
    l.PushBack(3);
    let f = l.Front().unwrap();
    let b = l.Back().unwrap();
    if l.Value(f) != Some(&1) {
        t.Errorf(Sprintf!("Front value mismatch"));
    }
    if l.Value(b) != Some(&3) {
        t.Errorf(Sprintf!("Back value mismatch"));
    }
}}

test!{ fn TestNextPrev(t) {
    let mut l = make!(list[i64]);
    l.PushBack(1); l.PushBack(2); l.PushBack(3);
    let f = l.Front().unwrap();
    let n = l.Next(f).unwrap();
    if l.Value(n) != Some(&2) {
        t.Errorf(Sprintf!("Next of Front value mismatch"));
    }
    let p = l.Prev(n).unwrap();
    if l.Value(p) != Some(&1) {
        t.Errorf(Sprintf!("Prev of Next value mismatch"));
    }
    // Prev of Front is None.
    if l.Prev(f).is_some() {
        t.Errorf(Sprintf!("Prev(Front) should be None"));
    }
}}

test!{ fn TestMixedPushRemove(t) {
    let mut l = make!(list[i64]);
    let _ = l.PushBack(1);
    let b = l.PushBack(2);
    let _ = l.PushBack(3);
    l.PushFront(0);
    // list: 0, 1, 2, 3
    if l.Len() != 4 { t.Errorf(Sprintf!("Len = %d, want 4", l.Len())); }
    l.Remove(b); // remove middle
    let got = values(&l);
    if got != vec![0, 1, 3] {
        t.Errorf(Sprintf!("after mixed ops: %d items", got.len() as i64));
    }
}}
