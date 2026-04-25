// Port of go1.25.5 src/slices/slices_test.go + sort_test.go — coverage
// of the core API surface goish implements.

#![allow(non_snake_case)]
use goish::prelude::*;

test!{ fn TestEqual(t) {
    if !slices::Equal(&[1i64, 2, 3], &[1, 2, 3]) {
        t.Errorf(Sprintf!("Equal identical = false"));
    }
    if slices::Equal(&[1i64, 2, 3], &[1, 2, 4]) {
        t.Errorf(Sprintf!("Equal different = true"));
    }
    if slices::Equal(&[1i64, 2, 3], &[1, 2]) {
        t.Errorf(Sprintf!("Equal different lengths = true"));
    }
}}

test!{ fn TestIndex(t) {
    let s = vec![1i64, 2, 3, 2, 1];
    if slices::Index(&s, &2) != 1 {
        t.Errorf(Sprintf!("Index(2) = %d, want 1", slices::Index(&s, &2)));
    }
    if slices::Index(&s, &99) != -1 {
        t.Errorf(Sprintf!("Index missing = %d, want -1", slices::Index(&s, &99)));
    }
}}

test!{ fn TestContains(t) {
    let s = slice!([]string { "a", "b", "c" });
    if !slices::Contains(&s, &"b".into()) {
        t.Errorf(Sprintf!("Contains(b) = false"));
    }
    if slices::Contains(&s, &"z".into()) {
        t.Errorf(Sprintf!("Contains(z) = true"));
    }
}}

test!{ fn TestSort(t) {
    let mut s = vec![3i64, 1, 4, 1, 5, 9, 2, 6];
    slices::Sort(&mut s);
    if s != vec![1i64, 1, 2, 3, 4, 5, 6, 9] {
        t.Errorf(Sprintf!("Sort result mismatch"));
    }
    if !slices::IsSorted(&s) {
        t.Errorf(Sprintf!("IsSorted after Sort = false"));
    }
}}

test!{ fn TestSortFunc(t) {
    let mut s = vec![3i64, 1, 4, 1, 5, 9];
    slices::SortFunc(&mut s, |a, b| cmp::Compare(b, a));  // descending
    if s != vec![9i64, 5, 4, 3, 1, 1] {
        t.Errorf(Sprintf!("SortFunc descending mismatch"));
    }
}}

test!{ fn TestBinarySearch(t) {
    let s = vec![1i64, 3, 5, 7, 9];
    let (i, f) = slices::BinarySearch(&s, &5);
    if !f || i != 2 {
        t.Errorf(Sprintf!("BinarySearch(5) = (%d, %v), want (2, true)", i, f));
    }
    let (i, f) = slices::BinarySearch(&s, &4);
    if f {
        t.Errorf(Sprintf!("BinarySearch(4) found unexpectedly"));
    }
    if i != 2 {
        t.Errorf(Sprintf!("BinarySearch(4) insertion = %d, want 2", i));
    }
}}

test!{ fn TestMinMax(t) {
    let s = vec![3i64, 1, 4, 1, 5, 9, 2, 6];
    if slices::Min(&s) != 1 { t.Errorf(Sprintf!("Min != 1")); }
    if slices::Max(&s) != 9 { t.Errorf(Sprintf!("Max != 9")); }
}}

test!{ fn TestMinPanicsOnEmpty(t) {
    let empty: slice<int> = slice::new();
    let r = recover!{ slices::Min(&empty) };
    if r.is_none() { t.Errorf(Sprintf!("Min(empty) should panic")); }
}}

test!{ fn TestReverse(t) {
    let mut s = vec![1i64, 2, 3, 4];
    slices::Reverse(&mut s);
    if s != vec![4i64, 3, 2, 1] {
        t.Errorf(Sprintf!("Reverse mismatch"));
    }
}}

test!{ fn TestInsertDelete(t) {
    let mut s = vec![1i64, 2, 5, 6];
    slices::Insert(&mut s, 2, &[3, 4]);
    if s != vec![1i64, 2, 3, 4, 5, 6] {
        t.Errorf(Sprintf!("Insert mismatch"));
    }
    slices::Delete(&mut s, 1, 3);
    if s != vec![1i64, 4, 5, 6] {
        t.Errorf(Sprintf!("Delete mismatch"));
    }
}}

test!{ fn TestCompactCompactFunc(t) {
    let mut s = vec![1i64, 1, 2, 3, 3, 3, 4, 4];
    slices::Compact(&mut s);
    if s != vec![1i64, 2, 3, 4] {
        t.Errorf(Sprintf!("Compact mismatch"));
    }
}}

test!{ fn TestConcatRepeat(t) {
    let c: slice<int> = slices::Concat(&[&[1, 2], &[3, 4], &[5]]).into();
    if c != vec![1i64, 2, 3, 4, 5] {
        t.Errorf(Sprintf!("Concat mismatch"));
    }
    let r: slice<int> = slices::Repeat(&[1, 2], 3).into();
    if r != vec![1i64, 2, 1, 2, 1, 2] {
        t.Errorf(Sprintf!("Repeat mismatch"));
    }
}}

test!{ fn TestCompare(t) {
    if slices::Compare(&[1i64, 2], &[1, 3]) != -1 {
        t.Errorf(Sprintf!("Compare [1,2] [1,3] != -1"));
    }
    if slices::Compare(&[1i64, 2], &[1, 2]) != 0 {
        t.Errorf(Sprintf!("Compare equal != 0"));
    }
    if slices::Compare(&[1i64, 2, 3], &[1, 2]) != 1 {
        t.Errorf(Sprintf!("Compare longer != 1"));
    }
}}

test!{ fn TestDeleteFunc(t) {
    let mut s = vec![1i64, 2, 3, 4, 5, 6];
    slices::DeleteFunc(&mut s, |x| *x % 2 == 0);
    if s != vec![1i64, 3, 5] {
        t.Errorf(Sprintf!("DeleteFunc mismatch"));
    }
}}
