# idiomatic-sh-ryo-go

The Go language of ryo: it reads `.go` files as Go 1.26 with `tree-sitter-go`, the grammar GitHub's code navigation, Zed, Helix, difftastic and ast-grep read Go with, into syntax trees in the shape `go/parser` gives them (its nodes the types of `go/ast`, its tokens the constants of `go/token`, the white space between the tokens an extra), and their declarations, names, calls, strings and comments into ryo's code relations; it reads `go.mod` and `go.work` into the manifest relations. A name it cannot resolve statically is an unknown value with its reason. It is held to the tree `go/parser` makes of every file of the Go project's own compiler test suite (`tests/fixtures/suite`), of a corpus of public Go projects (`tests/fixtures/corpus`) and of each relation's input, and to the facts `go/ast`, `go/scanner` and `go/build` read in them, all pinned in `tests/fixtures/standard/output.txt`.

## Usage

Link the language with `ryo_go::linked()` and hand the result to the SDK's calls.

```rust
let language = ryo_go::linked();
assert_eq!(&*language.name().0, "go");
```

## The tree

A kind is a node, a token or an extra, named in kebab case: `func-decl`, `lparen`, `whitespace`.

- The nodes are the types of `go/ast`: `FuncDecl` is `func-decl`, `BasicLit` is `basic-lit`. The comment nodes (`Comment`, `CommentGroup`), the nodes `go/parser` makes of text it cannot read (`BadExpr`, `BadStmt`, `BadDecl`) and `Package` are kinds the language never writes: comments are tokens, and a broken file is read as the tokens and the nodes the grammar can make of it, with a `syntax_error` for each place it could not.
- The tokens are the constants of `go/token`: `Add` is `add`, `AndNot` is `and-not`. A token that shares its name with a node is `ident-token`, `comment-token`, `ellipsis-token` or `package-token`. The semicolons the scanner inserts at a line end are not tokens: the line end is white space.
- The extras are `whitespace`, the text between two tokens, and `error`, the bytes of a file after the first that is not UTF-8. A byte order mark is white space. The tokens of a tree spell the file byte for byte.
- A tree is the nodes nested by the bytes they span, as `go/ast` places their positions: a `FuncDecl` holds a `FuncType` that starts at `func`, so a function's receiver, name and parameters are under its `FuncType` and its body is beside it; a `select` default is a `CommClause`; a label with no statement has the empty statement `go/parser` makes. A node that `go/ast` builds from a token alone (an `Ident`, a `BasicLit`) holds that token.

`tree-sitter-go` gives a different tree: a concrete one, with a node for every bracket and comma and groups that `go/ast` does not make. The adapter reads it into plain data in the words of Go's syntax (`domain/grammar`), and the domain finds in it the nodes and tokens `go/ast` and `go/token` make (`domain/shapes` and the concepts it calls); the conformance rails and the adapter's `Pinned` case hold every file of the fixtures to the tree the standard library makes.

## Relations

| relation | what the language reads |
|---|---|
| `function` | every `func` declaration, methods included; a function literal is none |
| `defines` | functions, methods, types, constants, variables, imports, fields, interface methods, parameters, results, receivers, type parameters, and the names `:=` and `range` declare; `_` declares nothing |
| `parameter` | each receiver, parameter and result, and each parameter of a method of an interface, in its place in its list; a variadic parameter takes the arguments left; a parameter the declaration does not name has the unknown value `unnamed` |
| `annotates` | the type of each named parameter, result, receiver and field, and of each name a `var` or `const` gives a type, as written |
| `assigns` | a string a name is bound to by `=`, `:=`, `var` or `const`, name and value paired by their places |
| `inherits` | each type embedded in a struct and each interface embedded in an interface |
| `reference` | a name used and a dotted name used, whole: `a.b.c` is one reference; the key of a composite literal is a field or a variable, so its target is the unknown value `ambiguous-key` |
| `static_call` | every call by the dotted name it calls; a callee that is no dotted name has the unknown value `dynamic-callee`; a conversion to a type literal (`[]byte(s)`) is none, and one to a named type (`int64(x)`) is a call of that name, which no tree tells from a call without the types |
| `static_import` | each package an `import` brings in; an import with no name binds the unknown value `declared-by-package`, and `import .` binds `dot-import` |
| `string_literal` | every string, interpreted or raw, decoded; a string whose escapes make a byte that is no UTF-8 has the unknown value `not-text` |
| `comment` | every comment with its text without its delimiters |
| `directive` | every build constraint of a file's header, a `//go:build` or `// +build` line as `go/build` reads one: its kind (`go:build` or `+build`) and its expression, the text after the word; each line is also a `comment` row |
| `mapping` | in a `go.mod`, the file, which holds the `require` lines of its own, and each `require` block; a Go source writes none |
| `entry` | in a `go.mod`, each module it requires, keyed by its path, with its version as its value; a Go source writes none |
| `scope` | the file, and each function declaration and function literal under its enclosing scope |
| `syntax_tree`, `syntax_child`, `syntax_error` | the tree, its edges, and each place the grammar could not read or read where Go does not allow it |
| `package_dependency`, `workspace_member`, `file_package` | from `go.mod` and `go.work` (`outbound/manifest_reading/adapters/gomod`, the line grammar `cmd/go` reads them with): each requirement of a module, on another module of the repository found by its path or by the directory a `replace` sends it to (origin `path`), or on a module outside it, with no target (origin `registry`: `go.mod` names a module by its path, which a proxy or its repository serves, and writes no git source); each module a `go.work` uses; the module each file belongs to |
| `source_dependency` | each import whose path is the path of the file's module, the one whose `go.mod` is the nearest above it, or starts with it: one row to each Go file but the tests of the directory under the module's that the rest of the path names. The standard library, another module and a module nested in the file's are no row (`package_dependency` answers a module of the repository); a path under the module that names no directory of Go files, and every file of a module whose `go.mod` was refused, read the relation in part at the file |
| `analyzed_source` | every Go file the parse noted, but one of a module whose `go.mod` was refused |

`decorates`, `mention`, `visibility`, `sequence_element` and `prose` are not in Go (the Go specification has no decorator, no construct a name is mentioned in outside code, no syntax that writes a visibility, an identifier being exported by its first letter, and no document of prose; a `go.mod` writes no list of values). The blocks of a function are no scopes: the function owns what is in it. A manifest the tool refuses (a `go.mod` with no `module`, a block not closed) declares nothing, and the manifest relations are read in part for it.

What the language does not know without types, it says: the name an import without a name binds is the package's own, which the file does not say; the key of a composite literal is a field or a variable; a call of an index (`fns[0](x)`, `f[int](x)`) is a dynamic callee.

## Differences from the standard

- `tree-sitter-go` 0.25.0 reads the Go of 1.25, and a grammar is not a compiler's parser. The language moves what it can to `go/ast`'s tree (a conversion to a pointer type is the `*` of a conversion, a long group of parameters that share a type is one field, a label with no statement has its empty statement); it does not read to the standard `new(expr)`, which Go 1.26 allows, a method expression on an interface literal (`interface{ m() }.m`), and `[...]T(x)` as a conversion. `tests/fixtures/suite/NOTICE` lists the files of Go's own test suite left out for these reasons.
- A file `go/parser` refuses is recovered by the grammar as the grammar recovers, which is not as `go/parser` does (`BadExpr`, `BadStmt`): the tree of such a file is not the standard's, so those files are held to their syntax error alone, in `tests/fixtures/suite/refused`; `suite/invalid` holds the ones whose trees agree.
- `Comment`, `CommentGroup`, `BadExpr`, `BadStmt`, `BadDecl` and `Package` of `go/ast`, and `EOF` of `go/token`, are kinds no tree of the language holds.
- The semicolons the scanner inserts at a line end are white space here, as `go/scanner` hands them over with the text `"\n"`.
- Facts the standard reads and the language reads otherwise are stated per relation above: a parameter the declaration does not name is a row with the unknown value `unnamed`, an import with no name is a row with the unknown value `declared-by-package`, a build constraint counts only in a file's header as `go/build` reads it.

## Domains

### assignments

The go domain's `assignments` concept: the two sides of an assignment, a short variable declaration and a `range` clause.

### constraints

The go domain's `constraints` concept: the build constraint lines of a file, `//go:build` and `// +build`, as `go/build` finds them in the file's header.

### declarations

The go domain's `declarations` concept: the nodes of `go/ast` a function, a method, a function type, a parameter list and a declaration are made of, which the library's tree does not group the way `go/ast` does.

### definitions

The go domain's `definitions` concept: what an element declares, as the `defines` and `annotates` relations say it: its shared kind, Go's word for it, its name and the scope that binds it.

### expressions

The go domain's `expressions` concept: the nodes of `go/ast` a union, an instantiation, a call, a conversion, a unary expression, an interface, a field and a composite literal are made of, which the library's tree does not group the way `go/ast` does.

### extras

The go domain's `extras` concept: the kinds of its trees that its standard has no name for.

### `fact_notes`

Notes the import paths each file writes when it is parsed, for the project to read.

### fields

The go domain's `fields` concept: the parts of a field: the names it declares, its type and its tag.

### grammar

The go domain's `grammar` concept: the parser library's tree of a file as plain data, in the words of Go's own syntax, and how to move about it. The library's names stop at the adapter; what it hands over is what each of its nodes is in Go.

### language

The go domain's `language` concept.

### links

Links each import to the Go files of the package of the file's module it names, from the module whose `go.mod` is the nearest above the file.

### literals

The go domain's `literals` concept: the text a literal of a file stands for, which the library does not decode.

### manifests

The go domain's `manifests` concept: one reader for each relation the language reads from the manifests of a project.

### modules

The go domain's `modules` concept: the modules the manifests of a repository declare, which require which, which a workspace holds, and the module each file belongs to.

### names

The go domain's `names` concept: the names a row holds, made from the tokens of the file; what cannot be resolved statically is the unknown value of its reason.

### nodes

The go domain's `nodes` concept: the nodes of its trees, named as its standard names them.

### pieces

The go domain's `pieces` concept: the nodes of `go/ast` and the tokens of `go/token` found in the library's tree, before they are written into the table in the order of the file.

### relations

The go domain's `relations` concept: one reader for each relation the language reads.

### scopes

The go domain's `scopes` concept: the function that holds an element, which a row names as its owner.

### shapes

The go domain's `shapes` concept: what the nodes and the tokens of `go/ast` and `go/token` are in the library's tree of a file. The library's tree is a concrete one, with a node for every keyword, bracket and comma and groups `go/ast` does not make; each node of it is a node of `go/ast` over the same bytes, or none, or several, and each leaf is a token of `go/token`.

### signatures

The go domain's `signatures` concept: the parameters, the receiver and the named results of a function, by where they are in its declaration.

### slots

The go domain's `slots` concept: what an identifier is where it stands: a name it declares, a use of one, or neither, and so whether it is a reference.

### specs

The go domain's `specs` concept: the parts of what a declaration lists: the names and the type of a constant or variable, the name and the type of a type, and the name and the path of an import.

### statements

The go domain's `statements` concept: the nodes of `go/ast` a switch, a select, a loop, a receive, a label and a branch are made of, which the library's tree does not group the way `go/ast` does.

### syntax

The go domain's `syntax` concept.

### tokens

The go domain's `tokens` concept: the tokens of its trees, named as its standard names them.
