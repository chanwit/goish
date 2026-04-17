package demo

import (
	"sync"
)

// #29 — variadic function.
func NewMultiWriteSyncer(ws ...string) string {
	return "multi"
}

// #35 — embedded field.
type Lock struct {
	*sync.Mutex
	name string
}

// #36 — interface{} field.
type Envelope struct {
	tag  string
	data interface{}
}

// #34 — inline defer+recover.
func protect() {
	defer func() {
		if r := recover(); r != nil {
			_ = r
		}
	}()
	doPanickyThing()
}

func doPanickyThing() {}

// Variadic call site — should auto-wrap args in &[...].
func useMulti() string {
	return NewMultiWriteSyncer("a", "b", "c")
}
