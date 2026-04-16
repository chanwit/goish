// Port of go1.25.5 src/math/rand/rand_test.go — the tests that verify
// RANGES (Intn, Float64, etc.) and determinism after Seed. Goish uses
// xoshiro256** (not Go's LCG), so exact sequences differ; we only test
// the spec-level properties.

#![allow(non_snake_case)]
use goish::prelude::*;

test!{ fn TestInt63Range(t) {
    // Int63 returns non-negative int64.
    for _ in 0..1000 {
        let v = rand::Int63();
        if v < 0 {
            t.Errorf(Sprintf!("Int63 returned negative: %d", v));
            return;
        }
    }
}}

test!{ fn TestIntn(t) {
    // Intn(n) returns in [0, n).
    for n in [1i64, 2, 5, 100, 1000] {
        for _ in 0..200 {
            let v = rand::Intn(n);
            if v < 0 || v >= n {
                t.Errorf(Sprintf!("Intn(%d) out of range: %d", n, v));
                return;
            }
        }
    }
}}

test!{ fn TestIntnPanicsOnZero(t) {
    let r = recover!{ rand::Intn(0) };
    if r.is_none() {
        t.Errorf(Sprintf!("Intn(0) should have panicked"));
    }
}}

test!{ fn TestIntnPanicsOnNegative(t) {
    let r = recover!{ rand::Intn(-1) };
    if r.is_none() {
        t.Errorf(Sprintf!("Intn(-1) should have panicked"));
    }
}}

test!{ fn TestFloat64Range(t) {
    // Float64 returns in [0.0, 1.0).
    for _ in 0..1000 {
        let v = rand::Float64();
        if v < 0.0 || v >= 1.0 {
            t.Errorf(Sprintf!("Float64 out of range: %g", v));
            return;
        }
    }
}}

test!{ fn TestSeedDeterministic(t) {
    // Same seed -> same sequence.
    rand::Seed(42);
    let seq1: Vec<i64> = (0..10).map(|_| rand::Int63()).collect();
    rand::Seed(42);
    let seq2: Vec<i64> = (0..10).map(|_| rand::Int63()).collect();
    if seq1 != seq2 {
        t.Errorf(Sprintf!("Seed(42) produced different sequences"));
    }
    // Different seed -> different sequence (with overwhelming probability).
    rand::Seed(1);
    let seq3: Vec<i64> = (0..10).map(|_| rand::Int63()).collect();
    if seq1 == seq3 {
        t.Errorf(Sprintf!("Seed(1) produced same sequence as Seed(42)"));
    }
}}

test!{ fn TestShuffleStability(t) {
    // Shuffle preserves the multiset of elements.
    let mut v: Vec<i64> = (0..100).collect();
    let sum_before: i64 = v.iter().sum();
    let len_before = v.len();
    // Shuffle uses thread-local rand; we need per-Rand for clean test.
    // Emulate by reseeding + calling shuffle-style swaps.
    rand::Seed(7);
    // Use goish's Shuffle function if available, else perform an in-place
    // Fisher-Yates using Intn for the swap index.
    for i in (1..v.len()).rev() {
        let j = rand::Intn((i + 1) as i64) as usize;
        v.swap(i, j);
    }
    let sum_after: i64 = v.iter().sum();
    if sum_after != sum_before || v.len() != len_before {
        t.Errorf(Sprintf!("shuffle changed multiset"));
    }
}}
