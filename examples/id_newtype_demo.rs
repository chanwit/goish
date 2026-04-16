// Go:
//   type ID uint64
//   func (i ID) String() string { return strconv.FormatUint(uint64(i), 16) }
//   type IDSlice []ID
//   func (p IDSlice) Len() int           { return len(p) }
//   func (p IDSlice) Less(i, j int) bool { return p[i] < p[j] }
//   func (p IDSlice) Swap(i, j int)      { p[i], p[j] = p[j], p[i] }
//   func (p IDSlice) String() string     { /* comma-join */ }
//
// Goish port — IntNewtype!, stringer!, and sort::Interface combined.

use goish::prelude::*;

// v0.17.7: `Type!(...)` — Go's `type` keyword, dispatches on RHS shape.
//   Type!(ID = uint64)     → int newtype
//   Type!(IDSlice = []ID)  → slice newtype
Type!(ID = uint64);
stringer! {
    impl ID {
        fn String(&self) -> string { strconv::FormatUint(self.0, 16) }
    }
}

Type!(IDSlice = []ID);

impl sort::Interface for IDSlice {
    fn Len(&self) -> int { self.0.len() as int }
    fn Less(&self, i: int, j: int) -> bool { self.0[i].0 < self.0[j].0 }
    fn Swap(&mut self, i: int, j: int) { self.0.Swap(i, j); }
}

// v0.17.6: sort::Interface is in the prelude, so `self.Len()` resolves
// to the trait method here — no explicit `use goish::sort::Interface`.
stringer! {
    impl IDSlice {
        fn String(&self) -> string {
            let mut b = strings::Builder::default();
            if self.Len() > 0 {
                let _ = b.WriteString(self.0[0i64].String());
            }
            for i in 1..self.Len() {
                let _ = b.WriteString(",");
                let _ = b.WriteString(self.0[i].String());
            }
            b.String()
        }
    }
}

fn main() {
    // Go: IDSlice{10, 1, 255, 16} — now works in goish thanks to IntNewtype's
    // From<i32> / From<i64> impls on ID.
    let mut ids = IDSlice(slice!([]ID{10, 1, 255, 16}));
    sort::Sort(&mut ids);
    // Sprintf!("%v", ids) goes through Display — provided by stringer!.
    Println!(Sprintf!("%v", ids));
    // Expect: 1,a,10,ff  (hex of 1, 10, 16, 255 after sort)
}
