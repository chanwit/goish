package path_test

import (
	"path"
	"testing"
)

type PathTest struct {
	path, want string
}

var tests = []PathTest{
	{"", "."},
	{"abc", "abc"},
	{"a/b/c", "a/b/c"},
}

func TestClean(t *testing.T) {
	for _, tc := range tests {
		got := path.Clean(tc.path)
		if got != tc.want {
			t.Errorf("Clean(%q) = %q, want %q", tc.path, got, tc.want)
		}
	}
}

func BenchmarkJoin(b *testing.B) {
	b.ReportAllocs()
	for i := 0; i < b.N; i++ {
		path.Join("a", "b", "c")
	}
}
