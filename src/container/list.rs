//! container/list: doubly linked list — value-based. `Element` is just an
//! index handle.
//!
//!   Go                                  goish
//!   ─────────────────────────────────   ──────────────────────────────────
//!   l := list.New()                     let mut l = container::list::New::<int>();
//!   e := l.PushBack(1)                  let e = l.PushBack(1);
//!   l.PushFront(0)                      l.PushFront(0);
//!   for e := l.Front(); …               for e in l.Iter() { … }
//!   l.Remove(e)                         l.Remove(e);

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
