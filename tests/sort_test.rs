// Port of go1.25.5 src/sort/sort_test.go — table-driven sorts and searches.
//
// Skipped: benchmarks, TestSort (reflection-based heap-sort comparison),
// TestNonDeterministic (stability stress). Covered: Ints, Strings,
// Float64s, Slice, SliceStable, SearchInts, SearchStrings.

#![allow(non_snake_case)]
use goish::prelude::*;

test!{ fn TestSortInts(t) {
    let mut data: slice<int> = slice!([]int { 74, 59, 238, -784, 9845, 959, 905, 0, 0, 42, 7586, -5467984, 7586 });
    sort::Ints(&mut data);
    for i in 1..(data.len() as int) {
        if data[i] < data[i-1] {
            t.Errorf(Sprintf!("not sorted at %d", i));
            return;
        }
    }
    if !sort::IntsAreSorted(&data) {
        t.Errorf(Sprintf!("IntsAreSorted returned false after sort"));
    }
}}

test!{ fn TestSortStrings(t) {
    let mut data: slice<string> = slice!([]string { "Banana", "apple", "cherry", "BANANA", "" });
    sort::Strings(&mut data);
    for i in 1..(data.len() as int) {
        if data[i] < data[i-1] {
            t.Errorf(Sprintf!("not sorted at %d", i));
            return;
        }
    }
}}

test!{ fn TestSortFloat64s(t) {
    let mut data: slice<float64> = slice!([]float64 { 74.3, 59.0, 238.2, -784.0, 2.3, 9845.768 });
    sort::Float64s(&mut data);
    for i in 1..(data.len() as int) {
        if data[i] < data[i-1] {
            t.Errorf(Sprintf!("not sorted at %d", i));
            return;
        }
    }
    if !sort::Float64sAreSorted(&data) {
        t.Errorf(Sprintf!("Float64sAreSorted = false after sort"));
    }
}}

test!{ fn TestSortSlice(t) {
    let mut data: slice<int> = slice!([]int { 3, 1, 4, 1, 5, 9, 2, 6 });
    sort::Slice(&mut data, |a, b| a < b);
    if data != vec![1i64, 1, 2, 3, 4, 5, 6, 9] {
        t.Errorf(Sprintf!("sort::Slice ascending failed"));
    }
    sort::Slice(&mut data, |a, b| a > b);
    if data != vec![9i64, 6, 5, 4, 3, 2, 1, 1] {
        t.Errorf(Sprintf!("sort::Slice descending failed"));
    }
}}

test!{ fn TestSortSliceStable(t) {
    // Stable: equal-keyed items preserve original order.
    let mut data: Vec<(i64, &'static str)> = vec![
        (1, "a"), (2, "b"), (1, "c"), (2, "d"), (1, "e"),
    ];
    sort::SliceStable(&mut data, |a, b| a.0 < b.0);
    // All 1s first (in original order a, c, e), then 2s (b, d).
    let mut tags: Vec<&str> = Vec::with_capacity(data.len());
    for p in &data { tags.push(p.1); }
    if tags != vec!["a", "c", "e", "b", "d"] {
        t.Errorf(Sprintf!("SliceStable did not preserve order"));
    }
}}

test!{ fn TestSearchInts(t) {
    let data: slice<int> = slice!([]int { 0, 42, 59, 74, 238, 905, 959 });
    // exact matches
    for (v, want) in [(0, 0), (42, 1), (59, 2), (959, 6)] {
        let got = sort::SearchInts(&data, v);
        if got != want {
            t.Errorf(Sprintf!("SearchInts(%d) = %d, want %d", v, got, want));
        }
    }
    // missing — returns insertion point
    let got = sort::SearchInts(&data, 100);
    if got != 4 {
        t.Errorf(Sprintf!("SearchInts(100) = %d, want 4", got));
    }
    let got = sort::SearchInts(&data, 1000);
    if got != 7 {
        t.Errorf(Sprintf!("SearchInts(1000) = %d, want 7 (end)", got));
    }
}}

test!{ fn TestSearchStrings(t) {
    let data: slice<string> = slice!([]string { "alice", "bob", "carol", "dave" });
    let got = sort::SearchStrings(&data, "carol");
    if got != 2 {
        t.Errorf(Sprintf!("SearchStrings(carol) = %d, want 2", got));
    }
    let got = sort::SearchStrings(&data, "zzz");
    if got != 4 {
        t.Errorf(Sprintf!("SearchStrings(zzz) = %d, want 4 (end)", got));
    }
}}

test!{ fn TestStressRandom(t) {
    // Large sort: 1000 reverse-sorted ints.
    let mut data: slice<int> = make!([]int, 0, 1000);
    for i in (0..1000i64).rev() { data.push(i); }
    sort::Ints(&mut data);
    for i in 0..1000 {
        if data[i] != i as i64 {
            t.Errorf(Sprintf!("stress: data[%d] = %d, want %d", i as i64, data[i], i as i64));
            return;
        }
    }
}}

test!{ fn TestSortedEmpty(t) {
    let mut empty: slice<int> = slice::new();
    sort::Ints(&mut empty);
    if !sort::IntsAreSorted(&empty) {
        t.Errorf(Sprintf!("empty slice not sorted?"));
    }
}}
