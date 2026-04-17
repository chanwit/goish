// Demonstrates v0.17 Go-shape indexing: `ss[i]` on slice<T> and `p[i]`
// on string — both with i: int (i64), no `as usize` casts.

use goish::prelude::*;

fn rand_string(l: int) -> string {
    // v0.17.2: string!("...") — cached, indexable with Go's int (i64).
    let chars = string!("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789");
    let mut s: slice<byte> = make!([]byte, l);
    for i in 0..l {
        s[i] = chars[rand::Intn(len!(chars))];
    }
    string::from(s)
}

// v0.17.3: custom type via sort::Interface — Go's `sort.Sort(x)` pattern.
struct Uint64Slice(slice<uint64>);
impl sort::Interface for Uint64Slice {
    fn Len(&self) -> int { self.0.len() as int }
    fn Less(&self, i: int, j: int) -> bool { self.0[i] < self.0[j] }
    fn Swap(&mut self, i: int, j: int) { self.0.Swap(i, j); }
}

fn main() {
    let mut ss: slice<string> = slice!([]string{"banana", "apple", "cherry", "date", "banana"});
    sort::Strings(&mut ss);
    for i in 1..len!(ss) {
        if ss[i - 1] == ss[i] {
            println!("dup: ss[i-1]={} == ss[i]={}", ss[i - 1], ss[i]);
        }
    }
    let p: string = "hello".into();
    assert_eq!(p[0], b'h');

    // v0.17.3: bare numeric literals in slice!([]uint64{...}) — no `u64` suffix.
    let mut g = Uint64Slice(slice!([]uint64{10, 500, 5, 1, 100, 25}));
    sort::Sort(&mut g);
    println!("sorted: {:?}", g.0.as_slice());

    println!("rand: {}", rand_string(12));
    println!("done");
}
