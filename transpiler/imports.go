package main

// importMap — Go stdlib package path → Goish Rust module path (`::`-form).
// Populated from REFERENCES.md §21–§24.
var importMap = map[string]string{
	"fmt":             "fmt",
	"errors":          "errors",
	"strings":         "strings",
	"strconv":         "strconv",
	"bytes":           "bytes",
	"bufio":           "bufio",
	"io":              "io",
	"io/ioutil":       "io",
	"os":              "os",
	"os/exec":         "exec",
	"sort":            "sort",
	"time":            "time",
	"sync":            "sync",
	"sync/atomic":     "sync::atomic",
	"context":         "context",
	"regexp":          "regexp",
	"math":            "math",
	"math/rand":       "rand",
	"log":             "log",
	"flag":            "flag",
	"cmp":             "cmp",
	"slices":          "slices",
	"maps":            "maps",
	"iter":            "iter",
	"unicode":         "unicode",
	"unicode/utf8":    "utf8",
	"html":            "html",
	"encoding/json":   "json",
	"encoding/base64": "base64",
	"encoding/hex":    "hex",
	"encoding/binary": "binary",
	"encoding/csv":    "csv",
	"net/http":        "http",
	"net/url":         "url",
	"net/mail":        "mail",
	"net/smtp":        "smtp",
	"net/netip":       "netip",
	"path":            "path",
	"path/filepath":   "filepath",
	"container/list":  "container::list",
	"container/heap":  "container::heap",
	"container/ring":  "container::ring",
	"crypto/md5":      "md5",
	"crypto/sha1":     "sha1",
	"crypto/sha256":   "sha256",
	"hash/crc32":      "crc32",
	"hash/fnv":        "fnv",
	"text/tabwriter":  "tabwriter",
	"text/scanner":    "scanner",
	"text/template":   "template",
	"testing":         "testing",
	"runtime":         "runtime",
	"mime/multipart":  "multipart",
}

// resolveImport returns (goishPath, ok). ok=false means "complex / unknown" —
// emit a TODO comment prompting manual review.
func resolveImport(goPath string) (string, bool) {
	if p, ok := importMap[goPath]; ok {
		return p, true
	}
	return goPath, false
}

// primitiveTypeMap — Go → Goish Rust for identifier-shaped types.
var primitiveTypeMap = map[string]string{
	"int":     "int",
	"int8":    "int8",
	"int16":   "int16",
	"int32":   "int32",
	"int64":   "int64",
	"uint":    "uint",
	"uint8":   "uint8",
	"uint16":  "uint16",
	"uint32":  "uint32",
	"uint64":  "uint64",
	"float32": "float32",
	"float64": "float64",
	"byte":    "byte",
	"rune":    "rune",
	"string":  "string",
	"bool":    "bool",
	"error":   "error",
	"any":     "String", // rough; flag if used nontrivially
}

// builtinCallMap — Go builtin → Goish macro name (without `!`).
// Presence means "rewrite f(args) → f!(args)".
var builtinCallMap = map[string]string{
	"len":    "len",
	"cap":    "cap",
	"append": "append",
	"copy":   "copy",
	"delete": "delete",
	"close":  "close",
	"make":   "make",
}
