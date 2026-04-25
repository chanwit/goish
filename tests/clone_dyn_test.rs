// Coverage for v0.21.0 `Interface!` macro (frictions #43, #46 from v0.20.7):
//   - Go-shape interface decl: `type Core interface { … }`
//   - `.into()` lifting a concrete impl into the interface newtype
//   - chainable With-style method returning the interface type
//   - transparent clone of interface-typed values (no dyn-trait leak)
//
// NO `Box<dyn …>`, `clone_trait_object!`, or `DynClone` at the call
// site. If any of those reappears here, it's a Rust-leak regression.

use goish::prelude::*;

// zapcore-shape `Core` interface — With returns a fresh Core, the call
// site needs to clone / share it freely without seeing Rust plumbing.
Interface!{
    type Core interface {
        fn Write(&self, msg: &str);
        fn With(&self, tag: &'static str) -> Core;
        fn Tags(&self) -> Vec<&'static str>;
    }
}

#[derive(Clone)]
struct InMem {
    tags: Vec<&'static str>,
    sink: sync::Mutex<slice<string>>,
}

Interface!{
    impl Core for InMem {
        fn Write(&self, msg: &str) {
            let line = Sprintf!("[%v] %v", self.tags.join("/"), msg);
            self.sink.Lock().push(line);
        }
        fn With(&self, tag: &'static str) -> Core {
            let mut tags = self.tags.clone();
            tags.push(tag);
            InMem { tags, sink: self.sink.clone() }.into()
        }
        fn Tags(&self) -> Vec<&'static str> { self.tags.clone() }
    }
}

test!{ fn TestCore_Clones(t) {
    let sink: sync::Mutex<slice<string>> = sync::Mutex::new(slice::new());
    let base: Core = InMem {
        tags: vec!["base"],
        sink: sink.clone(),
    }.into();

    let a = base.clone();                // #43: clone a Core value
    let b = a.With("worker");            // #46: With returns Core
    let c = b.clone();                   // #43 again — on the chained result

    a.Write("A");
    b.Write("B");
    c.Write("C");

    let logged = sink.Lock().clone();
    if len!(logged) != 3 {
        t.Errorf(Sprintf!("want 3 lines, got %d", len!(logged)));
    }
    let text = logged.join("\n");
    if !strings::Contains(&text, "[base] A") {
        t.Errorf(Sprintf!("missing base line: %s", text));
    }
    if !strings::Contains(&text, "[base/worker] B") {
        t.Errorf(Sprintf!("missing worker line from .With(): %s", text));
    }
    if !strings::Contains(&text, "[base/worker] C") {
        t.Errorf(Sprintf!("missing clone-of-worker line: %s", text));
    }
}}

test!{ fn TestCore_WithDoesntMutateOriginal(t) {
    let sink: sync::Mutex<slice<string>> = sync::Mutex::new(slice::new());
    let base: Core = InMem { tags: vec!["root"], sink }.into();
    let child = base.With("leaf");

    if len!(base.Tags()) != 1 {
        t.Errorf("With mutated the parent's tags");
    }
    if len!(child.Tags()) != 2 {
        t.Errorf("With didn't extend tags on the child");
    }
}}

// Supertrait clause — `type Core: LevelEnabler interface { … }`.
// Bound-only semantics in v0.21.0: any impl of Core must also impl
// LevelEnabler. We don't forward parent methods onto Core values.

Interface!{
    type LevelEnabler interface {
        fn Enabled(&self, lvl: i32) -> bool;
    }
}

Interface!{
    type TraceCore: LevelEnabler interface {
        fn Emit(&self, lvl: i32, msg: &str);
    }
}

#[derive(Clone)]
struct ThresholdCore { min_lvl: i32, sink: sync::Mutex<slice<string>> }

Interface!{
    impl LevelEnabler for ThresholdCore {
        fn Enabled(&self, lvl: i32) -> bool { lvl >= self.min_lvl }
    }
}

Interface!{
    impl TraceCore for ThresholdCore {
        fn Emit(&self, lvl: i32, msg: &str) {
            if self.Enabled(lvl) {
                self.sink.Lock().push(Sprintf!("L%v:%v", lvl, msg));
            }
        }
    }
}

test!{ fn TestInterface_SupertraitBound(t) {
    let sink: sync::Mutex<slice<string>> = sync::Mutex::new(slice::new());
    let tc: TraceCore = ThresholdCore { min_lvl: 1, sink: sink.clone() }.into();

    tc.Emit(0, "below");
    tc.Emit(1, "at");
    tc.Emit(2, "above");

    let out = sink.Lock().clone();
    if len!(out) != 2 {
        t.Errorf(Sprintf!("supertrait filtering: want 2 lines, got %d", len!(out)));
    }
    if out.get(0).map(|s| s.as_str()) != Some("L1:at") {
        t.Errorf(Sprintf!("first line wrong: %v", out));
    }
}}

// Friction #58 regression: Interface! declared in one module, impl'd in
// another. The impl arm accepts the qualified path `decl_mod::Logger`,
// from which the macro derives `decl_mod::__LoggerTrait`. The outer
// module (test body) never names `__LoggerTrait`.
mod decl_mod {
    use super::*;
    Interface!{
        type Logger interface {
            fn Log(&self, msg: &str);
        }
    }
}

mod impl_mod {
    use super::*;
    #[derive(Clone)]
    pub struct Capture { pub sink: sync::Mutex<slice<string>> }

    // Path form — user never types `__LoggerTrait`.
    Interface!{
        impl super::decl_mod::Logger for Capture {
            fn Log(&self, msg: &str) {
                self.sink.Lock().push(msg.into());
            }
        }
    }
}

test!{ fn TestInterface_CrossModuleImpl(t) {
    let sink: sync::Mutex<slice<string>> = sync::Mutex::new(slice::new());
    let lg: decl_mod::Logger = impl_mod::Capture { sink: sink.clone() }.into();
    lg.Log("hello");
    lg.Log("world");

    let out = sink.Lock().clone();
    if len!(out) != 2 {
        t.Errorf(Sprintf!("cross-module Interface! impl: want 2 lines, got %d", len!(out)));
    }
}}
