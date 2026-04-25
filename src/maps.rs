//! maps: Go's maps package — generic map operations.
//!
//!   Go                                  goish
//!   ─────────────────────────────────   ──────────────────────────────────
//!   maps.Keys(m)                        maps::Keys(&m)      — Vec<K>
//!   maps.Values(m)                      maps::Values(&m)    — Vec<V>
//!   maps.Equal(m1, m2)                  maps::Equal(&m1, &m2)
//!   maps.Clone(m)                       maps::Clone(&m)
//!   maps.Copy(dst, src)                 maps::Copy(&mut dst, &src)
//!   maps.DeleteFunc(m, f)               maps::DeleteFunc(&mut m, f)
//!
//! Go 1.23 changed `Keys` / `Values` to return an `iter.Seq`; goish
//! returns a `Vec` for simpler call sites. The order is arbitrary (as
//! in Go).

use crate::types::map;
use std::hash::Hash;

/// `maps.Keys(m)` — collect keys into a slice (order is arbitrary).
#[allow(non_snake_case)]
pub fn Keys<K: Clone + Eq + Hash, V>(m: &map<K, V>) -> crate::types::slice<K> {
    let v: Vec<K> = m.keys().cloned().collect();
    v.into()
}

/// `maps.Values(m)` — collect values into a slice (order is arbitrary).
#[allow(non_snake_case)]
pub fn Values<K: Eq + Hash, V: Clone>(m: &map<K, V>) -> crate::types::slice<V> {
    let v: Vec<V> = m.values().cloned().collect();
    v.into()
}

/// `maps.Equal(m1, m2)` — equal iff same keys and equal values.
#[allow(non_snake_case)]
pub fn Equal<K: Eq + Hash, V: PartialEq>(m1: &map<K, V>, m2: &map<K, V>) -> bool {
    if m1.len() != m2.len() { return false; }
    for (k, v1) in m1 {
        match m2.get(k) {
            Some(v2) if v1 == v2 => {}
            _ => return false,
        }
    }
    true
}

/// `maps.EqualFunc(m1, m2, eq)` — equality using custom value comparator.
#[allow(non_snake_case)]
pub fn EqualFunc<K: Eq + Hash, V1, V2, F: Fn(&V1, &V2) -> bool>(
    m1: &map<K, V1>, m2: &map<K, V2>, eq: F,
) -> bool {
    if m1.len() != m2.len() { return false; }
    for (k, v1) in m1 {
        match m2.get(k) {
            Some(v2) if eq(v1, v2) => {}
            _ => return false,
        }
    }
    true
}

/// `maps.Clone(m)` — shallow clone of every (key, value).
#[allow(non_snake_case)]
pub fn Clone<K: Clone + Eq + Hash, V: Clone>(m: &map<K, V>) -> map<K, V> {
    m.clone()
}

/// `maps.Copy(dst, src)` — inserts every (k, v) from src into dst,
/// overwriting on conflict.
#[allow(non_snake_case)]
pub fn Copy<K: Clone + Eq + Hash, V: Clone>(dst: &mut map<K, V>, src: &map<K, V>) {
    for (k, v) in src { dst.insert(k.clone(), v.clone()); }
}

/// `maps.DeleteFunc(m, del)` — removes every (k, v) where del(k, v) is true.
#[allow(non_snake_case)]
pub fn DeleteFunc<K: Eq + Hash, V, F: FnMut(&K, &V) -> bool>(
    m: &mut map<K, V>, mut del: F,
) {
    m.retain(|k, v| !del(k, v));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_and_values() {
        let mut m: map<&str, i64> = map::new();
        m.insert("a", 1);
        m.insert("b", 2);
        let mut ks = Keys(&m);
        ks.sort();
        assert_eq!(ks, vec!["a", "b"]);
        let mut vs = Values(&m);
        vs.sort();
        assert_eq!(vs, vec![1, 2]);
    }

    #[test]
    fn equal_and_clone() {
        let mut a: map<i64, i64> = map::new();
        a.insert(1, 10);
        a.insert(2, 20);
        let b = Clone(&a);
        assert!(Equal(&a, &b));
        let mut c = a.clone();
        c.insert(3, 30);
        assert!(!Equal(&a, &c));
    }

    #[test]
    fn copy_merges() {
        let mut dst: map<i64, i64> = map::new();
        dst.insert(1, 10);
        let mut src: map<i64, i64> = map::new();
        src.insert(2, 20);
        src.insert(1, 99);  // should overwrite
        Copy(&mut dst, &src);
        assert_eq!(dst.get(&1), Some(&99));
        assert_eq!(dst.get(&2), Some(&20));
    }

    #[test]
    fn delete_func_removes() {
        let mut m: map<i64, i64> = map::new();
        for i in 1..=5i64 { m.insert(i, i * 10); }
        DeleteFunc(&mut m, |k, _| *k % 2 == 0);
        assert_eq!(m.len(), 3);
        assert!(m.contains_key(&1));
        assert!(!m.contains_key(&2));
    }
}
