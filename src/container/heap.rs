//! container/heap: binary min-heap. Go's container/heap takes an interface;
//! we ship a concrete `Heap<T>` parameterised on the ordering. `Push/Pop/Peek`
//! maintain the heap invariant.
//!
//!   let mut h: heap::Heap<i64> = heap::New(|a, b| a < b); // min-heap
//!   h.Push(3); h.Push(1); h.Push(2);
//!   assert_eq!(h.Pop(), Some(1));

pub struct Heap<T> {
    data: Vec<T>,
    less: Box<dyn Fn(&T, &T) -> bool + Send>,
}

#[allow(non_snake_case)]
pub fn New<T>(less: impl Fn(&T, &T) -> bool + Send + 'static) -> Heap<T> {
    Heap { data: Vec::new(), less: Box::new(less) }
}

impl<T> Heap<T> {
    pub fn Len(&self) -> crate::types::int { self.data.len() as crate::types::int }

    pub fn Push(&mut self, v: T) {
        self.data.push(v);
        self.sift_up(self.data.len() - 1);
    }

    pub fn Pop(&mut self) -> Option<T> {
        if self.data.is_empty() { return None; }
        let last = self.data.len() - 1;
        self.data.swap(0, last);
        let v = self.data.pop();
        if !self.data.is_empty() {
            self.sift_down(0);
        }
        v
    }

    pub fn Peek(&self) -> Option<&T> { self.data.first() }

    /// Init — build a heap from arbitrary initial data.
    pub fn Init(&mut self) {
        let n = self.data.len();
        if n < 2 { return; }
        for i in (0..n / 2).rev() {
            self.sift_down(i);
        }
    }

    fn sift_up(&mut self, mut i: usize) {
        while i > 0 {
            let p = (i - 1) / 2;
            if (self.less)(&self.data[i], &self.data[p]) {
                self.data.swap(i, p);
                i = p;
            } else { break; }
        }
    }

    fn sift_down(&mut self, mut i: usize) {
        let n = self.data.len();
        loop {
            let l = 2 * i + 1;
            let r = 2 * i + 2;
            let mut smallest = i;
            if l < n && (self.less)(&self.data[l], &self.data[smallest]) { smallest = l; }
            if r < n && (self.less)(&self.data[r], &self.data[smallest]) { smallest = r; }
            if smallest == i { break; }
            self.data.swap(i, smallest);
            i = smallest;
        }
    }

    /// Drop-in replacement for push+pop in one call.
    pub fn Remove(&mut self, i: usize) -> Option<T> {
        if i >= self.data.len() { return None; }
        let last = self.data.len() - 1;
        self.data.swap(i, last);
        let v = self.data.pop();
        if i < self.data.len() {
            self.sift_down(i);
            self.sift_up(i);
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_heap_pops_sorted() {
        let mut h: Heap<i64> = New(|a, b| a < b);
        for v in [3i64, 1, 4, 1, 5, 9, 2, 6] { h.Push(v); }
        let mut out = Vec::new();
        while let Some(v) = h.Pop() { out.push(v); }
        assert_eq!(out, vec![1, 1, 2, 3, 4, 5, 6, 9]);
    }

    #[test]
    fn max_heap_via_comparator() {
        let mut h: Heap<i64> = New(|a, b| a > b);
        for v in [3i64, 1, 4, 1, 5] { h.Push(v); }
        assert_eq!(h.Pop(), Some(5));
        assert_eq!(h.Pop(), Some(4));
    }

    #[test]
    fn init_builds_heap_from_data() {
        let mut h: Heap<i64> = New(|a, b| a < b);
        h.Push(5); h.Push(4); h.Push(3);
        h.Init();
        assert_eq!(h.Pop(), Some(3));
    }

    #[test]
    fn peek_does_not_consume() {
        let mut h: Heap<i64> = New(|a, b| a < b);
        h.Push(2); h.Push(1);
        assert_eq!(h.Peek(), Some(&1));
        assert_eq!(h.Len(), 2);
    }

    #[test]
    fn remove_at_index() {
        let mut h: Heap<i64> = New(|a, b| a < b);
        h.Push(1); h.Push(2); h.Push(3);
        assert_eq!(h.Remove(0), Some(1));
        assert_eq!(h.Pop(), Some(2));
    }
}
