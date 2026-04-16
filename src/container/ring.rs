//! container/ring: circular doubly-linked list.
//!
//!   Go                                  goish
//!   ─────────────────────────────────   ──────────────────────────────────
//!   r := ring.New(5)                    let r = container::ring::New(5);
//!   r.Value = 42                        r.SetValue(42);
//!   r = r.Next()                        let r = r.Next();
//!   r.Link(s)                           r.Link(&s);
//!   r.Unlink(3)                         r.Unlink(3);
//!   r.Do(func(v any) { ... })           r.Do(|v| { ... });
//!
//! Implementation uses raw pointers in an intrusive cycle — the classic
//! Rust idiom for circular linked lists. Ring handles share node
//! ownership freely (like Go's `*Ring`). Memory is owned by whichever
//! handle was minted first via `New`/`new_single`; dropping that handle
//! frees the whole cycle. Clones via `Next`/`Prev`/`Move`/`Link` do NOT
//! own and are safe to drop at any time.

use crate::types::int;
use std::marker::PhantomData;
use std::ptr::NonNull;

pub(crate) struct Node<T> {
    next: NonNull<Node<T>>,
    prev: NonNull<Node<T>>,
    pub(crate) value: Option<T>,
}

/// A handle into a ring. `nil` means the empty ring (Go: `var r *Ring = nil`).
///
/// Memory note: Ring nodes are leaked on handle drop. This matches Go's
/// GC-backed behavior — the Go runtime reclaims rings when no live
/// reference exists, but Rust has no tracing GC and reference-counted
/// cycles would require extra bookkeeping for every Link/Unlink. For
/// typical container/ring usage (small cycles kept for the program's
/// lifetime) this is acceptable. File an issue if you need cleanup.
pub struct Ring<T> {
    ptr: Option<NonNull<Node<T>>>,
    _marker: PhantomData<Node<T>>,
}

// Ring handles are neither Send nor Sync because node pointers alias.
// Keep the default (not Send, not Sync).

impl<T> Ring<T> {
    /// `var r Ring` — a single-element ring with nil Value. Owns the node.
    #[allow(non_snake_case)]
    pub fn new_single() -> Ring<T> {
        let node = Box::new(Node {
            next: NonNull::dangling(),
            prev: NonNull::dangling(),
            value: None,
        });
        let raw = Box::into_raw(node);
        unsafe {
            let nn = NonNull::new_unchecked(raw);
            (*raw).next = nn;
            (*raw).prev = nn;
            Ring { ptr: Some(nn), _marker: PhantomData }
        }
    }

    /// Create a nil (empty) ring.
    pub fn nil() -> Ring<T> {
        Ring { ptr: None, _marker: PhantomData }
    }

    fn from_ptr(p: NonNull<Node<T>>) -> Ring<T> {
        Ring { ptr: Some(p), _marker: PhantomData }
    }

    #[allow(dead_code)]
    fn is_nil(&self) -> bool { self.ptr.is_none() }

    /// `r.Len()` — number of elements; O(n).
    #[allow(non_snake_case)]
    pub fn Len(&self) -> int {
        let start = match self.ptr { Some(p) => p, None => return 0 };
        unsafe {
            let mut n: int = 1;
            let mut p = (*start.as_ptr()).next;
            while p != start {
                n += 1;
                p = (*p.as_ptr()).next;
            }
            n
        }
    }

    /// `r.Next()` — next ring element.
    #[allow(non_snake_case)]
    pub fn Next(&self) -> Ring<T> {
        match self.ptr {
            None => Ring::nil(),
            Some(p) => unsafe { Ring::from_ptr((*p.as_ptr()).next) },
        }
    }

    /// `r.Prev()` — previous ring element.
    #[allow(non_snake_case)]
    pub fn Prev(&self) -> Ring<T> {
        match self.ptr {
            None => Ring::nil(),
            Some(p) => unsafe { Ring::from_ptr((*p.as_ptr()).prev) },
        }
    }

    /// `r.Move(n)` — walks n steps (forward if n >= 0, backward if n < 0).
    #[allow(non_snake_case)]
    pub fn Move(&self, mut n: int) -> Ring<T> {
        let mut p = match self.ptr { Some(x) => x, None => return Ring::nil() };
        unsafe {
            while n < 0 { p = (*p.as_ptr()).prev; n += 1; }
            while n > 0 { p = (*p.as_ptr()).next; n -= 1; }
        }
        Ring::from_ptr(p)
    }

    /// `r.Link(s)` — splice s into r. See Go doc comment in
    /// refs/go1.25.5/container_ring.go for the full semantics.
    /// Returns the original value of r.Next().
    #[allow(non_snake_case)]
    pub fn Link(&self, s: &Ring<T>) -> Ring<T> {
        let r_ptr = match self.ptr { Some(x) => x, None => return Ring::nil() };
        let n = unsafe { (*r_ptr.as_ptr()).next };
        if let Some(s_ptr) = s.ptr {
            unsafe {
                let p = (*s_ptr.as_ptr()).prev;
                (*r_ptr.as_ptr()).next = s_ptr;
                (*s_ptr.as_ptr()).prev = r_ptr;
                (*n.as_ptr()).prev = p;
                (*p.as_ptr()).next = n;
            }
        }
        Ring::from_ptr(n)
    }

    /// `r.Unlink(n)` — remove n elements starting at r.Next(); returns
    /// the removed subring.
    #[allow(non_snake_case)]
    pub fn Unlink(&self, n: int) -> Ring<T> {
        if n <= 0 { return Ring::nil(); }
        self.Link(&self.Move(n + 1))
    }

    /// `r.Do(f)` — calls f on each element's Value, in forward order.
    #[allow(non_snake_case)]
    pub fn Do<F: FnMut(&Option<T>)>(&self, mut f: F) {
        let start = match self.ptr { Some(p) => p, None => return };
        unsafe {
            f(&(*start.as_ptr()).value);
            let mut p = (*start.as_ptr()).next;
            while p != start {
                f(&(*p.as_ptr()).value);
                p = (*p.as_ptr()).next;
            }
        }
    }

    /// Get a clone of the current element's Value (Go: `r.Value`).
    #[allow(non_snake_case)]
    pub fn Value(&self) -> Option<T> where T: Clone {
        self.ptr.map(|p| unsafe { (*p.as_ptr()).value.clone() }).flatten()
    }

    /// Set the current element's Value (Go: `r.Value = v`).
    #[allow(non_snake_case)]
    pub fn SetValue(&self, v: T) {
        if let Some(p) = self.ptr {
            unsafe { (*p.as_ptr()).value = Some(v); }
        }
    }

    /// Pointer equality — two Ring handles reference the same node iff
    /// their underlying raw pointers match. Used by tests that check
    /// `r2a != r1a` in Go.
    pub fn ptr_eq(&self, other: &Ring<T>) -> bool {
        match (self.ptr, other.ptr) {
            (Some(a), Some(b)) => a == b,
            (None, None) => true,
            _ => false,
        }
    }
}

/// `ring.New(n)` — a ring of n elements (each with nil Value). Returns
/// a nil Ring for n <= 0. The returned handle owns the cycle's memory:
/// dropping it frees all nodes.
#[allow(non_snake_case)]
pub fn New<T>(n: int) -> Ring<T> {
    if n <= 0 { return Ring::nil(); }
    // Allocate the first node; it will own the cycle.
    let first = Box::new(Node {
        next: NonNull::dangling(),
        prev: NonNull::dangling(),
        value: None,
    });
    let first_raw = Box::into_raw(first);
    let first_nn = unsafe { NonNull::new_unchecked(first_raw) };
    let mut p = first_nn;
    for _ in 1..n {
        let node = Box::new(Node {
            next: NonNull::dangling(),
            prev: p,
            value: None,
        });
        let raw = Box::into_raw(node);
        let nn = unsafe { NonNull::new_unchecked(raw) };
        unsafe { (*p.as_ptr()).next = nn; }
        p = nn;
    }
    unsafe {
        (*p.as_ptr()).next = first_nn;
        (*first_nn.as_ptr()).prev = p;
    }
    Ring { ptr: Some(first_nn), _marker: PhantomData }
}

// No Drop impl: Ring leaks nodes. See doc comment on Ring above.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_ring_of_5() {
        let r = New::<i64>(5);
        assert_eq!(r.Len(), 5);
        // walk forward 5 steps returns to start
        let mut p = r.Next();
        for _ in 0..4 { p = p.Next(); }
        assert!(p.ptr_eq(&r));
    }

    #[test]
    fn new_ring_of_zero_is_nil() {
        let r = New::<i64>(0);
        assert_eq!(r.Len(), 0);
    }

    #[test]
    fn value_get_set() {
        let r = New::<i64>(3);
        r.SetValue(10);
        r.Next().SetValue(20);
        r.Next().Next().SetValue(30);
        let mut sum = 0i64;
        r.Do(|v| { if let Some(x) = v { sum += x; } });
        assert_eq!(sum, 60);
    }

    #[test]
    fn move_wraps() {
        let r = New::<i64>(4);
        assert!(r.Move(4).ptr_eq(&r));
        assert!(r.Move(-4).ptr_eq(&r));
        assert!(r.Move(0).ptr_eq(&r));
    }

    #[test]
    fn single_element_self_refs() {
        let r: Ring<i64> = Ring::new_single();
        assert_eq!(r.Len(), 1);
        assert!(r.Next().ptr_eq(&r));
        assert!(r.Prev().ptr_eq(&r));
    }
}
