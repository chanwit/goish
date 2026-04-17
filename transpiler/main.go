package main

import (
	"flag"
	"fmt"
	"io"
	"os"
)

func main() {
	outFlag := flag.String("o", "", "output .rs file (default stdout)")
	flag.Usage = func() {
		fmt.Fprintf(os.Stderr, "Usage: goishc [-o out.rs] input.go\n")
		fmt.Fprintln(os.Stderr, "")
		fmt.Fprintln(os.Stderr, "Transpiles a single Go file to Goish Rust per REFERENCES.md.")
		fmt.Fprintln(os.Stderr, "Emits // TODO: comments where a construct has no direct mapping.")
		flag.PrintDefaults()
	}
	flag.Parse()

	if flag.NArg() != 1 {
		flag.Usage()
		os.Exit(2)
	}
	inPath := flag.Arg(0)

	src, err := os.ReadFile(inPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "read %s: %v\n", inPath, err)
		os.Exit(1)
	}

	out, todos, err := Transpile(inPath, src)
	if err != nil {
		fmt.Fprintf(os.Stderr, "transpile %s: %v\n", inPath, err)
		os.Exit(1)
	}

	var w io.Writer = os.Stdout
	if *outFlag != "" {
		f, err := os.Create(*outFlag)
		if err != nil {
			fmt.Fprintf(os.Stderr, "create %s: %v\n", *outFlag, err)
			os.Exit(1)
		}
		defer f.Close()
		w = f
	}
	io.WriteString(w, out)

	if len(todos) > 0 {
		fmt.Fprintf(os.Stderr, "\ngoishc: %d TODO(s) — manual review needed:\n", len(todos))
		for _, t := range todos {
			fmt.Fprintf(os.Stderr, "  - %s\n", t)
		}
	}
}
