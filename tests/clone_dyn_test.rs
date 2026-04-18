// Coverage for v0.21.0 `Interface!` macro (frictions #43, #46 from v0.20.7):
//   - Go-shape interface decl: `type Core interface { … }`
//   - `.into()` lifting a concrete impl into the interface newtype
//   - chainable With-style method returning the interface type
//   - transparent clone of interface-typed values (no dyn-trait leak)
//
// NO `Box<dyn …>`, `clone_trait_object!`, or `DynClone` at the call
// site. If any of those reappears here, it's a Rust-leak regression.

use goish::prelude::*;
use std::sync::{Arc, Mutex};

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
    sink: Arc<Mutex<Vec<String>>>,
}

Interface!{
    impl Core for InMem {
        fn Write(&self, msg: &str) {
            let line = format!("[{}] {}", self.tags.join("/"), msg);
            self.sink.lock().unwrap().push(line);
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
    let sink: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
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

    let logged = sink.lock().unwrap().clone();
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
    let sink: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let base: Core = InMem { tags: vec!["root"], sink }.into();
    let child = base.With("leaf");

    if len!(base.Tags()) != 1 {
        t.Errorf("With mutated the parent's tags".to_string());
    }
    if len!(child.Tags()) != 2 {
        t.Errorf("With didn't extend tags on the child".to_string());
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
struct ThresholdCore { min_lvl: i32, sink: Arc<Mutex<Vec<String>>> }

Interface!{
    impl LevelEnabler for ThresholdCore {
        fn Enabled(&self, lvl: i32) -> bool { lvl >= self.min_lvl }
    }
}

Interface!{
    impl TraceCore for ThresholdCore {
        fn Emit(&self, lvl: i32, msg: &str) {
            if self.Enabled(lvl) {
                self.sink.lock().unwrap().push(format!("L{}:{}", lvl, msg));
            }
        }
    }
}

test!{ fn TestInterface_SupertraitBound(t) {
    let sink: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let tc: TraceCore = ThresholdCore { min_lvl: 1, sink: sink.clone() }.into();

    tc.Emit(0, "below");
    tc.Emit(1, "at");
    tc.Emit(2, "above");

    let out = sink.lock().unwrap().clone();
    if len!(out) != 2 {
        t.Errorf(Sprintf!("supertrait filtering: want 2 lines, got %d", len!(out)));
    }
    if out.get(0).map(|s| s.as_str()) != Some("L1:at") {
        t.Errorf(Sprintf!("first line wrong: %v", out));
    }
}}
