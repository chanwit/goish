package main

import "fmt"

// Plain interface — no supertrait.
type Writer interface {
	Write(msg string)
	Tag() string
}

// Interface with embedded Go-local interface → supertrait clause (bare ident).
type LevelEnabler interface {
	Enabled(lvl int) bool
}

type TraceCore interface {
	LevelEnabler
	Emit(lvl int, msg string)
}

// Interface with embedded dotted-path interface → supertrait clause (path form).
// Uses `fmt.Stringer` (a real Goish trait at `fmt::Stringer`) — the transpiler
// maps any SelectorExpr to `pkg::Name` and the Interface! macro passes the
// path verbatim.
type PrintableCloser interface {
	fmt.Stringer
	Close() error
}

// Interface with embedded inline interface literal → should fall through to
// the existing TODO, since it's not a supertrait shape.
type Weird interface {
	interface{ X() int }
	Y() string
}

func main() {}
