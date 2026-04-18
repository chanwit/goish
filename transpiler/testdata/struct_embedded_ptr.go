package main

// Plain backing type — something that satisfies Struct!'s derives (Clone,
// Debug, Default, PartialEq, Eq, Hash) so the embedding test can focus on
// the emission shape rather than on derive satisfaction.
type Inner struct {
	id int
}

// Bare embedded pointer — exercises the `Inner: &Inner` → `Inner Inner` fix
// (strip leading `*`, drop the colon separator).
type Guard struct {
	*Inner
}

// Mixed: embedded *T plus regular named fields, to confirm the field-separator
// and the pointer-strip interact correctly when there's more than one group.
type Resource struct {
	*Inner
	name  string
	count int
}

func main() {
	var g Guard
	var r Resource
	_ = g
	_ = r
}
