package main

import "fmt"

// Struct that has an Error() string method → should emit as ErrorType!.
type MultiError struct {
	errs []error
}

func (m *MultiError) Error() string {
	return fmt.Sprintf("multi error: %d", len(m.errs))
}

// Non-Error method on the same type → stays as a separate impl block.
// Returns a count (int) to dodge the unrelated "move out of &mut" friction
// that a `[]error` return would hit; the point here is that the method
// survives outside the `ErrorType!` macro, not the body shape.
func (m *MultiError) Count() int {
	return len(m.errs)
}

// Plain struct with no Error() method → should emit as Struct! (unchanged).
// Single-field — multi-field Struct! emission has a pre-existing separator
// bug (comma-vs-semicolon) flagged in the commit's bonus findings; keep the
// control case but don't trip that bug.
type Config struct {
	name string
}

// Struct with a value-receiver Error() — also counts as error type.
type SimpleError struct {
	msg string
}

func (s SimpleError) Error() string {
	return fmt.Sprintf("%s", s.msg)
}

func main() {}
