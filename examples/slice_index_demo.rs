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
    println!("rand: {}", rand_string(12));
    println!("done");
}
