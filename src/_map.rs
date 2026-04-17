// map<K, V>: Go's `map[K]V` ported to Rust.
//
// Newtype around HashMap<K, V> that adds:
//   - m[&key] returns &V with zero-value on miss (Go semantics, no panic)
//   - m.Get(&key) -> (V, bool) for Go's `v, ok := m[key]` pattern
//   - Deref/DerefMut to HashMap for all existing methods
//
// The zero value is lazily allocated via OnceLock on the first Index miss.

#![allow(non_camel_case_types)]

use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::Hash;
use std::ops::{Deref, DerefMut, Index};

/// Go's `map[K]V`. Thin wrapper around `HashMap<K, V>` adding
/// zero-value-on-miss indexing and `.Get()` → `(V, bool)`.
pub struct map<K, V> {
    inner: HashMap<K, V>,
    zero: std::sync::OnceLock<V>,
}

impl<K, V> map<K, V> {
    pub fn new() -> Self where K: Eq + Hash {
        map { inner: HashMap::new(), zero: std::sync::OnceLock::new() }
    }

    pub fn with_capacity(n: usize) -> Self where K: Eq + Hash {
        map { inner: HashMap::with_capacity(n), zero: std::sync::OnceLock::new() }
    }

    pub fn into_inner(self) -> HashMap<K, V> { self.inner }
}

// ── Go's `v, ok := m[key]` ────────────────────────────────────────────

impl<K: Eq + Hash, V: Clone + Default> map<K, V> {
    /// Go's `v, ok := m[key]` — returns (clone of value, true) if present,
    /// (zero-value, false) if absent.
    #[allow(non_snake_case)]
    pub fn Get<Q: ?Sized + Eq + Hash>(&self, key: &Q) -> (V, bool)
    where K: Borrow<Q>
    {
        match self.inner.get(key) {
            Some(v) => (v.clone(), true),
            None => (V::default(), false),
        }
    }
}

// ── Index: m[&key] returns &V, zero-value on miss ─────────────────────

impl<K, V, Q: ?Sized> Index<&Q> for map<K, V>
where
    K: Eq + Hash + Borrow<Q>,
    Q: Eq + Hash,
    V: Default + Send + Sync,
{
    type Output = V;
    fn index(&self, key: &Q) -> &V {
        self.inner.get(key)
            .unwrap_or_else(|| self.zero.get_or_init(V::default))
    }
}

// ── Deref to HashMap ──────────────────────────────────────────────────

impl<K, V> Deref for map<K, V> {
    type Target = HashMap<K, V>;
    fn deref(&self) -> &HashMap<K, V> { &self.inner }
}
impl<K, V> DerefMut for map<K, V> {
    fn deref_mut(&mut self) -> &mut HashMap<K, V> { &mut self.inner }
}

// ── Conversions ───────────────────────────────────────────────────────

impl<K: Eq + Hash, V> From<HashMap<K, V>> for map<K, V> {
    fn from(m: HashMap<K, V>) -> Self {
        map { inner: m, zero: std::sync::OnceLock::new() }
    }
}
impl<K, V> From<map<K, V>> for HashMap<K, V> {
    fn from(m: map<K, V>) -> HashMap<K, V> { m.inner }
}

// ── Iteration ─────────────────────────────────────────────────────────

impl<K, V> IntoIterator for map<K, V> {
    type Item = (K, V);
    type IntoIter = std::collections::hash_map::IntoIter<K, V>;
    fn into_iter(self) -> Self::IntoIter { self.inner.into_iter() }
}
impl<'a, K, V> IntoIterator for &'a map<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = std::collections::hash_map::Iter<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter { self.inner.iter() }
}
impl<'a, K, V> IntoIterator for &'a mut map<K, V> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = std::collections::hash_map::IterMut<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter { self.inner.iter_mut() }
}
impl<K: Eq + Hash, V> FromIterator<(K, V)> for map<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        map { inner: HashMap::from_iter(iter), zero: std::sync::OnceLock::new() }
    }
}
impl<K: Eq + Hash, V> Extend<(K, V)> for map<K, V> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        self.inner.extend(iter);
    }
}

// ── Traits ────────────────────────────────────────────────────────────

impl<K: Clone + Eq + Hash, V: Clone> Clone for map<K, V> {
    fn clone(&self) -> Self {
        map { inner: self.inner.clone(), zero: std::sync::OnceLock::new() }
    }
}
impl<K: std::fmt::Debug, V: std::fmt::Debug> std::fmt::Debug for map<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}
impl<K: Eq + Hash, V> Default for map<K, V> {
    fn default() -> Self { map { inner: HashMap::new(), zero: std::sync::OnceLock::new() } }
}
impl<K: Eq + Hash, V: PartialEq> PartialEq for map<K, V> {
    fn eq(&self, other: &Self) -> bool { self.inner == other.inner }
}
impl<K: Eq + Hash, V: Eq> Eq for map<K, V> {}

// Cross-type PartialEq with HashMap
impl<K: Eq + Hash, V: PartialEq> PartialEq<HashMap<K, V>> for map<K, V> {
    fn eq(&self, other: &HashMap<K, V>) -> bool { &self.inner == other }
}
impl<K: Eq + Hash, V: PartialEq> PartialEq<map<K, V>> for HashMap<K, V> {
    fn eq(&self, other: &map<K, V>) -> bool { self == &other.inner }
}

// ── RangeIter for range! macro ────────────────────────────────────────

impl<'a, K, V> crate::range::RangeIter for &'a map<K, V> {
    type Item = (&'a K, &'a V);
    type Iter = std::collections::hash_map::Iter<'a, K, V>;
    fn range(self) -> Self::Iter { self.inner.iter() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_returns_zero_on_miss() {
        let m: map<String, i64> = map::new();
        // Go: v := m["missing"] → 0
        assert_eq!(m[&String::from("missing")], 0);
    }

    #[test]
    fn index_returns_value_on_hit() {
        let mut m: map<String, i64> = map::new();
        m.insert("key".into(), 42);
        assert_eq!(m[&String::from("key")], 42);
    }

    #[test]
    fn get_returns_tuple() {
        let mut m: map<String, i64> = map::new();
        m.insert("a".into(), 10);
        // Go: v, ok := m["a"]
        let (v, ok) = m.Get("a");
        assert_eq!(v, 10);
        assert!(ok);
        // Go: v, ok := m["missing"]
        let (v, ok) = m.Get("missing");
        assert_eq!(v, 0); // zero value
        assert!(!ok);
    }

    #[test]
    fn from_hashmap_roundtrip() {
        let mut hm = HashMap::new();
        hm.insert("x".to_string(), 1i64);
        let m: map<String, i64> = hm.into();
        assert_eq!(m.len(), 1);
        let hm2: HashMap<String, i64> = m.into();
        assert_eq!(hm2.len(), 1);
    }

    #[test]
    fn deref_gives_hashmap_methods() {
        let mut m: map<String, i64> = map::new();
        m.insert("a".into(), 1);
        assert!(m.contains_key("a"));
        assert_eq!(m.len(), 1);
        m.remove("a");
        assert!(m.is_empty());
    }
}
