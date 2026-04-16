// Demonstrates v0.17.0 slice<T> Index<i64>: `ss[i]` where i: int (i64)
// works without `as usize`.

use goish::prelude::*;

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
    println!("done");
}
