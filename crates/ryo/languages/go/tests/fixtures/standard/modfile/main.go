// Command modfile prints what golang.org/x/mod/modfile, the Go project's own reader of go.mod files, makes of each go.mod it is
// given, in the words of the ryo go language and in the format of ../dump.go: a line `file <hash> <length>`, then the tree as lines
// `tree DEPTH N|T KIND START END`.
//
// The nodes are modfile's: the file (`mod-file`), each line (`mod-line`, from its first word to its last, as modfile.Line spans it)
// and each block (`mod-line-block`, from its verb to its `)`). The tokens are its words (`mod-word`, each found in order from the
// start of its line, as modfile.Line.Token holds them), the parentheses of a block (`mod-l-paren`, `mod-r-paren`, at the places
// modfile.LineBlock gives) and its comments (`mod-comment`, at modfile.Comment.Start, as long as its token); the bytes between them
// are `whitespace`, a run cut after each line end. A token is under the innermost node that holds it. No fact is written: go.mod
// holds none of the facts dump.go writes.
//
// Run from this directory as `go run . FILE... >> ../output.txt`, the files relative to tests/fixtures.
package main

import (
	"bufio"
	"fmt"
	"os"
	"path/filepath"
	"sort"

	"golang.org/x/mod/modfile"
)

// element is a node or a token of the tree: its kind and the bytes it spans.
type element struct {
	node       bool
	kind       string
	start, end int
}

func fnv(data []byte) uint64 {
	hash := uint64(0xcbf29ce484222325)
	for _, b := range data {
		hash ^= uint64(b)
		hash *= 0x100000001b3
	}
	return hash
}

func space(b byte) bool { return b == ' ' || b == '\t' || b == '\r' || b == '\n' }

// words finds each token of a line in order, from `at`, skipping the white space between them.
func words(data []byte, at int, tokens []string, into *[]element) int {
	for _, token := range tokens {
		for at < len(data) && space(data[at]) {
			at++
		}
		*into = append(*into, element{kind: "mod-word", start: at, end: at + len(token)})
		at += len(token)
	}
	return at
}

func comments(c modfile.Comments, into *[]element) {
	for _, group := range [][]modfile.Comment{c.Before, c.Suffix, c.After} {
		for _, each := range group {
			*into = append(*into, element{kind: "mod-comment", start: each.Start.Byte, end: each.Start.Byte + len(each.Token)})
		}
	}
}

// line is a modfile.Line: its node and its words (a line of a block holds no verb: the block's tokens hold it), and its comments.
func line(data []byte, l *modfile.Line, into *[]element) {
	*into = append(*into, element{node: true, kind: "mod-line", start: l.Start.Byte, end: l.End.Byte})
	words(data, l.Start.Byte, l.Token, into)
	comments(l.Comments, into)
}

func tree(data []byte, file *modfile.FileSyntax) []element {
	elements := []element{{node: true, kind: "mod-file", start: 0, end: len(data)}}
	comments(file.Comments, &elements)
	for _, statement := range file.Stmt {
		switch each := statement.(type) {
		case *modfile.Line:
			line(data, each, &elements)
		case *modfile.LineBlock:
			elements = append(elements, element{node: true, kind: "mod-line-block", start: each.Start.Byte, end: each.RParen.Pos.Byte + 1})
			words(data, each.Start.Byte, each.Token, &elements)
			elements = append(elements, element{kind: "mod-l-paren", start: each.LParen.Pos.Byte, end: each.LParen.Pos.Byte + 1})
			elements = append(elements, element{kind: "mod-r-paren", start: each.RParen.Pos.Byte, end: each.RParen.Pos.Byte + 1})
			comments(each.Comments, &elements)
			for _, inner := range each.Line {
				line(data, inner, &elements)
			}
		case *modfile.CommentBlock:
			comments(each.Comments, &elements)
		}
	}
	// The bytes no token covers are white space, a run cut after each line end.
	covered := make([]bool, len(data))
	for _, each := range elements {
		if !each.node {
			for at := each.start; at < each.end && at < len(data); at++ {
				covered[at] = true
			}
		}
	}
	for at := 0; at < len(data); {
		if covered[at] {
			at++
			continue
		}
		end := at
		for end < len(data) && !covered[end] {
			end++
			if data[end-1] == '\n' {
				break
			}
		}
		elements = append(elements, element{kind: "whitespace", start: at, end: end})
		at = end
	}
	sort.SliceStable(elements, func(i, j int) bool {
		a, b := elements[i], elements[j]
		if a.start != b.start {
			return a.start < b.start
		}
		if a.node != b.node {
			return a.node
		}
		return a.end > b.end
	})
	return elements
}

func main() {
	out := bufio.NewWriter(os.Stdout)
	defer out.Flush()
	for _, name := range os.Args[1:] {
		data, err := os.ReadFile(filepath.Join("..", "..", name))
		if err != nil {
			fmt.Fprintln(os.Stderr, err)
			os.Exit(1)
		}
		parsed, err := modfile.ParseLax(name, data, nil)
		if err != nil {
			fmt.Fprintln(os.Stderr, err)
			os.Exit(1)
		}
		fmt.Fprintf(out, "file %016x %d\n", fnv(data), len(data))
		var open []element
		for _, each := range tree(data, parsed.Syntax) {
			for len(open) > 0 && open[len(open)-1].end <= each.start {
				open = open[:len(open)-1]
			}
			mark := "T"
			if each.node {
				mark = "N"
			}
			fmt.Fprintf(out, "tree %d %s %s %d %d\n", len(open), mark, each.kind, each.start, each.end)
			if each.node {
				open = append(open, each)
			}
		}
	}
}
