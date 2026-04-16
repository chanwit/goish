// range: trait backing the `range!` macro.
//
// Different Go types give different shapes to `range`:
//   - slices/arrays → (index, &value)
//   - maps          → (&key, &value)
//   - strings       → (byte-index, rune/char)
//   - channels      → value (we only iterate values out of a Chan)
//
// `RangeIter` dispatches each of these to the right std iterator.

pub trait RangeIter {
    type Item;
    type Iter: Iterator<Item = Self::Item>;
    fn range(self) -> Self::Iter;
}

// ── slices & Vecs ──────────────────────────────────────────────────────

impl<'a, T> RangeIter for &'a [T] {
    type Item = (usize, &'a T);
    type Iter = std::iter::Enumerate<std::slice::Iter<'a, T>>;
    fn range(self) -> Self::Iter {
        self.iter().enumerate()
    }
}

// Fixed-size arrays — Go's `[N]T` or inline `[...]T{...}`.
impl<'a, T, const N: usize> RangeIter for &'a [T; N] {
    type Item = (usize, &'a T);
    type Iter = std::iter::Enumerate<std::slice::Iter<'a, T>>;
    fn range(self) -> Self::Iter {
        self.iter().enumerate()
    }
}

impl<'a, T> RangeIter for &'a Vec<T> {
    type Item = (usize, &'a T);
    type Iter = std::iter::Enumerate<std::slice::Iter<'a, T>>;
    fn range(self) -> Self::Iter {
        self.iter().enumerate()
    }
}

// ── maps ───────────────────────────────────────────────────────────────

impl<'a, K, V> RangeIter for &'a std::collections::HashMap<K, V> {
    type Item = (&'a K, &'a V);
    type Iter = std::collections::hash_map::Iter<'a, K, V>;
    fn range(self) -> Self::Iter {
        self.iter()
    }
}

// ── strings ────────────────────────────────────────────────────────────

impl<'a> RangeIter for &'a str {
    type Item = (usize, char);
    type Iter = std::iter::Enumerate<std::str::Chars<'a>>;
    fn range(self) -> Self::Iter {
        self.chars().enumerate()
    }
}

impl<'a> RangeIter for &'a String {
    type Item = (usize, char);
    type Iter = std::iter::Enumerate<std::str::Chars<'a>>;
    fn range(self) -> Self::Iter {
        self.chars().enumerate()
    }
}

#[cfg(test)]
mod tests {
    use crate::types::*;

    #[test]
    fn range_slice_gives_index_and_ref() {
        let v: slice<int> = crate::slice!([]int{10, 20, 30});
        let mut collected: Vec<(usize, int)> = Vec::new();
        crate::range!(&v, |i, val| {
            collected.push((i, *val));
        });
        assert_eq!(collected, vec![(0, 10), (1, 20), (2, 30)]);
    }

    #[test]
    fn range_map_gives_key_and_value_refs() {
        let m: map<string, int> = crate::map!([string]int{"a" => 1, "b" => 2});
        let mut total = 0i64;
        crate::range!(&m, |_k, v| {
            total += *v;
        });
        assert_eq!(total, 3);
    }

    #[test]
    fn range_str_gives_index_and_rune() {
        let mut chars: Vec<(usize, char)> = Vec::new();
        crate::range!("abc", |i, r| {
            chars.push((i, r));
        });
        assert_eq!(chars, vec![(0, 'a'), (1, 'b'), (2, 'c')]);
    }

    #[test]
    fn range_value_only_form() {
        let v: slice<int> = crate::slice!([]int{1, 2, 3});
        let mut sum = 0i64;
        crate::range!(v, |x| { sum += x; });
        assert_eq!(sum, 6);
    }
}
