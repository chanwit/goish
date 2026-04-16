// Port of go1.25.5 src/container/heap/heap_test.go — patterns adapted
// from Go's heap.Interface model to goish's concrete Heap<T>.

#![allow(non_snake_case)]
use goish::prelude::*;
use goish::container::heap::{self, Heap};

fn verify_heap_invariant<T: PartialOrd>(t: &goish::testing::T, data: &[T], i: usize) {
    let n = data.len();
    let l = 2 * i + 1;
    let r = 2 * i + 2;
    if l < n {
        if data[l] < data[i] {
            t.Errorf(Sprintf!("heap invariant invalidated at %d", i as i64));
            return;
        }
        verify_heap_invariant(t, data, l);
    }
    if r < n {
        if data[r] < data[i] {
            t.Errorf(Sprintf!("heap invariant invalidated at %d", i as i64));
            return;
        }
        verify_heap_invariant(t, data, r);
    }
}

// Expose Heap's internal data via a pop-all helper for verification.
fn drain_to_vec<T>(h: &mut Heap<T>) -> Vec<T> {
    let mut out = Vec::new();
    while let Some(v) = h.Pop() { out.push(v); }
    out
}

test!{ fn TestPushPopSorted(t) {
    let mut h: Heap<i64> = heap::New(|a, b| a < b);
    for v in [20i64, 10, 30, 5, 15, 25, 1] {
        h.Push(v);
    }
    if h.Len() != 7 {
        t.Errorf(Sprintf!("Len = %d, want 7", h.Len()));
    }
    let out = drain_to_vec(&mut h);
    let want = vec![1i64, 5, 10, 15, 20, 25, 30];
    if out != want {
        t.Errorf(Sprintf!("out %d items; first = %d", out.len() as i64, out[0]));
    }
}}

test!{ fn TestMaxHeap(t) {
    let mut h: Heap<i64> = heap::New(|a, b| a > b);
    for v in [3i64, 1, 4, 1, 5, 9, 2, 6, 5] { h.Push(v); }
    let first = h.Pop();
    if first != Some(9) {
        t.Errorf(Sprintf!("max-heap first pop = %d, want 9", first.unwrap_or(-1)));
    }
}}

test!{ fn TestInitBuildsHeap(t) {
    // Push items out of order, then Init.
    let mut h: Heap<i64> = heap::New(|a, b| a < b);
    for v in [5i64, 4, 3, 2, 1] { h.Push(v); }
    h.Init();
    let out = drain_to_vec(&mut h);
    let want = vec![1i64, 2, 3, 4, 5];
    if out != want {
        t.Errorf(Sprintf!("Init+drain: got %d items", out.len() as i64));
    }
}}

test!{ fn TestPeek(t) {
    let mut h: Heap<i64> = heap::New(|a, b| a < b);
    h.Push(3); h.Push(1); h.Push(2);
    let p = h.Peek();
    if p != Some(&1) {
        t.Errorf(Sprintf!("Peek = %v, want Some(1)", p.is_some()));
    }
    if h.Len() != 3 {
        t.Errorf(Sprintf!("Peek consumed; Len = %d", h.Len()));
    }
}}

test!{ fn TestRemove(t) {
    let mut h: Heap<i64> = heap::New(|a, b| a < b);
    for v in [1i64, 2, 3, 4, 5] { h.Push(v); }
    // Remove(0) — the min.
    let r = h.Remove(0);
    if r != Some(1) {
        t.Errorf(Sprintf!("Remove(0) = %v, want Some(1)", r.unwrap_or(-1)));
    }
    if h.Len() != 4 { t.Errorf(Sprintf!("Len = %d, want 4", h.Len())); }
    // Remaining drains in sorted order.
    let out = drain_to_vec(&mut h);
    if out != vec![2i64, 3, 4, 5] {
        t.Errorf(Sprintf!("after Remove(0) drain: got %d items", out.len() as i64));
    }
}}

test!{ fn TestEmpty(t) {
    let mut h: Heap<i64> = heap::New(|a, b| a < b);
    if h.Len() != 0 { t.Errorf(Sprintf!("empty Len = %d", h.Len())); }
    if h.Pop() != None { t.Errorf(Sprintf!("empty Pop != None")); }
    if h.Peek() != None { t.Errorf(Sprintf!("empty Peek != None")); }
}}

test!{ fn TestStress(t) {
    // Push 1000 values 999..=0, Pop — each pop should yield increasing values.
    let mut h: Heap<i64> = heap::New(|a, b| a < b);
    for i in (0..1000i64).rev() { h.Push(i); }
    let mut last = -1i64;
    for _ in 0..1000 {
        let v = h.Pop().unwrap();
        if v <= last {
            t.Errorf(Sprintf!("not sorted: %d after %d", v, last));
            return;
        }
        last = v;
    }
    let _ = verify_heap_invariant::<i64>;  // silence unused-fn warning
}}
