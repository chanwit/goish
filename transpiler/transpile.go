package main

import (
	"bytes"
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"
	"strconv"
	"strings"
)

// Transpile parses Go source and emits Goish Rust. todos is a list of
// human-readable TODOs for the caller to review.
func Transpile(path string, src []byte) (out string, todos []string, err error) {
	fset := token.NewFileSet()
	f, err := parser.ParseFile(fset, path, src, parser.ParseComments)
	if err != nil {
		return "", nil, err
	}
	t := &Transpiler{fset: fset, file: f, buf: &bytes.Buffer{}}
	t.emitFile()
	return t.buf.String(), t.todos, nil
}

type Transpiler struct {
	fset    *token.FileSet
	file    *ast.File
	buf     *bytes.Buffer
	indent  int
	todos   []string
	// importAliases: local Go alias → Goish module path. Populated from
	// import decls so `selector.X` where selector matches a known import
	// becomes `pkg::X` instead of `selector.X`.
	importAliases map[string]string
	// benchLoopVar: when non-empty, we're inside a benchmark!{} body and
	// this is the Go `*testing.B` parameter name. emitForStmt rewrites the
	// `for i := 0; i < <name>.N; i++` idiom to `while <name>.Loop()`.
	benchLoopVar string
	// fnVars: set of package-level var names whose RHS was a composite
	// literal and are emitted as `fn name() -> T { ... }`. emitIdent
	// rewrites bare uses to `name()`.
	fnVars map[string]bool
	// variadicFns: names of same-file functions declared with a `...T`
	// trailing parameter. Call sites with that name and plain args get
	// rewritten to `f(&[a, b, c])` so the Goish `&[T]` param accepts them.
	variadicFns map[string]bool
	// errorMethods: struct-type-name → receiver-name + body of its `Error() string`
	// method, collected in a pre-pass. Populated entries cause the matching
	// struct decl to emit as `ErrorType!{...}` instead of `Struct!{...}`, and
	// the Error method itself to be skipped from the normal receiver branch.
	errorMethods map[string]errorMethodInfo
	// emittedAsError: names of structs emitted as `ErrorType!`. Used at
	// emitFuncDecl time to suppress re-emission of the Error() method.
	emittedAsError map[string]bool
}

// errorMethodInfo holds what pre-pass 3 needs to roll the Error method body
// into the `ErrorType!` macro at struct-emit time.
type errorMethodInfo struct {
	recvName string        // Go receiver name, e.g. `m` in `func (m *MultiError) Error()...`
	body     *ast.BlockStmt
}

// ── low-level output helpers ───────────────────────────────────────────

func (t *Transpiler) write(s string)                 { t.buf.WriteString(s) }
func (t *Transpiler) writef(f string, a ...any)      { fmt.Fprintf(t.buf, f, a...) }
func (t *Transpiler) writeln(s string)               { t.buf.WriteString(s); t.buf.WriteByte('\n') }
func (t *Transpiler) nl()                            { t.buf.WriteByte('\n') }
func (t *Transpiler) in()                            { t.indent++ }
func (t *Transpiler) out()                           { t.indent-- }
func (t *Transpiler) pad() {
	for i := 0; i < t.indent; i++ {
		t.buf.WriteString("    ")
	}
}

func (t *Transpiler) todo(msg string, node ast.Node) {
	pos := t.fset.Position(node.Pos())
	full := fmt.Sprintf("%s:%d: %s", pos.Filename, pos.Line, msg)
	t.todos = append(t.todos, full)
	t.writef("/* TODO: goishc: %s */ ", msg)
}

// ── file-level ─────────────────────────────────────────────────────────

func (t *Transpiler) emitFile() {
	t.importAliases = map[string]string{}
	t.fnVars = map[string]bool{}
	t.variadicFns = map[string]bool{}
	t.errorMethods = map[string]errorMethodInfo{}
	t.emittedAsError = map[string]bool{}
	// Pre-pass 1: collect package-level vars with composite-literal RHS.
	// These become `fn name() -> T { ... }` and bare identifier uses get
	// rewritten to `name()`.
	for _, d := range t.file.Decls {
		gd, ok := d.(*ast.GenDecl)
		if !ok || gd.Tok != token.VAR {
			continue
		}
		for _, sp := range gd.Specs {
			vs := sp.(*ast.ValueSpec)
			if len(vs.Values) == 0 {
				continue
			}
			if _, ok := vs.Values[0].(*ast.CompositeLit); ok {
				for _, n := range vs.Names {
					t.fnVars[n.Name] = true
				}
			}
		}
	}
	// Pre-pass 2: collect variadic function names (receiver-less, same-file).
	// Enables call-site rewrite `f(a, b, c)` → `f(&[a, b, c])`.
	for _, d := range t.file.Decls {
		fd, ok := d.(*ast.FuncDecl)
		if !ok || fd.Recv != nil || fd.Type.Params == nil {
			continue
		}
		pl := fd.Type.Params.List
		if len(pl) == 0 {
			continue
		}
		if _, isEllipsis := pl[len(pl)-1].Type.(*ast.Ellipsis); isEllipsis {
			t.variadicFns[fd.Name.Name] = true
		}
	}
	// Pre-pass 3: collect methods shaped like `func (r *T) Error() string`.
	// Their struct decl will emit as `ErrorType!{...}` rolling this body in,
	// and the method itself will be skipped by the receiver branch of
	// emitFuncDecl. Both pointer and value receivers normalize to the un-starred
	// type name — Go semantics forbid both forms on the same type anyway.
	for _, d := range t.file.Decls {
		fd, ok := d.(*ast.FuncDecl)
		if !ok || fd.Recv == nil || fd.Name.Name != "Error" || fd.Body == nil {
			continue
		}
		if fd.Type.Params != nil && len(fd.Type.Params.List) > 0 {
			continue
		}
		if fd.Type.Results == nil || len(fd.Type.Results.List) != 1 {
			continue
		}
		resTy, ok := fd.Type.Results.List[0].Type.(*ast.Ident)
		if !ok || resTy.Name != "string" {
			continue
		}
		if len(fd.Recv.List) == 0 {
			continue
		}
		recvTypeName, _ := unwrapRecvType(fd.Recv.List[0].Type)
		if recvTypeName == "" {
			continue
		}
		recvName := ""
		if names := fd.Recv.List[0].Names; len(names) > 0 {
			recvName = names[0].Name
		}
		t.errorMethods[recvTypeName] = errorMethodInfo{recvName: recvName, body: fd.Body}
	}
	t.writeln("// Auto-generated by goishc. Review before shipping.")
	t.writeln("#![allow(non_snake_case)]")
	t.writeln("#![allow(non_camel_case_types)]")
	t.writeln("#![allow(unused_imports)]")
	t.writeln("#![allow(unused_variables)]")
	t.writeln("#![allow(unused_mut)]")
	t.writeln("#![allow(dead_code)]")
	t.writeln("")
	t.writeln("use goish::prelude::*;")
	t.writeln("")

	if t.file.Name != nil {
		t.writef("// from Go package %s\n\n", t.file.Name.Name)
	}

	// Imports first — collect aliases and emit complex ones as TODOs.
	for _, d := range t.file.Decls {
		gd, ok := d.(*ast.GenDecl)
		if !ok || gd.Tok != token.IMPORT {
			continue
		}
		for _, sp := range gd.Specs {
			t.emitImportSpec(sp.(*ast.ImportSpec))
		}
	}
	t.writeln("")

	// Then everything else.
	for _, d := range t.file.Decls {
		switch d := d.(type) {
		case *ast.GenDecl:
			if d.Tok == token.IMPORT {
				continue
			}
			t.emitGenDecl(d)
		case *ast.FuncDecl:
			t.emitFuncDecl(d)
		default:
			t.todo(fmt.Sprintf("unknown top-level decl %T", d), d)
			t.nl()
		}
	}
}

func (t *Transpiler) emitImportSpec(sp *ast.ImportSpec) {
	goPath, _ := strconv.Unquote(sp.Path.Value)
	goish, ok := resolveImport(goPath)
	// Local alias: use Name.Name if set, else last segment of goPath.
	var alias string
	if sp.Name != nil {
		alias = sp.Name.Name
	} else {
		alias = lastSegment(goPath)
	}
	if !ok {
		msg := fmt.Sprintf("complex import %q — no direct Goish mapping", goPath)
		t.writef("// TODO: %s\n", msg)
		t.todos = append(t.todos, msg)
		return
	}
	t.importAliases[alias] = goish
	// prelude already re-exports stdlib, but emit a comment for traceability:
	t.writef("// import %q → %s (via prelude)\n", goPath, goish)
}

func lastSegment(p string) string {
	if i := strings.LastIndex(p, "/"); i >= 0 {
		return p[i+1:]
	}
	return p
}

// ── top-level decls ────────────────────────────────────────────────────

func (t *Transpiler) emitGenDecl(d *ast.GenDecl) {
	switch d.Tok {
	case token.TYPE:
		for _, sp := range d.Specs {
			t.emitTypeSpec(sp.(*ast.TypeSpec))
			t.nl()
		}
	case token.CONST:
		t.emitConstBlock(d)
	case token.VAR:
		for _, sp := range d.Specs {
			t.emitPackageVar(sp.(*ast.ValueSpec))
		}
	default:
		t.todo(fmt.Sprintf("GenDecl tok=%s", d.Tok), d)
	}
}

func (t *Transpiler) emitTypeSpec(sp *ast.TypeSpec) {
	name := sp.Name.Name
	switch ty := sp.Type.(type) {
	case *ast.StructType:
		if info, ok := t.errorMethods[name]; ok {
			t.emitErrorType(name, ty, info)
			t.emittedAsError[name] = true
			return
		}
		t.writef("Struct!{ type %s struct {\n", name)
		t.in()
		t.emitStructFields(ty)
		t.out()
		t.writeln("} }")
	case *ast.ArrayType:
		if ty.Len == nil { // slice
			t.write("Type!(")
			t.write(name)
			t.write(" = []")
			t.emitType(ty.Elt)
			t.writeln(");")
		} else {
			t.todo("fixed-size array type alias", sp)
			t.nl()
		}
	case *ast.Ident:
		// type X Y  — int newtype or generic
		t.writef("Type!(%s = ", name)
		t.emitType(ty)
		t.writeln(");")
	case *ast.InterfaceType:
		t.emitInterfaceDecl(name, ty)
	default:
		t.writef("Type!(%s = ", name)
		t.emitType(sp.Type)
		t.writeln(");")
	}
}

func (t *Transpiler) emitStructFields(s *ast.StructType) {
	if s.Fields == nil {
		return
	}
	n := len(s.Fields.List)
	for i, f := range s.Fields.List {
		t.pad()
		if len(f.Names) == 0 {
			// Embedded field — Goish has no method promotion. Emit using the
			// type's name lowercased so later ports can either (a) delegate
			// each embedded method by hand, or (b) implement `Deref` / `DerefMut`
			// targeting this field so callers of `s.Foo()` still resolve.
			fieldName := embeddedFieldName(f.Type)
			msg := fmt.Sprintf("embedded %s — Goish has no method promotion; "+
				"delegate by hand, or impl Deref<Target=%s> to reach embedded methods",
				fieldName, goTypeString(f.Type))
			t.todo(msg, f)
			t.write(fieldName)
			t.write(": ")
			t.emitType(f.Type)
		} else {
			for j, n := range f.Names {
				if j > 0 {
					t.write(", ")
				}
				t.write(n.Name)
			}
			t.write(" ")
			t.emitType(f.Type)
		}
		if i < n-1 {
			t.writeln(",")
		} else {
			t.nl()
		}
	}
}

// emitInterfaceDecl emits `Interface!{ type X[: Super[+…]] interface { … } }`
// per goish v0.21.0. Anonymous method entries (embedded interfaces) are
// hoisted into the supertrait clause when they're bare idents or qualified
// paths; everything else still gets a TODO inside the macro body.
func (t *Transpiler) emitInterfaceDecl(name string, iface *ast.InterfaceType) {
	var supers []string
	type todoEntry struct {
		msg  string
		node ast.Node
	}
	var todos []todoEntry
	var methods []*ast.Field
	if iface.Methods != nil {
		for _, m := range iface.Methods.List {
			if _, isFn := m.Type.(*ast.FuncType); isFn && len(m.Names) > 0 {
				methods = append(methods, m)
				continue
			}
			// Anonymous entry — try to hoist.
			switch ty := m.Type.(type) {
			case *ast.Ident:
				// Bare ident — the Interface! macro mangles to `__<Id>Trait`.
				supers = append(supers, ty.Name)
			case *ast.SelectorExpr:
				// Qualified path `pkg.Name` → `pkg::Name` (verbatim by the macro).
				if pkg, ok := ty.X.(*ast.Ident); ok {
					supers = append(supers, pkg.Name+"::"+ty.Sel.Name)
				} else {
					todos = append(todos, todoEntry{
						msg:  "embedded interface — unsupported receiver shape",
						node: m,
					})
				}
			default:
				todos = append(todos, todoEntry{
					msg:  "embedded interface literal — hand-port as supertrait or separate decl",
					node: m,
				})
			}
		}
	}

	t.write("Interface!{\n")
	t.in()
	t.pad()
	t.writef("type %s", name)
	if len(supers) > 0 {
		t.write(": ")
		t.write(strings.Join(supers, " + "))
	}
	t.writeln(" interface {")
	t.in()
	for _, m := range methods {
		ft := m.Type.(*ast.FuncType)
		t.pad()
		t.writef("fn %s(&self", m.Names[0].Name)
		if ft.Params != nil {
			for _, p := range ft.Params.List {
				for _, n := range p.Names {
					t.write(", ")
					t.write(n.Name)
					t.write(": ")
					t.emitType(p.Type)
				}
				if len(p.Names) == 0 {
					t.write(", _: ")
					t.emitType(p.Type)
				}
			}
		}
		t.write(")")
		t.emitResults(ft.Results)
		t.writeln(";")
	}
	for _, td := range todos {
		t.pad()
		t.todo(td.msg, td.node)
		t.nl()
	}
	t.out()
	t.pad()
	t.writeln("}")
	t.out()
	t.writeln("}")
}

// emitErrorType emits `ErrorType!{ type X struct { … } fn Error(&self) -> string { … } }`
// for a struct that pre-pass 3 flagged as having an `Error() string` method.
// Struct fields use the Rust-colon shape that the macro's `Field::parse_named`
// expects; the Error body is copied verbatim with a `let <recv> = self;`
// rebinding when the Go source used a named receiver.
func (t *Transpiler) emitErrorType(name string, s *ast.StructType, info errorMethodInfo) {
	t.writef("ErrorType!{\n")
	t.in()
	t.pad()
	t.writef("type %s struct {\n", name)
	t.in()
	t.emitErrorTypeFields(s)
	t.out()
	t.pad()
	t.writeln("}")
	t.pad()
	t.writeln("fn Error(&self) -> string {")
	t.in()
	if info.recvName != "" && info.recvName != "_" && info.recvName != "self" {
		t.pad()
		t.writef("let %s = self;\n", info.recvName)
	}
	if info.body != nil {
		t.emitBlockBody(info.body)
	}
	t.out()
	t.pad()
	t.writeln("}")
	t.out()
	t.writeln("}")
}

// emitErrorTypeFields emits struct fields in `name: Type,` form for use
// inside `ErrorType!`, whose proc-macro uses `syn::Field::parse_named`.
// Embedded fields are rendered with a derived name + TODO (same policy as
// the existing Struct! path).
func (t *Transpiler) emitErrorTypeFields(s *ast.StructType) {
	if s.Fields == nil {
		return
	}
	for _, f := range s.Fields.List {
		t.pad()
		if len(f.Names) == 0 {
			fieldName := embeddedFieldName(f.Type)
			msg := fmt.Sprintf("embedded %s — Goish has no method promotion; "+
				"delegate by hand, or impl Deref<Target=%s> to reach embedded methods",
				fieldName, goTypeString(f.Type))
			t.todo(msg, f)
			t.write(fieldName)
			t.write(": ")
			t.emitType(f.Type)
			t.writeln(",")
			continue
		}
		for j, n := range f.Names {
			if j > 0 {
				t.write(", ")
			}
			t.write(n.Name)
		}
		t.write(": ")
		t.emitType(f.Type)
		t.writeln(",")
	}
}

func (t *Transpiler) emitConstBlock(d *ast.GenDecl) {
	// Detect iota use — if any spec references iota, emit as Const! block.
	usesIota := false
	for _, sp := range d.Specs {
		vs := sp.(*ast.ValueSpec)
		for _, v := range vs.Values {
			if containsIdent(v, "iota") {
				usesIota = true
				break
			}
		}
		if usesIota {
			break
		}
	}
	if usesIota || len(d.Specs) > 1 {
		t.writeln("Const! {")
		t.in()
		for _, sp := range d.Specs {
			t.emitConstSpecLine(sp.(*ast.ValueSpec))
		}
		t.out()
		t.writeln("}")
		return
	}
	// Simple single-const.
	for _, sp := range d.Specs {
		vs := sp.(*ast.ValueSpec)
		for i, n := range vs.Names {
			t.pad()
			t.write("pub const ")
			t.write(n.Name)
			t.write(": ")
			if vs.Type != nil {
				t.emitType(vs.Type)
			} else {
				t.write("i64")
			}
			t.write(" = ")
			if i < len(vs.Values) {
				t.emitExpr(vs.Values[i])
			} else {
				t.write("Default::default()")
			}
			t.writeln(";")
		}
	}
}

func (t *Transpiler) emitConstSpecLine(vs *ast.ValueSpec) {
	for i, n := range vs.Names {
		t.pad()
		t.write(n.Name)
		if vs.Type != nil {
			t.write(": ")
			t.emitType(vs.Type)
		}
		if i < len(vs.Values) {
			t.write(" = ")
			t.emitExpr(vs.Values[i])
		}
		t.writeln(";")
	}
}

func (t *Transpiler) emitPackageVar(vs *ast.ValueSpec) {
	for i, n := range vs.Names {
		if i < len(vs.Values) {
			// Composite literal RHS → emit as `fn name() -> T { ... }` so
			// call sites can use `name()` (matches REFERENCES.md §25).
			if cl, ok := vs.Values[i].(*ast.CompositeLit); ok {
				t.writef("pub fn %s() -> ", n.Name)
				if vs.Type != nil {
					t.emitType(vs.Type)
				} else {
					t.emitInferredType(cl.Type)
				}
				t.write(" { ")
				t.emitExpr(cl)
				t.writeln(" }")
				continue
			}
			t.writef("var!(%s", n.Name)
			if vs.Type != nil {
				t.write(" ")
				t.emitType(vs.Type)
			}
			t.write(" = ")
			t.emitExpr(vs.Values[i])
			t.writeln(");")
		} else {
			// no initializer — emit typed lazy with Default::default
			t.writef("var!(%s ", n.Name)
			if vs.Type != nil {
				t.emitType(vs.Type)
			} else {
				t.write("()")
			}
			t.writeln(" = Default::default());")
		}
	}
}

// emitInferredType writes the Goish Rust type corresponding to a composite-
// literal's Type expr (used when the user didn't annotate a var explicitly).
func (t *Transpiler) emitInferredType(e ast.Expr) {
	switch ty := e.(type) {
	case *ast.ArrayType:
		if ty.Len == nil {
			t.write("slice<")
			t.emitType(ty.Elt)
			t.write(">")
			return
		}
	case *ast.MapType:
		t.write("map<")
		t.emitType(ty.Key)
		t.write(", ")
		t.emitType(ty.Value)
		t.write(">")
		return
	}
	t.emitType(e)
}

// ── functions and methods ──────────────────────────────────────────────

func (t *Transpiler) emitFuncDecl(d *ast.FuncDecl) {
	// 1. testing.T / testing.B / testing.M functions → macro-wrapped forms.
	if d.Recv == nil {
		if ok, paramName := isTestingFunc(d, "T"); ok && strings.HasPrefix(d.Name.Name, "Test") && d.Name.Name != "TestMain" {
			t.emitTestMacro("test", d, paramName)
			return
		}
		if ok, paramName := isTestingFunc(d, "B"); ok && strings.HasPrefix(d.Name.Name, "Benchmark") {
			t.emitTestMacro("benchmark", d, paramName)
			return
		}
		if ok, paramName := isTestingFunc(d, "M"); ok && d.Name.Name == "TestMain" {
			t.emitTestMacro("test_main", d, paramName)
			return
		}
		if strings.HasPrefix(d.Name.Name, "Example") {
			t.todo(fmt.Sprintf("Go example function %q — Goish has no Example harness; port to a regular test", d.Name.Name), d)
		}
		if strings.HasPrefix(d.Name.Name, "Fuzz") {
			t.todo(fmt.Sprintf("Go fuzz function %q — Goish has no Fuzz harness; port to a regular test", d.Name.Name), d)
		}
	}

	// 2. Receiver → impl block
	if d.Recv != nil && len(d.Recv.List) > 0 {
		recv := d.Recv.List[0]
		recvType, isPtr := unwrapRecvType(recv.Type)
		// Error() method on a struct that pre-pass 3 flagged as an ErrorType!
		// target was already rolled into the macro — skip re-emission here.
		if d.Name.Name == "Error" && t.emittedAsError[recvType] {
			return
		}
		t.writef("impl %s {\n", recvType)
		t.in()
		t.pad()
		t.writef("pub fn %s(", d.Name.Name)
		if len(recv.Names) > 0 && recv.Names[0].Name != "_" {
			// Caller sees `self` as the receiver name; the Go source used a different
			// name — add a rebinding let so the body compiles without rewriting every
			// occurrence. Cheap + keeps the port obvious.
			if isPtr {
				t.write("&mut self")
			} else {
				t.write("&self")
			}
		} else {
			if isPtr {
				t.write("&mut self")
			} else {
				t.write("&self")
			}
		}
		t.emitParams(d.Type.Params, false)
		t.write(")")
		t.emitResults(d.Type.Results)
		t.writeln(" {")
		t.in()
		// rebinding so user's `s.X` style (where `s` is Go's receiver name) still works
		if len(recv.Names) > 0 && recv.Names[0].Name != "_" && recv.Names[0].Name != "self" {
			t.pad()
			if isPtr {
				t.writef("let %s = self;\n", recv.Names[0].Name)
			} else {
				t.writef("let %s = self;\n", recv.Names[0].Name)
			}
		}
		if d.Body != nil {
			t.emitBlockBody(d.Body)
		}
		t.out()
		t.pad()
		t.writeln("}")
		t.out()
		t.writeln("}")
		t.nl()
		return
	}

	// Plain function.
	t.writef("pub fn %s(", d.Name.Name)
	t.emitParams(d.Type.Params, true)
	t.write(")")
	t.emitResults(d.Type.Results)
	if d.Body == nil {
		t.writeln(";")
		return
	}
	t.writeln(" {")
	t.in()
	t.emitBlockBody(d.Body)
	t.out()
	t.writeln("}")
	t.nl()
}

// isTestingFunc returns (true, paramName) iff d has exactly one param of
// type `*testing.<sel>` (e.g. `*testing.T`). Works regardless of the local
// import alias — we only check the outer Sel name.
func isTestingFunc(d *ast.FuncDecl, sel string) (bool, string) {
	if d.Type.Params == nil || len(d.Type.Params.List) != 1 {
		return false, ""
	}
	p := d.Type.Params.List[0]
	if len(p.Names) != 1 {
		return false, ""
	}
	star, ok := p.Type.(*ast.StarExpr)
	if !ok {
		return false, ""
	}
	se, ok := star.X.(*ast.SelectorExpr)
	if !ok {
		return false, ""
	}
	if se.Sel.Name != sel {
		return false, ""
	}
	return true, p.Names[0].Name
}

func (t *Transpiler) emitTestMacro(macro string, d *ast.FuncDecl, paramName string) {
	t.writef("%s!{ fn %s(%s) {\n", macro, d.Name.Name, paramName)
	t.in()
	// Set/restore benchmark-loop-var context so emitForStmt rewrites b.N.
	save := t.benchLoopVar
	if macro == "benchmark" {
		t.benchLoopVar = paramName
	}
	if d.Body != nil {
		t.emitBlockBody(d.Body)
	}
	t.benchLoopVar = save
	t.out()
	t.writeln("}}")
	t.nl()
}

func unwrapRecvType(e ast.Expr) (string, bool) {
	switch ty := e.(type) {
	case *ast.StarExpr:
		if id, ok := ty.X.(*ast.Ident); ok {
			return id.Name, true
		}
	case *ast.Ident:
		return ty.Name, false
	}
	return "Unknown", false
}

func (t *Transpiler) emitParams(fl *ast.FieldList, first bool) {
	if fl == nil {
		return
	}
	for _, p := range fl.List {
		for _, n := range p.Names {
			if !first {
				t.write(", ")
			}
			first = false
			t.write(n.Name)
			t.write(": ")
			t.emitType(p.Type)
		}
		if len(p.Names) == 0 {
			if !first {
				t.write(", ")
			}
			first = false
			t.write("_: ")
			t.emitType(p.Type)
		}
	}
}

func (t *Transpiler) emitResults(fl *ast.FieldList) {
	if fl == nil || len(fl.List) == 0 {
		return
	}
	// Count actual result types (Go can have named returns).
	var types []ast.Expr
	for _, p := range fl.List {
		if len(p.Names) == 0 {
			types = append(types, p.Type)
		} else {
			for range p.Names {
				types = append(types, p.Type)
			}
		}
	}
	t.write(" -> ")
	if len(types) == 1 {
		t.emitType(types[0])
		return
	}
	t.write("(")
	for i, ty := range types {
		if i > 0 {
			t.write(", ")
		}
		t.emitType(ty)
	}
	t.write(")")
}

// emitBlockBody emits the statements inside a block. Used for fn bodies,
// if/for/etc bodies. Does NOT emit surrounding `{`/`}`.
func (t *Transpiler) emitBlockBody(b *ast.BlockStmt) {
	for _, s := range b.List {
		t.pad()
		t.emitStmt(s)
		// Most stmt emitters end with either a newline or a `;` + newline.
		// Ensure one newline at end.
		if !bytes.HasSuffix(t.buf.Bytes(), []byte("\n")) {
			t.nl()
		}
	}
}

// ── statements ─────────────────────────────────────────────────────────

func (t *Transpiler) emitStmt(s ast.Stmt) {
	switch s := s.(type) {
	case *ast.DeclStmt:
		if gd, ok := s.Decl.(*ast.GenDecl); ok {
			t.emitLocalDecl(gd)
			return
		}
		t.todo("non-GenDecl DeclStmt", s)
	case *ast.AssignStmt:
		t.emitAssignStmt(s)
	case *ast.ReturnStmt:
		t.emitReturnStmt(s)
	case *ast.IfStmt:
		t.emitIfStmt(s)
	case *ast.ForStmt:
		t.emitForStmt(s)
	case *ast.RangeStmt:
		t.emitRangeStmt(s)
	case *ast.SwitchStmt:
		t.emitSwitchStmt(s)
	case *ast.TypeSwitchStmt:
		t.todo("type switch — port manually with match on a concrete enum", s)
	case *ast.SelectStmt:
		t.emitSelectStmt(s)
	case *ast.BranchStmt:
		t.emitBranchStmt(s)
	case *ast.IncDecStmt:
		t.emitExpr(s.X)
		if s.Tok == token.INC {
			t.write(" += 1;")
		} else {
			t.write(" -= 1;")
		}
	case *ast.ExprStmt:
		t.emitExpr(s.X)
		t.write(";")
	case *ast.DeferStmt:
		if t.emitDeferRecoverIdiom(s) {
			return
		}
		t.write("defer!{ ")
		t.emitExpr(s.Call)
		t.write("; }")
	case *ast.GoStmt:
		t.write("let _ = go!{ ")
		t.emitExpr(s.Call)
		t.write("; };")
	case *ast.SendStmt:
		t.emitExpr(s.Chan)
		t.write(".Send(")
		t.emitExpr(s.Value)
		t.write(");")
	case *ast.BlockStmt:
		t.writeln("{")
		t.in()
		t.emitBlockBody(s)
		t.out()
		t.pad()
		t.write("}")
	case *ast.LabeledStmt:
		t.todo(fmt.Sprintf("label %q — Rust needs explicit loop labels", s.Label.Name), s)
		t.emitStmt(s.Stmt)
	case *ast.EmptyStmt:
		t.write("// (empty)")
	default:
		t.todo(fmt.Sprintf("unknown stmt %T", s), s)
	}
}

// emitDeferRecoverIdiom detects:
//
//	defer func() { ... recover() ... }()
//
// and emits an actionable template. Goish's `recover!{body}` wraps the
// *risky code*, not the handler — which means the statements that come
// AFTER this defer in Go source logically belong inside `recover!{}`, and
// the handler body belongs inside `if let Some(r) = ... { }`. We can't do
// that rewrite mechanically without reordering the enclosing block, so we
// emit a clear template the porter can shape by hand.
func (t *Transpiler) emitDeferRecoverIdiom(s *ast.DeferStmt) bool {
	fl, ok := s.Call.Fun.(*ast.FuncLit)
	if !ok {
		return false
	}
	if !containsRecoverCall(fl.Body) {
		return false
	}
	msg := "defer func(){ recover() ... }() — Goish inverts this: wrap the " +
		"code *after* this defer in recover!{} and put the handler inside " +
		"`if let Some(r) = ... { }`"
	t.todo(msg, s)
	t.writeln("if let Some(r) = recover!{")
	t.in()
	t.pad()
	t.writeln("/* TODO goishc: move everything AFTER this defer into this block */")
	t.out()
	t.pad()
	t.writeln("} {")
	t.in()
	if fl.Body != nil {
		for _, st := range unwrapRecoverGuard(fl.Body.List) {
			t.pad()
			t.emitStmt(st)
			if !bytes.HasSuffix(t.buf.Bytes(), []byte("\n")) {
				t.nl()
			}
		}
	}
	t.out()
	t.pad()
	t.write("}")
	return true
}

// unwrapRecoverGuard strips the canonical `if r := recover(); r != nil { body }`
// wrapper — when we're already inside `if let Some(r) = recover!{}`, `r` is
// bound, so the guard + the bare recover() call are redundant. Returns the
// inner body's statements (or the original list if the shape doesn't match).
func unwrapRecoverGuard(stmts []ast.Stmt) []ast.Stmt {
	if len(stmts) != 1 {
		return stmts
	}
	ifs, ok := stmts[0].(*ast.IfStmt)
	if !ok || ifs.Init == nil {
		return stmts
	}
	as, ok := ifs.Init.(*ast.AssignStmt)
	if !ok || as.Tok != token.DEFINE || len(as.Rhs) != 1 {
		return stmts
	}
	call, ok := as.Rhs[0].(*ast.CallExpr)
	if !ok {
		return stmts
	}
	id, ok := call.Fun.(*ast.Ident)
	if !ok || id.Name != "recover" {
		return stmts
	}
	if ifs.Body != nil {
		return ifs.Body.List
	}
	return stmts
}

func containsRecoverCall(b *ast.BlockStmt) bool {
	if b == nil {
		return false
	}
	found := false
	ast.Inspect(b, func(n ast.Node) bool {
		if call, ok := n.(*ast.CallExpr); ok {
			if id, ok := call.Fun.(*ast.Ident); ok && id.Name == "recover" {
				found = true
				return false
			}
		}
		return true
	})
	return found
}

func (t *Transpiler) emitLocalDecl(gd *ast.GenDecl) {
	switch gd.Tok {
	case token.VAR:
		for _, sp := range gd.Specs {
			vs := sp.(*ast.ValueSpec)
			for i, n := range vs.Names {
				t.pad()
				t.write("let mut ")
				t.write(n.Name)
				if vs.Type != nil {
					t.write(": ")
					t.emitType(vs.Type)
				}
				t.write(" = ")
				if i < len(vs.Values) {
					t.emitExpr(vs.Values[i])
				} else if vs.Type != nil {
					t.write("Default::default()")
				} else {
					t.write("Default::default()")
				}
				t.writeln(";")
			}
		}
	case token.CONST:
		t.emitConstBlock(gd)
	case token.TYPE:
		for _, sp := range gd.Specs {
			t.emitTypeSpec(sp.(*ast.TypeSpec))
		}
	}
}

func (t *Transpiler) emitAssignStmt(s *ast.AssignStmt) {
	// Special: x := <-ch  or  x, ok := <-ch  → let (x, _) = ch.Recv();
	if len(s.Rhs) == 1 {
		if u, ok := s.Rhs[0].(*ast.UnaryExpr); ok && u.Op == token.ARROW {
			t.write("let ")
			if s.Tok != token.DEFINE {
				// reassign variant — just emit assignment form
			}
			t.write("(")
			for i, l := range s.Lhs {
				if i > 0 {
					t.write(", ")
				}
				t.emitExpr(l)
			}
			if len(s.Lhs) == 1 {
				t.write(", _")
			}
			t.write(") = ")
			t.emitExpr(u.X)
			t.write(".Recv();")
			return
		}
	}

	isDefine := s.Tok == token.DEFINE
	// Multi-LHS: always tuple destructure.
	if len(s.Lhs) > 1 {
		if isDefine {
			t.write("let (")
		} else {
			t.write("(")
		}
		for i, l := range s.Lhs {
			if i > 0 {
				t.write(", ")
			}
			if id, ok := l.(*ast.Ident); ok && id.Name == "_" {
				t.write("_")
			} else {
				t.emitExpr(l)
			}
		}
		t.write(") = ")
		if len(s.Rhs) == 1 {
			t.emitExpr(s.Rhs[0])
		} else {
			t.write("(")
			for i, r := range s.Rhs {
				if i > 0 {
					t.write(", ")
				}
				t.emitExpr(r)
			}
			t.write(")")
		}
		t.write(";")
		return
	}

	// Single LHS.
	lhs := s.Lhs[0]
	if isDefine {
		if id, ok := lhs.(*ast.Ident); ok && id.Name == "_" {
			t.write("let _ = ")
		} else {
			t.write("let mut ")
			t.emitExpr(lhs)
			t.write(" = ")
		}
		t.emitExpr(s.Rhs[0])
		t.write(";")
		return
	}

	// Compound / plain assignment.
	op := ""
	switch s.Tok {
	case token.ASSIGN:
		op = "="
	case token.ADD_ASSIGN:
		op = "+="
	case token.SUB_ASSIGN:
		op = "-="
	case token.MUL_ASSIGN:
		op = "*="
	case token.QUO_ASSIGN:
		op = "/="
	case token.REM_ASSIGN:
		op = "%="
	case token.AND_ASSIGN:
		op = "&="
	case token.OR_ASSIGN:
		op = "|="
	case token.XOR_ASSIGN:
		op = "^="
	case token.SHL_ASSIGN:
		op = "<<="
	case token.SHR_ASSIGN:
		op = ">>="
	default:
		op = "=" // fall back
		t.todo(fmt.Sprintf("assign op %s", s.Tok), s)
	}
	t.emitExpr(lhs)
	t.write(" " + op + " ")
	t.emitExpr(s.Rhs[0])
	t.write(";")
}

func (t *Transpiler) emitReturnStmt(s *ast.ReturnStmt) {
	if len(s.Results) == 0 {
		t.write("return;")
		return
	}
	if len(s.Results) == 1 {
		t.write("return ")
		t.emitExpr(s.Results[0])
		t.write(";")
		return
	}
	t.write("return (")
	for i, r := range s.Results {
		if i > 0 {
			t.write(", ")
		}
		t.emitExpr(r)
	}
	t.write(");")
}

func (t *Transpiler) emitIfStmt(s *ast.IfStmt) {
	if s.Init != nil {
		// Go:  if x := f(); cond { ... }
		// Rust: wrap in a block so `x` stays scoped to the if.
		t.writeln("{")
		t.in()
		t.pad()
		t.emitStmt(s.Init)
		if !bytes.HasSuffix(t.buf.Bytes(), []byte("\n")) {
			t.nl()
		}
		t.pad()
	}
	t.write("if ")
	t.emitExpr(s.Cond)
	t.writeln(" {")
	t.in()
	t.emitBlockBody(s.Body)
	t.out()
	t.pad()
	t.write("}")
	if s.Else != nil {
		t.write(" else ")
		switch el := s.Else.(type) {
		case *ast.IfStmt:
			t.emitIfStmt(el)
		case *ast.BlockStmt:
			t.writeln("{")
			t.in()
			t.emitBlockBody(el)
			t.out()
			t.pad()
			t.write("}")
		default:
			t.emitStmt(el)
		}
	}
	if s.Init != nil {
		t.nl()
		t.out()
		t.pad()
		t.write("}")
	}
}

func (t *Transpiler) emitForStmt(s *ast.ForStmt) {
	// Benchmark idiom: for i := 0; i < b.N; i++ → while b.Loop()
	if t.benchLoopVar != "" && isBenchForLoop(s, t.benchLoopVar) {
		t.writef("while %s.Loop() {\n", t.benchLoopVar)
		t.in()
		t.emitBlockBody(s.Body)
		t.out()
		t.pad()
		t.write("}")
		return
	}
	// Classic counting: for i := lo; i < hi; i++ { }  → for i in lo..hi { }
	if isSimpleCountFor(s) {
		init := s.Init.(*ast.AssignStmt)
		cond := s.Cond.(*ast.BinaryExpr)
		name := init.Lhs[0].(*ast.Ident).Name
		lo := init.Rhs[0]
		hi := cond.Y
		op := ".."
		if cond.Op == token.LEQ {
			op = "..="
		}
		t.writef("for %s in ", name)
		t.emitExpr(lo)
		t.write(op)
		t.emitExpr(hi)
		t.writeln(" {")
		t.in()
		t.emitBlockBody(s.Body)
		t.out()
		t.pad()
		t.write("}")
		return
	}

	// No init / post: while-style.
	if s.Init == nil && s.Post == nil {
		if s.Cond == nil {
			t.writeln("loop {")
		} else {
			t.write("while ")
			t.emitExpr(s.Cond)
			t.writeln(" {")
		}
		t.in()
		t.emitBlockBody(s.Body)
		t.out()
		t.pad()
		t.write("}")
		return
	}

	// General: desugar init + while cond + post at end of body.
	t.writeln("{")
	t.in()
	if s.Init != nil {
		t.pad()
		t.emitStmt(s.Init)
		if !bytes.HasSuffix(t.buf.Bytes(), []byte("\n")) {
			t.nl()
		}
	}
	t.pad()
	if s.Cond != nil {
		t.write("while ")
		t.emitExpr(s.Cond)
		t.writeln(" {")
	} else {
		t.writeln("loop {")
	}
	t.in()
	t.emitBlockBody(s.Body)
	if s.Post != nil {
		t.pad()
		t.emitStmt(s.Post)
		if !bytes.HasSuffix(t.buf.Bytes(), []byte("\n")) {
			t.nl()
		}
	}
	t.out()
	t.pad()
	t.writeln("}")
	t.out()
	t.pad()
	t.write("}")
}

// isBenchForLoop matches exactly `for X := 0; X < <bench>.N; X++`.
func isBenchForLoop(s *ast.ForStmt, bench string) bool {
	if !isSimpleCountFor(s) {
		return false
	}
	cond := s.Cond.(*ast.BinaryExpr)
	sel, ok := cond.Y.(*ast.SelectorExpr)
	if !ok || sel.Sel.Name != "N" {
		return false
	}
	id, ok := sel.X.(*ast.Ident)
	if !ok || id.Name != bench {
		return false
	}
	// low must be 0
	init := s.Init.(*ast.AssignStmt)
	lit, ok := init.Rhs[0].(*ast.BasicLit)
	if !ok || lit.Value != "0" {
		return false
	}
	return true
}

func isSimpleCountFor(s *ast.ForStmt) bool {
	if s.Init == nil || s.Cond == nil || s.Post == nil {
		return false
	}
	init, ok := s.Init.(*ast.AssignStmt)
	if !ok || init.Tok != token.DEFINE || len(init.Lhs) != 1 || len(init.Rhs) != 1 {
		return false
	}
	if _, ok := init.Lhs[0].(*ast.Ident); !ok {
		return false
	}
	cond, ok := s.Cond.(*ast.BinaryExpr)
	if !ok || (cond.Op != token.LSS && cond.Op != token.LEQ) {
		return false
	}
	post, ok := s.Post.(*ast.IncDecStmt)
	if !ok || post.Tok != token.INC {
		return false
	}
	// Check the loop variable matches.
	vname := init.Lhs[0].(*ast.Ident).Name
	if left, ok := cond.X.(*ast.Ident); !ok || left.Name != vname {
		return false
	}
	if left, ok := post.X.(*ast.Ident); !ok || left.Name != vname {
		return false
	}
	return true
}

func (t *Transpiler) emitRangeStmt(s *ast.RangeStmt) {
	// Go: for k, v := range m { }  → for (k, v) in range!(m) { }
	// Go: for i, v := range xs { }
	// Go: for v := range ch { }    → while let (v, true) = ch.Recv() { }
	// Go: for _, v := range xs { }
	// Go: for i := range xs { }    → one-value form: for i in range!(xs).map(|(i,_)| i)
	// We emit the common (k, v) form; single-value form uses a tuple with `_`.

	// Detect channel range: the Range target's type is unknown at AST level, but
	// the "for v := range ch" (no value 2nd) pattern plus a receiver-typed target
	// is common. We emit a conservative two-value tuple — user can tweak.
	keyIsBlank := s.Key == nil
	valIsBlank := s.Value == nil
	t.write("for ")
	if keyIsBlank && valIsBlank {
		t.write("_")
	} else if valIsBlank {
		t.write("(")
		if keyIsBlank {
			t.write("_")
		} else {
			t.emitExpr(s.Key)
		}
		t.write(", _)")
	} else {
		t.write("(")
		if keyIsBlank {
			t.write("_")
		} else {
			t.emitExpr(s.Key)
		}
		t.write(", ")
		t.emitExpr(s.Value)
		t.write(")")
	}
	t.write(" in range!(")
	t.emitExpr(s.X)
	t.writeln(") {")
	t.in()
	t.emitBlockBody(s.Body)
	t.out()
	t.pad()
	t.write("}")
}

func (t *Transpiler) emitSwitchStmt(s *ast.SwitchStmt) {
	// Two shapes:
	//   switch tag { case a,b: ... default: ... }   → match tag { a|b => {}, _ => {} }
	//   switch { case cond: ... }                   → if cond { } else if ... { } else { }
	if s.Tag == nil {
		first := true
		for _, c := range s.Body.List {
			cc := c.(*ast.CaseClause)
			if cc.List == nil {
				t.pad()
				t.write("else {\n")
				t.in()
				for _, st := range cc.Body {
					t.pad()
					t.emitStmt(st)
					if !bytes.HasSuffix(t.buf.Bytes(), []byte("\n")) {
						t.nl()
					}
				}
				t.out()
				t.pad()
				t.write("}")
				continue
			}
			if first {
				t.write("if ")
				first = false
			} else {
				t.write(" else if ")
			}
			for i, cond := range cc.List {
				if i > 0 {
					t.write(" || ")
				}
				t.emitExpr(cond)
			}
			t.writeln(" {")
			t.in()
			for _, st := range cc.Body {
				t.pad()
				t.emitStmt(st)
				if !bytes.HasSuffix(t.buf.Bytes(), []byte("\n")) {
					t.nl()
				}
			}
			t.out()
			t.pad()
			t.write("}")
		}
		return
	}

	// Tag-form: match.
	t.write("match ")
	t.emitExpr(s.Tag)
	t.writeln(" {")
	t.in()
	hasDefault := false
	for _, c := range s.Body.List {
		cc := c.(*ast.CaseClause)
		t.pad()
		if cc.List == nil {
			hasDefault = true
			t.write("_ => {")
		} else {
			for i, cond := range cc.List {
				if i > 0 {
					t.write(" | ")
				}
				t.emitExpr(cond)
			}
			t.write(" => {")
		}
		t.nl()
		t.in()
		for _, st := range cc.Body {
			t.pad()
			t.emitStmt(st)
			if !bytes.HasSuffix(t.buf.Bytes(), []byte("\n")) {
				t.nl()
			}
		}
		t.out()
		t.pad()
		t.writeln("},")
	}
	if !hasDefault {
		t.pad()
		t.writeln("_ => {},")
	}
	t.out()
	t.pad()
	t.write("}")
}

func (t *Transpiler) emitSelectStmt(s *ast.SelectStmt) {
	t.writeln("select! {")
	t.in()
	for _, c := range s.Body.List {
		cc := c.(*ast.CommClause)
		t.pad()
		t.emitCommClause(cc)
	}
	t.out()
	t.pad()
	t.write("}")
}

func (t *Transpiler) emitCommClause(cc *ast.CommClause) {
	switch comm := cc.Comm.(type) {
	case nil:
		t.write("default => {")
	case *ast.SendStmt:
		t.write("send(")
		t.emitExpr(comm.Chan)
		t.write(", ")
		t.emitExpr(comm.Value)
		t.write(") => {")
	case *ast.ExprStmt:
		// naked <-ch receive, no binding
		if u, ok := comm.X.(*ast.UnaryExpr); ok && u.Op == token.ARROW {
			t.write("recv(")
			t.emitExpr(u.X)
			t.write(") |_| => {")
		} else {
			t.todo("unusual select comm", cc)
		}
	case *ast.AssignStmt:
		if len(comm.Rhs) == 1 {
			if u, ok := comm.Rhs[0].(*ast.UnaryExpr); ok && u.Op == token.ARROW {
				t.write("recv(")
				t.emitExpr(u.X)
				t.write(")")
				if len(comm.Lhs) == 1 {
					t.write(" |")
					t.emitExpr(comm.Lhs[0])
					t.write("| => {")
				} else if len(comm.Lhs) == 2 {
					t.write(", _) |")
					t.emitExpr(comm.Lhs[0])
					t.write(", ")
					t.emitExpr(comm.Lhs[1])
					t.write("| => {")
				}
			}
		}
	default:
		t.todo(fmt.Sprintf("select comm %T", comm), cc)
	}
	t.nl()
	t.in()
	for _, st := range cc.Body {
		t.pad()
		t.emitStmt(st)
		if !bytes.HasSuffix(t.buf.Bytes(), []byte("\n")) {
			t.nl()
		}
	}
	t.out()
	t.pad()
	t.writeln("},")
}

func (t *Transpiler) emitBranchStmt(s *ast.BranchStmt) {
	switch s.Tok {
	case token.BREAK:
		t.write("break;")
	case token.CONTINUE:
		t.write("continue;")
	case token.GOTO:
		t.todo("goto — Rust has no goto; refactor the control flow", s)
		t.write("/* goto */")
	case token.FALLTHROUGH:
		t.todo("fallthrough — match doesn't support; refactor the case", s)
		t.write("/* fallthrough */")
	}
	if s.Label != nil {
		t.todo(fmt.Sprintf("labeled branch %q", s.Label.Name), s)
	}
}

// ── expressions ────────────────────────────────────────────────────────

func (t *Transpiler) emitExpr(e ast.Expr) {
	switch e := e.(type) {
	case *ast.Ident:
		t.emitIdent(e)
	case *ast.BasicLit:
		t.emitBasicLit(e)
	case *ast.BinaryExpr:
		t.emitExpr(e.X)
		t.write(" " + e.Op.String() + " ")
		t.emitExpr(e.Y)
	case *ast.UnaryExpr:
		t.emitUnaryExpr(e)
	case *ast.ParenExpr:
		t.write("(")
		t.emitExpr(e.X)
		t.write(")")
	case *ast.SelectorExpr:
		t.emitSelectorExpr(e)
	case *ast.CallExpr:
		t.emitCallExpr(e)
	case *ast.IndexExpr:
		t.emitExpr(e.X)
		t.write("[")
		t.emitExpr(e.Index)
		t.write("]")
	case *ast.SliceExpr:
		t.emitSliceExpr(e)
	case *ast.StarExpr:
		t.write("*")
		t.emitExpr(e.X)
	case *ast.CompositeLit:
		t.emitCompositeLit(e)
	case *ast.KeyValueExpr:
		t.emitExpr(e.Key)
		t.write(": ")
		t.emitExpr(e.Value)
	case *ast.TypeAssertExpr:
		t.todo("type assertion — Rust requires match/as for this; hand-port", e)
		t.emitExpr(e.X)
	case *ast.FuncLit:
		t.emitFuncLit(e)
	case *ast.ArrayType, *ast.MapType, *ast.ChanType, *ast.StructType, *ast.InterfaceType, *ast.FuncType:
		// Type used as value (e.g., in make([]int, ...)).
		t.emitType(e)
	default:
		t.todo(fmt.Sprintf("expr %T", e), e)
	}
}

func (t *Transpiler) emitIdent(id *ast.Ident) {
	switch id.Name {
	case "nil":
		t.write("nil")
		return
	case "true":
		t.write("true")
		return
	case "false":
		t.write("false")
		return
	case "iota":
		t.write("iota")
		return
	}
	// Map Go primitive type names used as values (rare — happens in
	// conversions like int(x) handled in emitCallExpr). Otherwise, bare
	// identifiers pass through.
	if rs, ok := primitiveTypeMap[id.Name]; ok {
		t.write(rs)
		return
	}
	// Package-level var emitted as a fn — rewrite bare use to a call.
	if t.fnVars != nil && t.fnVars[id.Name] {
		t.write(id.Name)
		t.write("()")
		return
	}
	t.write(id.Name)
}

func (t *Transpiler) emitBasicLit(lit *ast.BasicLit) {
	switch lit.Kind {
	case token.INT:
		t.write(lit.Value)
	case token.FLOAT:
		t.write(lit.Value)
	case token.IMAG:
		t.todo("imaginary literal — no Goish equivalent", lit)
	case token.STRING:
		// Go allows both "..." and `...` (raw). Rust raw strings use r#"..."#.
		s := lit.Value
		if strings.HasPrefix(s, "`") && strings.HasSuffix(s, "`") {
			inner := s[1 : len(s)-1]
			// Prefer a safe hash count in case inner contains `"#`.
			hash := strings.Repeat("#", pickRawHash(inner))
			t.writef("r%s\"%s\"%s", hash, inner, hash)
		} else {
			t.write(s)
		}
	case token.CHAR:
		t.write(lit.Value)
	default:
		t.write(lit.Value)
	}
}

func pickRawHash(s string) int {
	n := 1
	for strings.Contains(s, "\""+strings.Repeat("#", n)) {
		n++
	}
	return n
}

func (t *Transpiler) emitUnaryExpr(e *ast.UnaryExpr) {
	switch e.Op {
	case token.ADD:
		t.write("+")
		t.emitExpr(e.X)
	case token.SUB:
		t.write("-")
		t.emitExpr(e.X)
	case token.NOT:
		t.write("!")
		t.emitExpr(e.X)
	case token.XOR:
		t.write("!") // Go's ^ (bitwise NOT) is ! in Rust for integers
		t.emitExpr(e.X)
	case token.AND:
		t.write("&")
		t.emitExpr(e.X)
	case token.ARROW:
		// Outside of select/assign context: `<-ch` as value expression.
		// Emit as .Recv().0 so callers get the value; flag for user review.
		t.emitExpr(e.X)
		t.write(".Recv().0")
	default:
		t.write(e.Op.String())
		t.emitExpr(e.X)
	}
}

func (t *Transpiler) emitSelectorExpr(e *ast.SelectorExpr) {
	// pkg.X when pkg is an import alias → pkg::X
	if id, ok := e.X.(*ast.Ident); ok {
		if pkg, ok := t.importAliases[id.Name]; ok {
			t.write(pkg)
			t.write("::")
			t.write(e.Sel.Name)
			return
		}
	}
	t.emitExpr(e.X)
	t.write(".")
	t.write(e.Sel.Name)
}

func (t *Transpiler) emitSliceExpr(e *ast.SliceExpr) {
	if e.Slice3 {
		t.todo("three-index slice s[i:j:k] — deferred in Goish", e)
		t.emitExpr(e.X)
		return
	}
	t.emitExpr(e.X)
	switch {
	case e.Low == nil && e.High == nil:
		t.write(".Slice(0, len!(")
		t.emitExpr(e.X)
		t.write("))")
	case e.Low == nil:
		t.write(".SliceTo(")
		t.emitExpr(e.High)
		t.write(")")
	case e.High == nil:
		t.write(".SliceFrom(")
		t.emitExpr(e.Low)
		t.write(")")
	default:
		t.write(".Slice(")
		t.emitExpr(e.Low)
		t.write(", ")
		t.emitExpr(e.High)
		t.write(")")
	}
}

func (t *Transpiler) emitCallExpr(e *ast.CallExpr) {
	// 1. Builtin rewrites: len/cap/append/copy/delete/close/make
	if id, ok := e.Fun.(*ast.Ident); ok {
		if _, isBuiltin := builtinCallMap[id.Name]; isBuiltin {
			t.emitBuiltinCall(id.Name, e)
			return
		}
		switch id.Name {
		case "panic":
			t.write("panic!(\"{}\", ")
			if len(e.Args) > 0 {
				t.emitExpr(e.Args[0])
			}
			t.write(")")
			return
		case "recover":
			t.todo("recover — use recover!{ body } block form; cannot be rewritten from bare call", e)
			t.write("recover!()")
			return
		case "print", "println":
			t.write("Println!(")
			for i, a := range e.Args {
				if i > 0 {
					t.write(", ")
				}
				t.emitExpr(a)
			}
			t.write(")")
			return
		case "new":
			// new(T) → <T>::default() (approximation; user may want Box/Arc).
			t.write("<")
			if len(e.Args) > 0 {
				t.emitType(e.Args[0])
			} else {
				t.write("()")
			}
			t.write(" as ::std::default::Default>::default()")
			return
		}
		// Primitive type conversion: int(x) / string(x) / byte(x) / etc.
		if _, isPrim := primitiveTypeMap[id.Name]; isPrim && len(e.Args) == 1 {
			if id.Name == "string" {
				// Go: string(b) converts []byte or rune → string
				t.write("string::from(")
				t.emitExpr(e.Args[0])
				t.write(")")
				return
			}
			t.write("(")
			t.emitExpr(e.Args[0])
			t.write(" as ")
			t.write(primitiveTypeMap[id.Name])
			t.write(")")
			return
		}
	}

	// 2. fmt.* macro rewrites.
	if sel, ok := e.Fun.(*ast.SelectorExpr); ok {
		if id, ok := sel.X.(*ast.Ident); ok {
			pkgName := id.Name
			if mod, known := t.importAliases[pkgName]; known && mod == "fmt" {
				switch sel.Sel.Name {
				case "Println", "Printf", "Sprintf", "Errorf":
					t.writef("%s!(", sel.Sel.Name)
					for i, a := range e.Args {
						if i > 0 {
							t.write(", ")
						}
						t.emitExpr(a)
					}
					t.write(")")
					return
				case "Fprintf", "Fprintln", "Fprint":
					t.writef("%s!(", sel.Sel.Name)
					for i, a := range e.Args {
						if i > 0 {
							t.write(", ")
						}
						t.emitExpr(a)
					}
					t.write(")")
					return
				case "Sprint", "Sprintln":
					t.write("Sprintf!(\"{}\", ")
					if len(e.Args) > 0 {
						t.emitExpr(e.Args[0])
					}
					t.write(")")
					return
				}
			}
			if mod, known := t.importAliases[pkgName]; known && mod == "log" {
				switch sel.Sel.Name {
				case "Println", "Printf", "Fatalf", "Fatal", "Panic", "Panicln":
					t.writef("log::%s!(", sel.Sel.Name)
					for i, a := range e.Args {
						if i > 0 {
							t.write(", ")
						}
						t.emitExpr(a)
					}
					t.write(")")
					return
				}
			}
		}
	}

	// 3a. reflect.ValueOf(x).IsNil() → x == nil.
	//
	// Goish's `error` is a single newtype — there's no typed-nil shape to
	// detect, so the Go idiom for guarding against nil-pointer-in-interface
	// collapses to a plain equality check. Accept both
	// `reflect.ValueOf(x).IsNil()` and the direct `reflect.Value.IsNil` on
	// the ValueOf result.
	if sel, ok := e.Fun.(*ast.SelectorExpr); ok && sel.Sel.Name == "IsNil" && len(e.Args) == 0 {
		if inner, ok := sel.X.(*ast.CallExpr); ok {
			if isReflectValueOf(inner) && len(inner.Args) == 1 {
				t.write("(")
				t.emitExpr(inner.Args[0])
				t.write(" == nil)")
				return
			}
		}
	}

	// 3. testing.T / testing.B Printf-style methods → wrap args in Sprintf!.
	if sel, ok := e.Fun.(*ast.SelectorExpr); ok && isTestingPrintfMethod(sel.Sel.Name) && len(e.Args) >= 1 {
		t.emitExpr(sel.X)
		t.writef(".%s(Sprintf!(", sel.Sel.Name)
		for i, a := range e.Args {
			if i > 0 {
				t.write(", ")
			}
			t.emitExpr(a)
		}
		t.write("))")
		return
	}

	// 4. Same-file variadic fn call — wrap trailing args in `&[...]`.
	if id, ok := e.Fun.(*ast.Ident); ok && t.variadicFns[id.Name] && !e.Ellipsis.IsValid() {
		t.write(id.Name)
		t.write("(&[")
		for i, a := range e.Args {
			if i > 0 {
				t.write(", ")
			}
			t.emitExpr(a)
		}
		t.write("])")
		return
	}

	// 5. Plain call.
	t.emitExpr(e.Fun)
	t.write("(")
	for i, a := range e.Args {
		if i > 0 {
			t.write(", ")
		}
		t.emitExpr(a)
	}
	if e.Ellipsis.IsValid() {
		// Go variadic splat — no direct Rust form. Flag.
		t.todo("variadic splat f(xs...) — Rust has no splat; rewrite to explicit args or .extend", e)
		t.write("/* ... */")
	}
	t.write(")")
}

// isReflectValueOf matches `reflect.ValueOf(...)` — regardless of local
// import alias, since we only inspect the outer Sel name.
func isReflectValueOf(c *ast.CallExpr) bool {
	sel, ok := c.Fun.(*ast.SelectorExpr)
	if !ok || sel.Sel.Name != "ValueOf" {
		return false
	}
	id, ok := sel.X.(*ast.Ident)
	if !ok || id.Name != "reflect" {
		return false
	}
	return true
}

// isTestingPrintfMethod returns true for testing.T / testing.B methods that
// take a Go Printf-style `(format, args...)` signature. Goish's equivalents
// take a single pre-formatted `string`, so we wrap the args in Sprintf!.
func isTestingPrintfMethod(name string) bool {
	switch name {
	case "Errorf", "Fatalf", "Logf", "Skipf":
		return true
	}
	return false
}

func (t *Transpiler) emitBuiltinCall(name string, e *ast.CallExpr) {
	switch name {
	case "len", "cap", "close", "copy", "delete":
		t.writef("%s!(", name)
		for i, a := range e.Args {
			if i > 0 {
				t.write(", ")
			}
			t.emitExpr(a)
		}
		t.write(")")
	case "append":
		t.write("append!(")
		for i, a := range e.Args {
			if i > 0 {
				t.write(", ")
			}
			t.emitExpr(a)
		}
		if e.Ellipsis.IsValid() {
			t.todo("append(s, xs...) — rewrite to explicit args or .extend_from_slice", e)
		}
		t.write(")")
	case "make":
		// make([]T, n) / make([]T, n, c) / make(map[K]V) / make(chan T[, n])
		// The first arg is a type expression — emit using make!'s syntax.
		if len(e.Args) == 0 {
			t.write("make!()")
			return
		}
		t.write("make!(")
		// Type rendering for make! uses make!([]T, n), make!(map[K]V), make!(chan T).
		t.emitMakeType(e.Args[0])
		for _, a := range e.Args[1:] {
			t.write(", ")
			t.emitExpr(a)
		}
		t.write(")")
	}
}

func (t *Transpiler) emitMakeType(e ast.Expr) {
	// Expected shapes: ArrayType (slice — no Len), MapType, ChanType.
	switch ty := e.(type) {
	case *ast.ArrayType:
		if ty.Len != nil {
			t.todo("make of fixed-length array", ty)
		}
		t.write("[]")
		t.emitType(ty.Elt)
	case *ast.MapType:
		t.write("map[")
		t.emitType(ty.Key)
		t.write("]")
		t.emitType(ty.Value)
	case *ast.ChanType:
		t.write("chan ")
		t.emitType(ty.Value)
	default:
		t.emitType(e)
	}
}

func (t *Transpiler) emitCompositeLit(e *ast.CompositeLit) {
	switch ty := e.Type.(type) {
	case *ast.ArrayType:
		// []T{a, b, c}  → slice!([]T{a, b, c})
		if ty.Len != nil {
			t.todo("fixed-size array literal", e)
		}
		t.write("slice!([]")
		t.emitType(ty.Elt)
		t.write("{")
		for i, el := range e.Elts {
			if i > 0 {
				t.write(", ")
			}
			t.emitElemWithHint(el, ty.Elt)
		}
		t.write("})")
	case *ast.MapType:
		// map[K]V{k: v}  → map!([K]V{k => v})
		t.write("map!([")
		t.emitType(ty.Key)
		t.write("]")
		t.emitType(ty.Value)
		t.write("{")
		for i, el := range e.Elts {
			if i > 0 {
				t.write(", ")
			}
			kv := el.(*ast.KeyValueExpr)
			t.emitElemWithHint(kv.Key, ty.Key)
			t.write(" => ")
			t.emitElemWithHint(kv.Value, ty.Value)
		}
		t.write("})")
	case *ast.Ident:
		// StructName{...}. If all elements are KeyValueExpr, use named init.
		// Otherwise assume positional and use the Struct!-generated macro.
		named := len(e.Elts) > 0
		for _, el := range e.Elts {
			if _, ok := el.(*ast.KeyValueExpr); !ok {
				named = false
				break
			}
		}
		if named {
			t.write(ty.Name)
			t.write(" { ")
			for i, el := range e.Elts {
				if i > 0 {
					t.write(", ")
				}
				kv := el.(*ast.KeyValueExpr)
				t.emitExpr(kv.Key)
				t.write(": ")
				t.emitExpr(kv.Value)
			}
			t.write(" }")
			return
		}
		// Positional — use the companion `TypeName!` macro from Struct!.
		t.writef("%s!(", ty.Name)
		for i, el := range e.Elts {
			if i > 0 {
				t.write(", ")
			}
			t.emitExpr(el)
		}
		t.write(")")
	case *ast.SelectorExpr:
		// pkg.Type{...} — emit as named init; user can switch to positional.
		t.emitType(ty)
		t.write(" { ")
		for i, el := range e.Elts {
			if i > 0 {
				t.write(", ")
			}
			if kv, ok := el.(*ast.KeyValueExpr); ok {
				t.emitExpr(kv.Key)
				t.write(": ")
				t.emitExpr(kv.Value)
			} else {
				t.emitExpr(el)
			}
		}
		t.write(" }")
	default:
		t.todo(fmt.Sprintf("composite type %T", e.Type), e)
	}
}

// emitElemWithHint emits an expression with a known element-type hint,
// used for composite-literal elements where Go elides the inner type
// (e.g. `[]PathTest{{"a", "A"}, {"b", "B"}}` — inner `{...}` has nil Type).
func (t *Transpiler) emitElemWithHint(e ast.Expr, hint ast.Expr) {
	cl, ok := e.(*ast.CompositeLit)
	if !ok || cl.Type != nil {
		t.emitExpr(e)
		return
	}
	// Patch the elided type from the hint and re-dispatch.
	patched := *cl
	patched.Type = hint
	t.emitCompositeLit(&patched)
}

func (t *Transpiler) emitFuncLit(f *ast.FuncLit) {
	// Go: func(a int) int { body }  → |a: int| -> int { body }
	t.write("|")
	if f.Type.Params != nil {
		first := true
		for _, p := range f.Type.Params.List {
			for _, n := range p.Names {
				if !first {
					t.write(", ")
				}
				first = false
				t.write(n.Name)
				t.write(": ")
				t.emitType(p.Type)
			}
		}
	}
	t.write("|")
	if f.Type.Results != nil && len(f.Type.Results.List) > 0 {
		t.emitResults(f.Type.Results)
	}
	t.writeln(" {")
	t.in()
	if f.Body != nil {
		t.emitBlockBody(f.Body)
	}
	t.out()
	t.pad()
	t.write("}")
}

// ── types ──────────────────────────────────────────────────────────────

func (t *Transpiler) emitType(e ast.Expr) {
	switch ty := e.(type) {
	case *ast.Ident:
		if rs, ok := primitiveTypeMap[ty.Name]; ok {
			t.write(rs)
			return
		}
		t.write(ty.Name)
	case *ast.SelectorExpr:
		// pkg.Type — map pkg alias if known.
		if id, ok := ty.X.(*ast.Ident); ok {
			if mod, known := t.importAliases[id.Name]; known {
				t.write(mod)
				t.write("::")
				t.write(ty.Sel.Name)
				return
			}
		}
		t.emitExpr(ty.X)
		t.write("::")
		t.write(ty.Sel.Name)
	case *ast.ArrayType:
		if ty.Len == nil {
			t.write("slice<")
			t.emitType(ty.Elt)
			t.write(">")
			return
		}
		// [N]T → [T; N]
		t.write("[")
		t.emitType(ty.Elt)
		t.write("; ")
		t.emitExpr(ty.Len)
		t.write("]")
	case *ast.MapType:
		t.write("map<")
		t.emitType(ty.Key)
		t.write(", ")
		t.emitType(ty.Value)
		t.write(">")
	case *ast.ChanType:
		t.write("Chan<")
		t.emitType(ty.Value)
		t.write(">")
	case *ast.StarExpr:
		// *T — Goish maps pointer → & reference by default. Flag if ambiguity matters.
		t.write("&")
		t.emitType(ty.X)
	case *ast.InterfaceType:
		if ty.Methods == nil || len(ty.Methods.List) == 0 {
			// `interface{}` / `any` — Rust has no 1:1 runtime-any, but
			// `Box<dyn Any + Send + Sync>` lands closest for field-typed use.
			t.write("Box<dyn ::std::any::Any + Send + Sync>")
			return
		}
		t.write("Box<dyn ::std::any::Any + Send + Sync>")
	case *ast.StructType:
		t.todo("inline struct type — port to named Struct!", ty)
		t.write("()")
	case *ast.FuncType:
		t.write("impl Fn(")
		if ty.Params != nil {
			first := true
			for _, p := range ty.Params.List {
				count := len(p.Names)
				if count == 0 {
					count = 1
				}
				for i := 0; i < count; i++ {
					if !first {
						t.write(", ")
					}
					first = false
					t.emitType(p.Type)
				}
			}
		}
		t.write(")")
		if ty.Results != nil && len(ty.Results.List) > 0 {
			t.emitResults(ty.Results)
		}
	case *ast.Ellipsis:
		// variadic param type — Goish: &[T]
		t.write("&[")
		t.emitType(ty.Elt)
		t.write("]")
	default:
		t.todo(fmt.Sprintf("type %T", e), e)
		t.write("()")
	}
}

// ── utilities ──────────────────────────────────────────────────────────

// embeddedFieldName derives a Go-style field name for an anonymous embedded
// field. Go's rules: `*sync.Mutex` → `Mutex`, `sync.Mutex` → `Mutex`,
// `Logger` → `Logger`. The resulting identifier is usable as a field name.
func embeddedFieldName(e ast.Expr) string {
	switch ty := e.(type) {
	case *ast.Ident:
		return ty.Name
	case *ast.StarExpr:
		return embeddedFieldName(ty.X)
	case *ast.SelectorExpr:
		return ty.Sel.Name
	}
	return "_embed"
}

// goTypeString renders a Go type for use in a comment (cosmetic only).
func goTypeString(e ast.Expr) string {
	switch ty := e.(type) {
	case *ast.Ident:
		return ty.Name
	case *ast.StarExpr:
		return "*" + goTypeString(ty.X)
	case *ast.SelectorExpr:
		return goTypeString(ty.X) + "." + ty.Sel.Name
	}
	return "?"
}

func containsIdent(e ast.Expr, name string) bool {
	found := false
	ast.Inspect(e, func(n ast.Node) bool {
		if id, ok := n.(*ast.Ident); ok && id.Name == name {
			found = true
			return false
		}
		return true
	})
	return found
}
