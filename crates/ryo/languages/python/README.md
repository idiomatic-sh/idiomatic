# idiomatic-sh-ryo-python

The Python language of ryo: it reads `.py`, `.pyi` and `.pyw` files with `ruff_python_parser` 0.0.18, the parser ruff and ty run, for the latest Python it reads, into syntax trees whose tokens spell the file, and their definitions, imports, names, calls, strings and comments into ryo's code relations. A name it cannot resolve statically is an unknown value with its reason. It is held to the parser's own test resources, pinned in `tests/fixtures/suite`.

## Usage

Link the language with `ryo_python::linked()` and hand the result to the SDK's calls.

```rust
let language = ryo_python::linked();
assert_eq!(&*language.name().0, "python");
```

## Domains

### definitions

The python domain's `definitions` concept: what an element defines, as the `defines` and `parameter` relations say it: a function, a class, an imported or assigned name, a parameter and its place, an exception's name, a pattern's capture or a type parameter, with Python's own word for each.

### extras

The python domain's `extras` concept: the kinds of its trees that `ruff_python_ast` has no name for.

### `fact_notes`

The python domain's `fact_notes` concept: what the parse leaves of a file for the project to read, the imports it writes.

### language

The python domain's `language` concept: which files are Python.

### links

The python domain's `links` concept: which file of the repository each import of a file names, from the imports the parse noted of each file and the packages the manifests declared.

### manifests

The python domain's `manifests` concept: one reader for each relation the language reads from the manifests of a project and the links between its files.

### model

The python domain's `model` concept: the `global` and `nonlocal` names of every scope of a file, worked out once for its readers.

### names

The python domain's `names` concept: dotted paths, the bindings and targets of imports, the values of strings, and the reasons a name cannot be resolved statically.

### nodes

The python domain's `nodes` concept: the nodes of its trees, named as `ruff_python_ast` names its node kinds in `ast.toml`.

### packages

The python domain's `packages` concept: the packages the manifests of a repository declare, and how they hold and need one another.

### relations

The python domain's `relations` concept: one reader for each relation the language reads.

### scopes

The python domain's `scopes` concept: the scope each element is evaluated in, and where a name is bound once `global` and `nonlocal` have been read.

### syntax

The python domain's `syntax` concept: its kinds, and how a reader moves about a file's tree.

### tokens

The python domain's `tokens` concept: the tokens of its trees, named as `ruff_python_ast` names its token kinds (`TokenKind`).

## Differences from the standard

The standard is `CPython` 3.15.0rc2's `ast` and `token` (`tests/fixtures/standard/README.md`). Our trees are `ruff_python_parser`'s, kept whole with their tokens; projected onto the standard they have its nodes, in every pinned file, but for these differences.

- One kind where the standard has two: `async def`, `async for`, `async with` and `except*` are `FunctionDef`, `For`, `With` and `Try`, the `async` or `*` token saying which; the standard has `AsyncFunctionDef`, `AsyncFor`, `AsyncWith` and `TryStar`.
- One `Constant` for strings, bytes, numbers, booleans, `None` and `...`: the library has six kinds and the tree keeps the one `Constant` with its token; the standard's `Constant.kind` and value are the token's text.
- Operators, expression contexts and type comments are not nodes: `Add`, `And`, `Load`, `Store`, `Del`, `TypeIgnore` and the rest are the tokens of the operator or nothing; the standard prints them as nodes.
- Extras, kinds the standard has no node for: identifiers, decorators, `elif` and `else` clauses, call arguments, a parameter with its default, type parameter brackets, class pattern arguments and keywords, the parts of an implicit string concatenation, and `IPython` escape commands. Each holds tokens or a place the standard keeps in a field; the projection drops them.
- An `elif` is a flat clause of its `if`; the standard nests it as an `If` in `orelse`. The projection nests it.
- A lambda with no parameters has no `arguments` node; the standard has an empty one. The projection adds it.
- The text of a self-documenting interpolation, `{x=}`, is tokens of the interpolation; the standard prints a `Constant` before it. The projection adds it.
- Tokens: a keyword is a `NAME` as Python's tokenizer gives it; the white space the tokenizer gives no token for is a `whitespace` extra, so the tokens spell the file; `?`, an unknown character and an `IPython` escape are `ERRORTOKEN`.
- Stated exception, `suite/valid/inline/unary_plus_py315.py`: `CPython` folds the key `+1` of a mapping pattern into a `Constant`; the library keeps the `UnaryOp` written, so the tree has one node more than the standard's there. It is the one pinned file whose projected tree is not the standard's.
- Accepted where `CPython` refuses: a `try` with `else` and no `except`, and an unknown `__future__` feature. The library reads both without an error; the language reports `CPython`'s message on the same line (`tests/fixtures/standard/refused/`).
