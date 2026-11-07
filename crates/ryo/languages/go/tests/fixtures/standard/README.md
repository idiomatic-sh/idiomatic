# go standard

## Source

The Go project's own definitions and its own reading of Go, pinned.

- `ast.go` and `token.go`: the packages `go/ast` and `go/token` of Go 1.26.5, copied unchanged (go.dev, BSD 3-Clause, `NOTICE-LICENSE`). `ast.go` names the nodes and `token.go` the tokens. A token whose name is a node's has `-token` after it: `ident-token`, `comment-token`, `ellipsis-token`, `package-token`. The white space between tokens, which `go/scanner` gives no token for, is the language's extra `whitespace`.
- `dump.go`: a Go program written for this language that reads files with `go/parser`, scans them with `go/scanner` and reads their build constraints as `go/build` does (`parseFileHeader`, copied from `go/build/build.go` and cut to the lines it reads). For each file it writes the tree (the nodes of `go/ast`, the tokens of `go/scanner` with their comments, and the white space between them, nested by the bytes they span; the partial tree `go/parser` makes of a file it refuses) and the facts they read in it: `comment`, `constraint`, `string`, `import`, `function`, `parameter` and `call`, each with its place.
- `modfile/`: a Go program written for this language that reads each `go.mod` of the fixtures with `golang.org/x/mod/modfile` (the Go project's own reader of module files, pinned in `modfile/go.mod`) and writes its tree in the same lines: the file, each line and each block of `modfile`, their words, parentheses and comments, and the white space between them.
- `output.txt`: what `dump.go` wrote for every file of the language's fixtures, found in it by the file's length and its hash. The language's tree is held to the tree byte for byte (the conformance's shape rail reads `tree` lines through `fixtures/walk.rs`, and the adapter's `Pinned` case reads them again, with a control that a tree that is not `go/parser`'s is a difference), and its rows to the facts (`fixtures/oracle.rs`). To pin it again, from `tests/fixtures`:

      GO111MODULE=off go build -o "${TMPDIR:-/tmp}/dump" standard/dump.go && "${TMPDIR:-/tmp}/dump" $(find relations suite corpus -name '*.go' | sort) > standard/output.txt
      (cd standard/modfile && go run . $(cd ../.. && find relations suite corpus -name 'go.mod' -o -name 'input.go.mod' | sort)) >> standard/output.txt

  (`go run` is refused: the files it is given are in more than one directory.)

- `../suite/refused/`: files of Go's own test suite (`test/syntax/`) that `go/parser` refuses and whose partial tree the language does not reproduce (see below). `output.txt` marks each `refused`; the adapter's `Refused` case holds each to a syntax error reported by the language.

## Version

Go 1.26.5 (`go version go1.26.5`), the version of the pinned sources, of `go/parser`, `go/scanner` and `go/build`, and of the Go the grammar reads as (`tree-sitter-go` 0.25.0 reads Go 1.25).

## Differences from the standard

The language's tree is `go/parser`'s on every file of `relations/`, `corpus/` and `suite/` but these, each held out of the fixtures with its reason:

| file | where the language parts from `go/parser` |
|---|---|
| `suite/valid/newexpr.go`, `suite/valid/method7.go`, `suite/valid/typeparam/issue51521.go` | the grammar has no rule for what Go 1.26 allows (`new(123)`, `interface{ m1(string) }.m1(x)`): the library reports a syntax error in a valid file |
| `suite/refused/*.go` (16 files) | `go/parser` refuses them and recovers with `BadExpr`, `BadStmt` and `BadDecl`; the grammar recovers as its own parser does, so the trees differ. Each is held to a syntax error |
| `suite/invalid/typesw.go` | `go/parser` accepts it (the type checker refuses it), and the language reports what its parser reports |

`suite/NOTICE` repeats the files of the suite with the same reasons. A file `go/parser` refuses and the language reads to the same tree stays in `suite/invalid/` (`composite.go`, `import.go`) and `relations/syntax_error/`.
