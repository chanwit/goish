package main

// Multi-field struct regression — guards against the comma-vs-semicolon
// separator bug in emitStructFields that previously made any transpiled
// `Struct!` with 2+ field groups fail to compile.
type Server struct {
	host string
	port int
}

// Same-type multi-name group mixed with other groups — exercises both
// intra-group `,` (name separator) and inter-group `;` (field separator).
type Point struct {
	x, y int
	name string
}

func main() {
	// Zero-value declarations are enough to force the struct decls to
	// be type-checked; avoid string-literal construction here because
	// Go's `"foo"` → Rust `&str` vs Goish `GoString` is a separate
	// call-site friction, orthogonal to this regression.
	var s Server
	var p Point
	_ = s
	_ = p
}
