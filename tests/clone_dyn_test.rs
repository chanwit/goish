// Coverage for v0.20.7 DynClone (frictions #43, #46):
//   - trait-object `Clone` via the `DynClone` supertrait + `clone_trait_object!`
//   - motivating pattern: zapcore-shape `Core::With` returning `Box<dyn Core>`
//     that the caller can clone and chain.

use goish::prelude::*;
use std::sync::{Arc, Mutex};

// Shape mirroring zapcore.Core — With returns a fresh boxed Core, the
// call site needs to clone/share it freely.
trait Core: DynClone + Send + Sync {
    fn write(&self, msg: &str);
    fn with(&self, tag: &'static str) -> Box<dyn Core>;
    fn tags(&self) -> Vec<&'static str>;
}
clone_trait_object!(Core);

#[derive(Clone)]
struct InMem {
    tags: Vec<&'static str>,
    sink: Arc<Mutex<Vec<String>>>,
}

impl Core for InMem {
    fn write(&self, msg: &str) {
        let line = format!("[{}] {}", self.tags.join("/"), msg);
        self.sink.lock().unwrap().push(line);
    }
    fn with(&self, tag: &'static str) -> Box<dyn Core> {
        let mut tags = self.tags.clone();
        tags.push(tag);
        Box::new(InMem { tags, sink: self.sink.clone() })
    }
    fn tags(&self) -> Vec<&'static str> { self.tags.clone() }
}

test!{ fn TestBoxDynCore_Clones(t) {
    let sink: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let base: Box<dyn Core> = Box::new(InMem {
        tags: vec!["base"],
        sink: sink.clone(),
    });

    let a = base.clone();                // #43: clone a Box<dyn Core>
    let b = a.with("worker");            // #46: With returns Box<dyn Core>
    let c = b.clone();                   // #43 again — on the chained result

    a.write("A");
    b.write("B");
    c.write("C");

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

test!{ fn TestBoxDynCore_WithDoesntMutateOriginal(t) {
    let sink: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let base: Box<dyn Core> = Box::new(InMem {
        tags: vec!["root"],
        sink,
    });
    let child = base.with("leaf");

    if len!(base.tags()) != 1 {
        t.Errorf("With mutated the parent's tags".to_string());
    }
    if len!(child.tags()) != 2 {
        t.Errorf("With didn't extend tags on the child".to_string());
    }
}}
