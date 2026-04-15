// container: Go's container/list and container/heap packages.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   l := list.New()                     let mut l = container::list::New::<int>();
//   e := l.PushBack(1)                  let e = l.PushBack(1);
//   l.PushFront(0)                      l.PushFront(0);
//   for e := l.Front(); e != nil; ...   for e in l.Iter() { … }
//   l.Remove(e)                         l.Remove(e);
//
//   h := &IntHeap{2,1,5}                let mut h: container::heap::Heap<i64> = …
//   heap.Init(h)                        h.Init();
//   heap.Push(h, 3)                     h.Push(3);
//   heap.Pop(h)                         h.Pop();

pub mod list {
    //! Doubly linked list — value-based. `Element` is just an index handle.

    use crate::types::int;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Element(usize);

    #[derive(Debug)]
    struct Node<T> {
        value: T,
        prev: Option<usize>,
        next: Option<usize>,
        alive: bool,
    }

    #[derive(Debug)]
    pub struct List<T> {
        nodes: Vec<Node<T>>,
        head: Option<usize>,
        tail: Option<usize>,
        len: int,
    }

    #[allow(non_snake_case)]
    pub fn New<T>() -> List<T> {
        List { nodes: Vec::new(), head: None, tail: None, len: 0 }
    }

    impl<T: Clone> List<T> {
        pub fn Len(&self) -> int { self.len }

        pub fn Front(&self) -> Option<Element> { self.head.map(Element) }
        pub fn Back(&self) -> Option<Element> { self.tail.map(Element) }

        pub fn PushBack(&mut self, value: T) -> Element {
            let id = self.nodes.len();
            self.nodes.push(Node { value, prev: self.tail, next: None, alive: true });
            if let Some(t) = self.tail { self.nodes[t].next = Some(id); }
            else { self.head = Some(id); }
            self.tail = Some(id);
            self.len += 1;
            Element(id)
        }

        pub fn PushFront(&mut self, value: T) -> Element {
            let id = self.nodes.len();
            self.nodes.push(Node { value, prev: None, next: self.head, alive: true });
            if let Some(h) = self.head { self.nodes[h].prev = Some(id); }
            else { self.tail = Some(id); }
            self.head = Some(id);
            self.len += 1;
            Element(id)
        }

        pub fn Remove(&mut self, e: Element) -> Option<T> {
            let id = e.0;
            if id >= self.nodes.len() || !self.nodes[id].alive { return None; }
            let prev = self.nodes[id].prev;
            let next = self.nodes[id].next;
            match prev { Some(p) => self.nodes[p].next = next, None => self.head = next }
            match next { Some(n) => self.nodes[n].prev = prev, None => self.tail = prev }
            self.nodes[id].alive = false;
            self.len -= 1;
            Some(self.nodes[id].value.clone())
        }

        pub fn Value(&self, e: Element) -> Option<&T> {
            self.nodes.get(e.0).filter(|n| n.alive).map(|n| &n.value)
        }

        pub fn Next(&self, e: Element) -> Option<Element> {
            self.nodes.get(e.0).and_then(|n| n.next).map(Element)
        }

        pub fn Prev(&self, e: Element) -> Option<Element> {
            self.nodes.get(e.0).and_then(|n| n.prev).map(Element)
        }

        pub fn Iter(&self) -> ListIter<'_, T> {
            ListIter { list: self, cur: self.head }
        }
    }

    pub struct ListIter<'a, T> {
        list: &'a List<T>,
        cur: Option<usize>,
    }

    impl<'a, T> Iterator for ListIter<'a, T> {
        type Item = &'a T;
        fn next(&mut self) -> Option<&'a T> {
            let id = self.cur?;
            let node = &self.list.nodes[id];
            self.cur = node.next;
            Some(&node.value)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn push_back_and_iterate() {
            let mut l = New::<i64>();
            l.PushBack(1);
            l.PushBack(2);
            l.PushBack(3);
            let vals: Vec<i64> = l.Iter().copied().collect();
            assert_eq!(vals, vec![1, 2, 3]);
            assert_eq!(l.Len(), 3);
        }

        #[test]
        fn push_front_prepends() {
            let mut l = New::<i64>();
            l.PushBack(2);
            l.PushFront(1);
            let vals: Vec<i64> = l.Iter().copied().collect();
            assert_eq!(vals, vec![1, 2]);
        }

        #[test]
        fn remove_element() {
            let mut l = New::<i64>();
            let _ = l.PushBack(1);
            let b = l.PushBack(2);
            let _ = l.PushBack(3);
            let removed = l.Remove(b);
            assert_eq!(removed, Some(2));
            let vals: Vec<i64> = l.Iter().copied().collect();
            assert_eq!(vals, vec![1, 3]);
            assert_eq!(l.Len(), 2);
        }

        #[test]
        fn front_back_navigation() {
            let mut l = New::<i64>();
            l.PushBack(1);
            l.PushBack(2);
            let f = l.Front().unwrap();
            assert_eq!(l.Value(f), Some(&1));
            let n = l.Next(f).unwrap();
            assert_eq!(l.Value(n), Some(&2));
            assert!(l.Next(n).is_none());
        }
    }
}

pub mod heap {
    //! Binary min-heap. Go's container/heap takes an interface; we ship a
    //! concrete `Heap<T>` parameterised on the ordering. `Push/Pop/Peek`
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
            // Push unsorted directly into data via Push; then Init should still hold.
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
}
