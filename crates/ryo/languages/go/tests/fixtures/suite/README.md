# go test suite

## Source

The Go project's own compiler test suite, `test/` of go.dev's source (BSD 3-Clause, `LICENSE` beside this file), copied unchanged. A file whose first lines are `// run` or `// compile` is a program Go accepts, and is in `valid/`: those of `test/` itself, and those of `test/typeparam/` (as `valid/typeparam/`), the programs that use generics. The files of `test/syntax/` are the programs Go refuses for their syntax, and are in `invalid/`. `NOTICE` names the files of `test/` that are left out, with the reason; the files of `test/syntax/` whose tree `go/parser` makes of the error the language does not reproduce are in `refused/`, held to a syntax error.

## Version

Go 1.26.5, the release whose `go/ast` and `go/token` the standard here pins, and whose `go/parser` accepts every file of `valid/` and refuses every file of `invalid/` (`standard/output.txt` states it).
