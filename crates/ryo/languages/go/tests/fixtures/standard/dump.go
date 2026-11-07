// Command dump prints what the standard library's go/parser, go/ast, go/scanner and go/build say of Go files, in the words of the ryo
// go language: the tree each file makes, and the facts the language's rows are compared with.
//
// Run from the directory tests/fixtures as `go run standard/dump.go FILE... > standard/output.txt`.
//
// A file is a line `file <hash> <length>`, the hash being the 64-bit FNV-1a of its bytes in hex, followed by ` refused` when
// go/parser reports an error in it, and then its lines, each a word and fields separated by one space.
//
// The tree is the lines `tree DEPTH N|T KIND START END`: an element of the tree in pre-order (N a node, T a token), its depth from the
// root (0), and its byte range. The nodes are the go/ast nodes of the file (the partial tree go/parser makes of a file it refuses),
// comments left out, the root spanning the whole file, and nested by their ranges. The tokens are go/scanner's tokens, the comments
// included and the semicolons the scanner inserts at a line end left out, and the text between two tokens is a `whitespace` token. A
// token is under the innermost node that holds it.
//
// The facts are lines of the words below. Every text is an `x` and the hex of its bytes, so that no escaping is needed and the empty
// text is a field; a position is `line:column`, both counted from one, the column in bytes, as go/token counts them. A file go/parser
// refuses has the tree and no fact.
//
//	comment <from> <to> <text>                    without its delimiters
//	constraint <from> <to> <kind> <text>          a build constraint line of the header, as go/build reads one: its kind (`go:build` or `+build`) and its expression
//	string <from> <to> <value|?>                  the decoded value of a string literal; ? when it is no UTF-8 text
//	import <from> <to> <binding> <path>           the binding is the alias, `?` for an import with none (its package names it)
//	function <from> <to> <name>
//	parameter <from> <to> <kind> <ordinal> <name> of a function, a function literal or a method of an interface; kind is receiver,
//	                                              positional, variadic or result; the name is ? for none
//	call <from> <to> <target>                     the span is the callee's; the target is ? when the callee is no dotted name
//	role <start> <end> <N|T> <kind> <field>       the field of go/ast that holds an element of the tree, by the bytes it spans and
//	                                              its kind as in the tree: for a node, the field of its go/ast parent that holds it
//	                                              (`Lhs` is lhs); for a token, the token.Pos field of the innermost node that says
//	                                              where it is, `Pos` cut from its end (`OpPos` is op), the first such field of the
//	                                              node when two name one token
//
// parseFileHeader is go/build's own function of that name (go/build/build.go, Go 1.26.5, BSD 3-Clause), cut to the lines it reads.
package main

import (
	"bytes"
	"encoding/hex"
	"fmt"
	"go/ast"
	"go/build/constraint"
	"go/parser"
	"go/scanner"
	"go/token"
	"hash/fnv"
	"os"
	"reflect"
	"sort"
	"strconv"
	"strings"
	"unicode"
	"unicode/utf8"
)

// names are go/token's constants, in kebab case; the four that are also a go/ast node end in -token.
var names = map[token.Token]string{
	token.ILLEGAL: "illegal", token.EOF: "eof", token.COMMENT: "comment-token", token.IDENT: "ident-token", token.INT: "int",
	token.FLOAT: "float", token.IMAG: "imag", token.CHAR: "char", token.STRING: "string", token.ADD: "add", token.SUB: "sub",
	token.MUL: "mul", token.QUO: "quo", token.REM: "rem", token.AND: "and", token.OR: "or", token.XOR: "xor", token.SHL: "shl",
	token.SHR: "shr", token.AND_NOT: "and-not", token.ADD_ASSIGN: "add-assign", token.SUB_ASSIGN: "sub-assign",
	token.MUL_ASSIGN: "mul-assign", token.QUO_ASSIGN: "quo-assign", token.REM_ASSIGN: "rem-assign", token.AND_ASSIGN: "and-assign",
	token.OR_ASSIGN: "or-assign", token.XOR_ASSIGN: "xor-assign", token.SHL_ASSIGN: "shl-assign", token.SHR_ASSIGN: "shr-assign",
	token.AND_NOT_ASSIGN: "and-not-assign", token.LAND: "land", token.LOR: "lor", token.ARROW: "arrow", token.INC: "inc",
	token.DEC: "dec", token.EQL: "eql", token.LSS: "lss", token.GTR: "gtr", token.ASSIGN: "assign", token.NOT: "not",
	token.NEQ: "neq", token.LEQ: "leq", token.GEQ: "geq", token.DEFINE: "define", token.ELLIPSIS: "ellipsis-token",
	token.LPAREN: "lparen", token.LBRACK: "lbrack", token.LBRACE: "lbrace", token.COMMA: "comma", token.PERIOD: "period",
	token.RPAREN: "rparen", token.RBRACK: "rbrack", token.RBRACE: "rbrace", token.SEMICOLON: "semicolon", token.COLON: "colon",
	token.BREAK: "break", token.CASE: "case", token.CHAN: "chan", token.CONST: "const", token.CONTINUE: "continue",
	token.DEFAULT: "default", token.DEFER: "defer", token.ELSE: "else", token.FALLTHROUGH: "fallthrough", token.FOR: "for",
	token.FUNC: "func", token.GO: "go", token.GOTO: "goto", token.IF: "if", token.IMPORT: "import",
	token.INTERFACE: "interface", token.MAP: "map", token.PACKAGE: "package-token", token.RANGE: "range",
	token.RETURN: "return", token.SELECT: "select", token.STRUCT: "struct", token.SWITCH: "switch", token.TYPE: "type",
	token.VAR: "var", token.TILDE: "tilde",
}

// element is a node or a token of the tree.
type element struct {
	node       bool
	kind       string
	start, end int
}

// kebab spells a go/ast type name in kebab case: FuncDecl is func-decl.
func kebab(name string) string {
	var out strings.Builder
	for i, r := range name {
		if unicode.IsUpper(r) {
			if i > 0 {
				out.WriteByte('-')
			}
			out.WriteRune(unicode.ToLower(r))
		} else {
			out.WriteRune(r)
		}
	}
	return out.String()
}

// tokensOf are the tokens go/scanner reads in src, without the semicolons it inserts, and the white space between them.
func tokensOf(src []byte) []element {
	fset := token.NewFileSet()
	file := fset.AddFile("", fset.Base(), len(src))
	var scan scanner.Scanner
	scan.Init(file, src, func(token.Position, string) {}, scanner.ScanComments)
	var found []element
	at := 0
	for {
		pos, tok, lit := scan.Scan()
		if tok == token.EOF {
			break
		}
		start := file.Offset(pos)
		if tok == token.SEMICOLON && lit == "\n" {
			continue
		}
		var end int
		switch {
		case tok == token.COMMENT && strings.HasPrefix(string(src[start:]), "//"):
			end = start + strings.IndexByte(string(src[start:]), '\n')
			if end < start {
				end = len(src)
			}
			if end > start && src[end-1] == '\r' {
				end--
			}
		case tok == token.COMMENT:
			if i := strings.Index(string(src[start+2:]), "*/"); i >= 0 {
				end = start + 2 + i + 2
			} else {
				end = len(src)
			}
		case tok == token.STRING && src[start] == '`':
			end = start + 1 + strings.IndexByte(string(src[start+1:]), '`') + 1
			if end <= start+1 {
				end = len(src)
			}
		case tok == token.ILLEGAL:
			end = start + max(len(lit), 1)
		case lit != "" && tok != token.SEMICOLON:
			end = start + len(lit)
		default:
			end = start + len(tok.String())
		}
		end = min(end, len(src))
		if start > at {
			found = append(found, element{kind: "whitespace", start: at, end: start})
		}
		found = append(found, element{kind: names[tok], start: start, end: end})
		at = end
	}
	if at < len(src) {
		found = append(found, element{kind: "whitespace", start: at, end: len(src)})
	}
	return found
}

// nodesOf are the go/ast nodes of the file in pre-order, comments left out, and the file spanning all of src.
func nodesOf(fset *token.FileSet, tree *ast.File, length int) []element {
	var found []element
	ast.Inspect(tree, func(n ast.Node) bool {
		switch n.(type) {
		case nil:
			return false
		case *ast.Comment, *ast.CommentGroup:
			return false
		}
		start, end := fset.Position(n.Pos()).Offset, fset.Position(n.End()).Offset
		if _, ok := n.(*ast.File); ok {
			start, end = 0, length
		}
		found = append(found, element{node: true, kind: kebab(reflect.TypeOf(n).Elem().Name()), start: start, end: min(end, length)})
		return true
	})
	// The tree is the nesting of the nodes' ranges: a node that starts earlier or, from the same place, ends later holds the other, and of
	// two ranges the same the one met first in pre-order holds the other. (go/ast visits a function's name before its type, which starts
	// at `func`.)
	sort.SliceStable(found, func(i, j int) bool {
		if found[i].start != found[j].start {
			return found[i].start < found[j].start
		}
		return found[i].end > found[j].end
	})
	return found
}

// tree nests the tokens in the nodes and writes the elements in pre-order.
func tree(out *strings.Builder, nodes, tokens []element) {
	var open []element
	closeTo := func(upto int) {
		for len(open) > 1 && open[len(open)-1].end <= upto {
			open = open[:len(open)-1]
		}
	}
	write := func(e element) {
		letter := "T"
		if e.node {
			letter = "N"
		}
		fmt.Fprintf(out, "tree %d %s %s %d %d\n", len(open), letter, e.kind, e.start, e.end)
	}
	n, t := 0, 0
	for n < len(nodes) || t < len(tokens) {
		if n < len(nodes) && (t >= len(tokens) || nodes[n].start <= tokens[t].start) {
			if len(open) > 0 {
				closeTo(nodes[n].start)
			}
			write(nodes[n])
			open = append(open, nodes[n])
			n++
		} else {
			closeTo(tokens[t].start)
			write(tokens[t])
			t++
		}
	}
}

// text is a text as the dump writes it: an x, and the hex of its bytes. The x keeps the empty text a field of its own.
func text(value string) string { return "x" + hex.EncodeToString([]byte(value)) }

// inner is a comment without its delimiters.
func inner(c string) string {
	if strings.HasPrefix(c, "//") {
		return c[2:]
	}
	return strings.TrimSuffix(strings.TrimPrefix(c, "/*"), "*/")
}

// dotted is the dotted name an expression writes: a name, or a selection from one, instantiated with several types or not; "" for any
// other. One index may be of a slice or of a map, so a call of f[T]() is no dotted name.
func dotted(expr ast.Expr) string {
	switch e := expr.(type) {
	case *ast.Ident:
		return e.Name
	case *ast.SelectorExpr:
		if base := dotted(e.X); base != "" {
			return base + "." + e.Sel.Name
		}
	case *ast.IndexListExpr:
		return dotted(e.X)
	}
	return ""
}

type dump struct {
	fset *token.FileSet
	out  *strings.Builder
}

func (d dump) place(from, to token.Pos) string {
	a, b := d.fset.Position(from), d.fset.Position(to)
	return fmt.Sprintf("%d:%d %d:%d", a.Line, a.Column, b.Line, b.Column)
}

// span is the line and column of the bytes start..end of src.
func span(src []byte, start, end int) string {
	point := func(at int) string {
		line := 1 + bytes.Count(src[:at], []byte("\n"))
		column := at - (bytes.LastIndexByte(src[:at], '\n') + 1) + 1
		return fmt.Sprintf("%d:%d", line, column)
	}
	return point(start) + " " + point(end)
}

func (d dump) fields(kind string, list *ast.FieldList, variadicKind string) {
	if list == nil {
		return
	}
	ordinal := 0
	for _, field := range list.List {
		kindOf := kind
		if _, ok := field.Type.(*ast.Ellipsis); ok {
			kindOf = variadicKind
		}
		if len(field.Names) == 0 {
			if kind != "result" {
				fmt.Fprintf(d.out, "parameter %s %s %d ?\n", d.place(field.Type.Pos(), field.Type.End()), kindOf, ordinal)
			}
			ordinal++
			continue
		}
		for _, name := range field.Names {
			fmt.Fprintf(d.out, "parameter %s %s %d %s\n", d.place(name.Pos(), name.End()), kindOf, ordinal, text(name.Name))
			ordinal++
		}
	}
}

// parseFileHeader is go/build's: the part of a file where build constraints may be, up to the last blank line before the first line
// of code, and the `//go:build` line in it.
func parseFileHeader(content []byte) (trimmed, goBuild []byte, goBuildAt int) {
	end := 0
	p := content
	ended := false
	inSlashStar := false
	slashSlash, slashStar, starSlash := []byte("//"), []byte("/*"), []byte("*/")
Lines:
	for len(p) > 0 {
		line := p
		before := len(content) - len(p)
		if i := bytes.IndexByte(line, '\n'); i >= 0 {
			line, p = line[:i], p[i+1:]
		} else {
			p = p[len(p):]
		}
		lead := len(line) - len(bytes.TrimLeft(line, " \t\r\v\f"))
		line = bytes.TrimSpace(line)
		if len(line) == 0 && !ended {
			end = len(content) - len(p)
			continue Lines
		}
		if !bytes.HasPrefix(line, slashSlash) {
			ended = true
		}
		if !inSlashStar && constraint.IsGoBuild(string(line)) {
			if goBuild != nil {
				return nil, nil, 0
			}
			goBuild, goBuildAt = line, before+lead
		}
	Comments:
		for len(line) > 0 {
			if inSlashStar {
				if i := bytes.Index(line, starSlash); i >= 0 {
					inSlashStar = false
					line = bytes.TrimSpace(line[i+len(starSlash):])
					continue Comments
				}
				continue Lines
			}
			if bytes.HasPrefix(line, slashSlash) {
				continue Lines
			}
			if bytes.HasPrefix(line, slashStar) {
				inSlashStar = true
				line = bytes.TrimSpace(line[len(slashStar):])
				continue Comments
			}
			break Lines
		}
	}
	return content[:end], goBuild, goBuildAt
}

// directive splits a constraint line, without its `//`, into its kind and its expression.
func directive(line string) (string, string) {
	line = strings.TrimSpace(line)
	if end := strings.IndexAny(line, " \t\r\v\f"); end >= 0 {
		return line[:end], strings.TrimSpace(line[end:])
	}
	return line, ""
}

// constraints writes the build constraint lines of the header as go/build reads them: the `//go:build` line, and each `// +build` line
// above the last blank line of the header.
func (d dump) constraints(src []byte) {
	trimmed, goBuild, goBuildAt := parseFileHeader(src)
	if goBuild != nil {
		kind, expression := directive(string(goBuild[2:]))
		fmt.Fprintf(d.out, "constraint %s %s %s\n", span(src, goBuildAt, goBuildAt+len(goBuild)), text(kind), text(expression))
	}
	at := 0
	for _, line := range bytes.SplitAfter(trimmed, []byte("\n")) {
		body := bytes.TrimSpace(line)
		if constraint.IsPlusBuild(string(body)) {
			lead := len(line) - len(bytes.TrimLeft(line, " \t\r\v\f"))
			kind, expression := directive(string(body[2:]))
			fmt.Fprintf(d.out, "constraint %s %s %s\n", span(src, at+lead, at+lead+len(body)), text(kind), text(expression))
		}
		at += len(line)
	}
}

// facts writes what the library reads in the file.
func (d dump) facts(parsed *ast.File, src []byte) {
	for _, group := range parsed.Comments {
		for _, comment := range group.List {
			fmt.Fprintf(d.out, "comment %s %s\n", d.place(comment.Pos(), comment.End()), text(inner(comment.Text)))
		}
	}
	d.constraints(src)
	ast.Inspect(parsed, func(node ast.Node) bool {
		switch n := node.(type) {
		case *ast.BasicLit:
			if n.Kind == token.STRING {
				value, err := strconv.Unquote(n.Value)
				shown := "?"
				if err == nil && utf8.ValidString(value) {
					shown = text(value)
				}
				fmt.Fprintf(d.out, "string %s %s\n", d.place(n.Pos(), n.End()), shown)
			}
		case *ast.ImportSpec:
			path, _ := strconv.Unquote(n.Path.Value)
			binding := "?"
			if n.Name != nil {
				binding = text(n.Name.Name)
			}
			fmt.Fprintf(d.out, "import %s %s %s\n", d.place(n.Pos(), n.End()), binding, text(path))
		case *ast.FuncDecl:
			fmt.Fprintf(d.out, "function %s %s\n", d.place(n.Pos(), n.End()), text(n.Name.Name))
			d.fields("receiver", n.Recv, "receiver")
			d.fields("positional", n.Type.Params, "variadic")
			d.fields("result", n.Type.Results, "result")
		case *ast.FuncLit:
			d.fields("positional", n.Type.Params, "variadic")
			d.fields("result", n.Type.Results, "result")
		case *ast.InterfaceType:
			// The methods of an interface are functions with parameters, and no body.
			for _, method := range n.Methods.List {
				if signature, ok := method.Type.(*ast.FuncType); ok {
					d.fields("positional", signature.Params, "variadic")
					d.fields("result", signature.Results, "result")
				}
			}
		case *ast.CallExpr:
			switch n.Fun.(type) {
			case *ast.ArrayType, *ast.MapType, *ast.ChanType, *ast.FuncType, *ast.StructType, *ast.InterfaceType:
				// A conversion to a type written out is no call of a name.
				return true
			}
			target := dotted(n.Fun)
			if target == "" {
				fmt.Fprintf(d.out, "call %s ?\n", d.place(n.Fun.Pos(), n.Fun.End()))
			} else {
				fmt.Fprintf(d.out, "call %s %s\n", d.place(n.Fun.Pos(), n.Fun.End()), text(target))
			}
		}
		return true
	})
}

// posType and nodeType are the types of a position and of a node of go/ast.
var (
	posType  = reflect.TypeOf(token.Pos(0))
	nodeType = reflect.TypeOf((*ast.Node)(nil)).Elem()
)

// skipped are the fields of go/ast that hold no element of the tree where it is: the comments, which are tokens of the tree, the
// file's list of imports, which repeats specs of its declarations, and the bounds of the file, of an import and of a literal, which no token of the element starts.
var skipped = map[string]bool{"Doc": true, "Comment": true, "Comments": true, "Imports": true, "Unresolved": true, "FileStart": true, "FileEnd": true, "EndPos": true, "ValueEnd": true}

// role is a field's name as a role: in kebab case, `Pos` cut from its end.
func role(name string) string {
	if len(name) > 3 && strings.HasSuffix(name, "Pos") {
		name = strings.TrimSuffix(name, "Pos")
	}
	return kebab(name)
}

// roles writes the field of go/ast that holds each element of the tree.
func (d dump) roles(parsed *ast.File, tokens []element) {
	tokenAt := map[int]element{}
	for _, each := range tokens {
		if each.kind != "whitespace" {
			tokenAt[each.start] = each
		}
	}
	type claim struct {
		depth int
		name  string
	}
	claimed := map[int]claim{}
	var lines []string
	depth := 0
	var visit func(n ast.Node)
	visit = func(n ast.Node) {
		value := reflect.ValueOf(n).Elem()
		kind := value.Type()
		named := map[int]bool{}
		for i := 0; i < kind.NumField(); i++ {
			field, name := value.Field(i), kind.Field(i).Name
			if skipped[name] {
				continue
			}
			switch {
			case field.Type() == posType:
				pos := token.Pos(field.Int())
				if !pos.IsValid() {
					continue
				}
				at := d.fset.Position(pos).Offset
				if _, ok := tokenAt[at]; !ok || named[at] {
					continue
				}
				named[at] = true
				if before, ok := claimed[at]; !ok || before.depth < depth {
					claimed[at] = claim{depth, role(name)}
				}
			case field.Kind() == reflect.Slice && field.Type().Elem().Implements(nodeType):
				for j := 0; j < field.Len(); j++ {
					if child, ok := field.Index(j).Interface().(ast.Node); ok && !reflect.ValueOf(child).IsNil() {
						lines = append(lines, d.node(child, role(name)))
					}
				}
			case field.Type().Implements(nodeType) && !field.IsNil():
				if child, ok := field.Interface().(ast.Node); ok {
					switch child.(type) {
					case *ast.CommentGroup, *ast.Comment:
						continue
					}
					lines = append(lines, d.node(child, role(name)))
				}
			}
		}
	}
	ast.Inspect(parsed, func(n ast.Node) bool {
		switch n.(type) {
		case nil:
			depth--
			return false
		case *ast.Comment, *ast.CommentGroup:
			return false
		}
		visit(n)
		depth++
		return true
	})
	for at, each := range claimed {
		t := tokenAt[at]
		lines = append(lines, fmt.Sprintf("role %d %d T %s %s", t.start, t.end, t.kind, each.name))
	}
	sort.Strings(lines)
	for _, line := range lines {
		fmt.Fprintln(d.out, line)
	}
}

// node is the role line of a node.
func (d dump) node(n ast.Node, name string) string {
	start, end := d.fset.Position(n.Pos()).Offset, d.fset.Position(n.End()).Offset
	return fmt.Sprintf("role %d %d N %s %s", start, end, kebab(reflect.TypeOf(n).Elem().Name()), name)
}

func main() {
	for _, name := range os.Args[1:] {
		src, err := os.ReadFile(name)
		if err != nil {
			fmt.Fprintln(os.Stderr, err)
			os.Exit(1)
		}
		hash := fnv.New64a()
		hash.Write(src)
		head := fmt.Sprintf("file %016x %d", hash.Sum64(), len(src))
		fset := token.NewFileSet()
		parsed, err := parser.ParseFile(fset, name, src, parser.ParseComments|parser.SkipObjectResolution)
		if err != nil {
			head += " refused"
		}
		fmt.Println(head)
		if parsed == nil {
			continue
		}
		var out strings.Builder
		tree(&out, nodesOf(fset, parsed, len(src)), tokensOf(src))
		if err == nil {
			d := dump{fset: fset, out: &out}
			d.facts(parsed, src)
			d.roles(parsed, tokensOf(src))
		}
		fmt.Print(out.String())
	}
}
