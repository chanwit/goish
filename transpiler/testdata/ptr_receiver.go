package main

// Regression fixture for #63: Go pointer receivers (`*T`) default to
// `&self` in the emitted output; only methods that actually mutate the
// receiver get promoted to `&mut self`. Drives the flow-analysis in
// `receiverIsMutated`.

type Counter struct {
	n int
}

// Read-only method on *Counter — should emit as `&self`.
func (c *Counter) Read() int {
	return c.n
}

// Mutating method on *Counter — should emit as `&mut self`.
func (c *Counter) Bump() {
	c.n = c.n + 1
}

// Compound-assign mutator — also `&mut self`.
func (c *Counter) Add(x int) {
	c.n += x
}

// Increment-statement mutator — also `&mut self`.
func (c *Counter) Inc() {
	c.n++
}

// Value receiver (no `*`) — always `&self` regardless of body.
func (c Counter) Peek() int {
	return c.n
}

func main() {
	var c Counter
	_ = c.Read()
	c.Bump()
	c.Add(3)
	c.Inc()
	_ = c.Peek()
}
