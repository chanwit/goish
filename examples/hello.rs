// hello: a tour of goish idioms — looks like Go, runs as Rust.
//
//   $ cargo run --example hello

use goish::prelude::*;

fn divide(a: int64, b: int64) -> (int64, error) {
    if b == 0 {
        return (0, errors::New("divide by zero"));
    }
    (a / b, nil)
}

// Go: type Color struct { R, G, B int }
//     func (c Color) String() string { return fmt.Sprintf("#%02x%02x%02x", c.R, c.G, c.B) }
struct Color { r: int, g: int, b: int }

fmt::stringer! {
    impl Color {
        fn String(&self) -> string {
            fmt::Sprintf!("#%02x%02x%02x", self.r, self.g, self.b)
        }
    }
}

fn main() {
    // fmt.Println("hello", "world", 42)
    fmt::Println!("hello", "world", 42);

    // fmt.Printf("%-8s = %d\n", "answer", 42)
    fmt::Printf!("%-8s = %d\n", "answer", 42);

    // s := fmt.Sprintf("pi = %.4f", 3.14159)
    let s = fmt::Sprintf!("pi = %.4f", 3.14159);
    fmt::Println!(s);

    // q, err := divide(10, 2)
    let (q, err) = divide(10, 2);
    if err != nil {
        fmt::Println!("error:", err);
    } else {
        fmt::Printf!("10 / 2 = %d\n", q);
    }

    let (_, err) = divide(10, 0);
    if err != nil {
        fmt::Println!("error:", err);
    }

    // err wrapping & errors.Is
    let base = errors::New("disk full");
    let wrapped = errors::Wrap(base.clone(), "save failed");
    fmt::Println!("wrapped:", wrapped);
    fmt::Println!("is base?", errors::Is(&wrapped, &base));

    // fmt.Errorf
    let e = fmt::Errorf!("connection to %s failed (code %d)", "db.local", 500);
    fmt::Println!("formatted:", e);

    // fmt.Fprintf into a bytes.Buffer
    let mut buf = bytes::Buffer::new();
    fmt::Fprintf!(&mut buf, "logged: %s=%d", "n", 7);
    fmt::Println!("buf:", buf.String());

    // Stringer — Println & %s both pick up Color.String()
    let red = Color { r: 255, g: 0, b: 0 };
    let teal = Color { r: 0, g: 128, b: 128 };
    fmt::Println!("color:", red);
    fmt::Printf!("teal as %%s = %s\n", teal);
}
