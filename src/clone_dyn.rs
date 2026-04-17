// clone_dyn: enable `Clone` on `Box<dyn Trait>` objects.
//
// Rust's `Clone` trait has `fn clone(&self) -> Self` — that `Self` return
// makes it non-object-safe, so `dyn Trait: Clone` is forbidden. This
// blocks the Go pattern of returning a boxed interface from a
// With-style method that needs to make a fresh copy:
//
//   // Go: type Core interface { With(fields []Field) Core }
//   // Goish:
//   pub trait Core: DynClone {
//       fn With(&self, fields: slice<Field>) -> Box<dyn Core>;
//   }
//   clone_trait_object!(Core);
//
//   let a: Box<dyn Core> = ...;
//   let b = a.clone();              // works — dispatched via the fat
//                                    // pointer's vtable.
//
// Technique is the same as the `dyn-clone` crate: hidden `__clone_box`
// method returns a raw pointer to a heap-allocated clone (no vtable),
// and the macro's `Clone for Box<dyn Trait>` splices that data pointer
// into the existing fat pointer. No unsafe at the user's call site.

/// Supertrait that lets a trait object be cloned. Any type that impls
/// `Clone` automatically impls `DynClone` via a blanket impl.
pub trait DynClone {
    #[doc(hidden)]
    fn __clone_box(&self) -> *mut ();
}

impl<T: Clone + 'static> DynClone for T {
    fn __clone_box(&self) -> *mut () {
        Box::into_raw(Box::<T>::new(self.clone())) as *mut ()
    }
}

/// Emits `impl Clone for Box<dyn Trait>` for the given trait. The trait
/// must have `DynClone` as a supertrait.
///
/// Usage:
///
///     pub trait MyTrait: goish::DynClone { /* ... */ }
///     goish::clone_trait_object!(MyTrait);
///
/// Now `Box<dyn MyTrait>` implements `Clone`.
#[macro_export]
macro_rules! clone_trait_object {
    ($trait:path) => {
        impl ::std::clone::Clone for ::std::boxed::Box<dyn $trait> {
            fn clone(&self) -> Self {
                let mut fat_ptr: *const (dyn $trait) = &**self;
                unsafe {
                    // Overwrite the data portion of the fat pointer with a
                    // freshly heap-allocated clone of the pointee; keep the
                    // vtable portion intact.
                    let data_ptr = &mut fat_ptr as *mut *const (dyn $trait) as *mut *mut ();
                    *data_ptr = $crate::DynClone::__clone_box(&**self);
                    ::std::boxed::Box::from_raw(fat_ptr as *mut (dyn $trait))
                }
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    // A zapcore-shaped `Core` trait to mirror the failing port.
    trait Core: DynClone + Send + Sync {
        fn tag(&self) -> &'static str;
        fn count(&self) -> i64;
    }
    crate::clone_trait_object!(Core);

    #[derive(Clone)]
    struct Real { tag: &'static str, n: i64 }
    impl Core for Real {
        fn tag(&self) -> &'static str { self.tag }
        fn count(&self) -> i64 { self.n }
    }

    #[test]
    fn box_dyn_core_is_clonable() {
        let a: Box<dyn Core> = Box::new(Real { tag: "first", n: 42 });
        let b: Box<dyn Core> = a.clone();
        assert_eq!(a.tag(), b.tag());
        assert_eq!(a.count(), b.count());
    }

    #[test]
    fn clones_are_distinct_allocations() {
        let a: Box<dyn Core> = Box::new(Real { tag: "x", n: 7 });
        let b: Box<dyn Core> = a.clone();
        let a_ptr = Box::as_ref(&a) as *const dyn Core as *const ();
        let b_ptr = Box::as_ref(&b) as *const dyn Core as *const ();
        assert_ne!(a_ptr, b_ptr, "clone should allocate a fresh Box");
        assert_eq!(a.count(), b.count());
    }

    // With-style composition: the motivating zapcore pattern.
    #[test]
    fn with_returns_clonable_boxed_trait() {
        trait Bumpable: DynClone {
            fn n(&self) -> i64;
            fn bump(&self) -> Box<dyn Bumpable>;
        }
        crate::clone_trait_object!(Bumpable);
        #[derive(Clone)]
        struct Impl(i64);
        impl Bumpable for Impl {
            fn n(&self) -> i64 { self.0 }
            fn bump(&self) -> Box<dyn Bumpable> { Box::new(Impl(self.0 + 1)) }
        }
        let a: Box<dyn Bumpable> = Box::new(Impl(1));
        let b = a.bump();       // returns Box<dyn Bumpable>
        let c = b.clone();      // THIS is what #43/#46 needed
        assert_eq!(a.n(), 1);
        assert_eq!(b.n(), 2);
        assert_eq!(c.n(), 2);
    }
}
