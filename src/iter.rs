//! iter: Go 1.23's iter package — generic sequence iterators.
//!
//!   Go                                  goish
//!   ─────────────────────────────────   ──────────────────────────────────
//!   iter.Seq[V]                         iter::Seq<V>     (impl FnMut(&mut dyn FnMut(V) -> bool))
//!   iter.Seq2[K, V]                     iter::Seq2<K, V>
//!   for v := range seq { ... }          seq.for_each(|v| -> bool { ...; true })
//!
//! Go's `Seq[V]` is `func(yield func(V) bool)`. The `yield` callback
//! returns `false` to stop iteration early. In goish we expose the same
//! shape via traits so user code can call `seq.for_each(|v| { ... })`.
//!
//! Natural Rust iterators (`impl Iterator`) still work: we provide
//! `from_iterator` / `collect` bridges.

/// Shape of a Go `iter.Seq[V]`.
pub trait Seq<V> {
    /// Run the sequence, calling `yield_` on each value. The callback
    /// returns `true` to continue, `false` to stop early.
    fn for_each<F: FnMut(V) -> bool>(&mut self, yield_: F);
}

/// Shape of a Go `iter.Seq2[K, V]`.
pub trait Seq2<K, V> {
    fn for_each<F: FnMut(K, V) -> bool>(&mut self, yield_: F);
}

// Blanket impl: any FnMut(&mut dyn FnMut(V) -> bool) is a Seq.
impl<V, T: FnMut(&mut dyn FnMut(V) -> bool)> Seq<V> for T {
    fn for_each<F: FnMut(V) -> bool>(&mut self, mut yield_: F) {
        self(&mut |v| yield_(v));
    }
}

impl<K, V, T: FnMut(&mut dyn FnMut(K, V) -> bool)> Seq2<K, V> for T {
    fn for_each<F: FnMut(K, V) -> bool>(&mut self, mut yield_: F) {
        self(&mut |k, v| yield_(k, v));
    }
}

/// Build a `Seq<V>` from a Rust `IntoIterator`.
#[allow(non_snake_case)]
pub fn FromIterator<I: IntoIterator>(iter: I) -> impl FnMut(&mut dyn FnMut(I::Item) -> bool)
where
    I::IntoIter: 'static,
    I::Item: 'static,
{
    let mut it = Some(iter.into_iter());
    move |yield_| {
        if let Some(mut iter) = it.take() {
            for v in &mut iter {
                if !yield_(v) { break; }
            }
        }
    }
}

/// Collect a `Seq<V>` into a `Vec<V>`.
#[allow(non_snake_case)]
pub fn Collect<V, S: Seq<V>>(mut seq: S) -> Vec<V> {
    let mut out = Vec::new();
    seq.for_each(|v| { out.push(v); true });
    out
}

/// Collect a `Seq2<K, V>` into a `Vec<(K, V)>`.
#[allow(non_snake_case)]
pub fn Collect2<K, V, S: Seq2<K, V>>(mut seq: S) -> Vec<(K, V)> {
    let mut out = Vec::new();
    seq.for_each(|k, v| { out.push((k, v)); true });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seq_from_iter_collects() {
        let seq = FromIterator(vec![1i64, 2, 3, 4]);
        let v = Collect(seq);
        assert_eq!(v, vec![1i64, 2, 3, 4]);
    }

    #[test]
    fn seq_early_stop() {
        // A Seq that yields 1,2,3,... forever, stopping when yield returns false.
        let mut counter = 0i64;
        let seq = move |yield_: &mut dyn FnMut(i64) -> bool| {
            loop {
                counter += 1;
                if !yield_(counter) { return; }
            }
        };
        let mut collected = Vec::new();
        let mut seq = seq;
        seq.for_each(|v| {
            collected.push(v);
            v < 5
        });
        assert_eq!(collected, vec![1i64, 2, 3, 4, 5]);
    }

    #[test]
    fn seq2_pairs() {
        let seq = |yield_: &mut dyn FnMut(i64, &'static str) -> bool| {
            for (k, v) in &[(1i64, "a"), (2, "b"), (3, "c")] {
                if !yield_(*k, *v) { return; }
            }
        };
        let out = Collect2(seq);
        assert_eq!(out, vec![(1i64, "a"), (2, "b"), (3, "c")]);
    }
}
