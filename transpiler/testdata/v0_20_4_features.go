package demo

import (
	"errors"
	"reflect"
	"runtime"

	"go.uber.org/multierr"
)

// #31 — runtime.Goexit should map directly to runtime::Goexit().
func cleanup() {
	runtime.Goexit()
}

// #33 — multierr.Append → errors::Append.
func collect(first, second error) error {
	return multierr.Append(first, second)
}

// #32 — nil-pointer-in-interface guard is meaningless in Goish (error is a
// single newtype; typed-nil doesn't exist). Collapse to `e == nil`.
func isNilErr(e error) bool {
	return reflect.ValueOf(e).IsNil()
}

// Broader reflect use — should still be flagged as a TODO.
func kindCheck(e error) bool {
	return reflect.ValueOf(e).Kind() == reflect.Ptr
}

func main() {
	_ = cleanup
	_ = collect(errors.New("a"), errors.New("b"))
	_ = isNilErr
	_ = kindCheck
}
