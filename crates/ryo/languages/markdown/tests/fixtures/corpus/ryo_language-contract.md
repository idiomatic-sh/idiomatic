# The language contract

Ryo owns the concepts: the relations of the catalog. A language fulfils them. A language implements two
traits of `ryo::languages`, `Parse` and `Project`, and that is all of ryo it implements: every relation of the
catalog is one required method of one of them, with no default body, so a relation added to the catalog is a
compile error in every language until the language has answered it. The crate does not compile with a relation
left out, and no engine code lists a language or a language's file names.

## What a language answers

| trait | what it says | per relation |
|---|---|---|
| `Parse` | `recognizes(request)`: whether it reads the file; `skips()`: the names of the directories it never reads, at any depth (`node_modules`, `target`; empty when its ecosystem has none), declared as `SKIPS` in `domain/language/constants.rs` and not entered by the walks of ryo and Tastemaker; `kind_count()`: how many kinds its trees have (at most `KIND_LIMIT`, 1024); `read_tree(request, builder)`: reads the file with its parser library into the builder the engine gives, and nothing else; `complete(request, table, symbols)`: names the kinds and writes the notes. The engine composes `build` from them, and the allocation row's bound is `read_tree` through a builder that builds nothing, so no body can be written beside the library to be compared with | one method for each syntax relation (`function`, `comment`, `syntax_tree`, `mapping`, ...): `Support<Reader<R>>` |
| `Project` | `manifests()`: the names of the files it reads as manifests; `resolve(request)`: reads them through its own tools and answers the bytes the readers read and the run keeps | one method for each manifest relation (`package_dependency`, `workspace_member`, `source_dependency`, `analyzed_source`): `Support<ManifestReader<R>>` |

The relations the engine gives for every file (`filesystem.file`, `filesystem.line_count`) are in neither trait: a
language cannot write them.

Each answer is `Support::reads::<Item>()` (`Support::reads_manifests::<Item>()` for a manifest relation) or `Support::absent(reason)`: a `Reads` is made only from an item that implements `ReadItem` or `ManifestItem`, never from a reader written beside it. The reason is one of
`NoSuchConstruct`, `NotCode`, `NotAKeyedDocument`, `NotADocument`, `NotYetRead`, `NoManifests` (see "Why a language does not answer a relation"); `ryo relation describe <relation>` lists, for
every linked language, `reads` or the reason.

A reader gets the rows of its own relation to write and no other: `Reader<R>` is given `&mut dyn Rows<R's row>`,
`ManifestReader<R>` the same. The engine reads the tree once for the readers of every wanted relation, so a
language never receives the writer of the whole store.

## Scopes

Every scope has a row of `scope`, including the top-level ones: a function, a class or any other item that makes a scope at the top of a file has the file's module scope for its parent. The module scope itself has a row too, with its own node for its parent (the root of a tree is its own parent in the element table, and here too), so the shape has no parentless row: a scope with no parent is not a shape. It is the only row of its file with that. The conformance holds it: of the scope rows of each sample file, exactly one has the same node for parent and child, and every other has a parent that is a child of a row of the file. A file with nothing in it still gives that one row. Rust's scopes are the module of the file, a nested module, an `impl`, a trait and a function.

## Documents

A language of documents (Markdown, HTML) says what a document declares with the shared kinds of `defines`: a titled section is `defines { kind: section }`, nested through `scope` as functions nest in classes, and an anchor, an id or a label is `defines { kind: label }`. Code languages never write these two kinds. Markdown reads them: a heading is a section (named by its title, held by the section around it, scoped from the heading to the next heading as shallow or shallower), and the definition of a link reference, of a footnote and an `id` or `a name` of raw HTML are labels.

A link target is `reference.target`, and the language resolves it to a run-relative path as it resolves an import (`static_import.target`): `[text](../guide.md#setup)` in `docs/a.md` has the target `guide.md#setup`, where `docs/` is the document's own directory; a fragment alone is the document's own path with it (`docs/a.md#top`), a path from the root (`/x.md`) is the run's own `x.md`, and a destination with a scheme stays as written. The language does it where it builds the tree: `read_tree` is given the request with the file's path, and writes the resolved target as the decoded text of the first marker of the link, which `named_by` reads back (Markdown's adapter shows how). RyoQL has nothing for it: a rule compares the target with a path or a `defines.name` as it compares any two names.

The text a reader reads is `prose { id, owner, text, file, span }`: one row for each run of a document: a paragraph, the title of a heading, a cell of a table, the text an item of a list holds itself (an item of paragraphs gives the rows of its paragraphs). The `text` joins the markup of the run and masks what a reader does not read as prose: inline code, math and raw HTML become a space, a break is a space, and the white space between words is one space. The `owner` is the section that holds the run, or the document. A language that is not a document answers `NotADocument`.

## Directives

An instruction to the build tool that a file carries is `directive { file, span, kind, text }`: `kind` is the directive's name as the language writes it and `text` its argument text as the tool reads it. Go's `//go:build linux && amd64` is `kind: go:build, text: linux && amd64` and a header `// +build linux` is `kind: +build, text: linux`; the line is also a `comment` row, and it is no `prose` row, which is the text a reader of a document reads. The kinds are the language's words and ryo keeps no list of them. A language with no such instruction answers `NoSuchConstruct`.

## Why a language does not answer a relation

| reason | said by | when |
|---|---|---|
| `NoSuchConstruct` | any language | the language has no such construct (Markdown has no `function`) |
| `NotCode` | a data format | the language is data: it reads no relation of code (TOML says it of `function`). A language that reads any relation of code never answers it |
| `NotAKeyedDocument` | a language of code | it has no keyed documents |
| `NotADocument` | a language that is not a document | it has no prose; one that reads `prose` never answers it |
| `NoManifests` | a language with no manifest | it names no manifest file |
| `NotYetRead` | a language being written | the construct is in the language and its reader is not written yet. It is a stage, never an answer for a finished language: a finished language answers `NoSuchConstruct` (the construct is not there) or reads it |

The conformance holds each reason against the class of the relation and against the other answers of the language.

## Manifests are files

A column that names a manifest holds the kind File, never text: `workspace_member.workspace` and `package`, `file_package.package`, `package_dependency.source` and `target`. A File is the number of the file among the files the run lists, in the order of their paths, so a rule joins a manifest with `filesystem.file.path` as it joins any file. A manifest the run does not list (a workspace names a member the scope leaves out) is still a file the rows name: its number is the one after the last file the run lists, in the order of the paths of such manifests, and the language says which they are in `Resolved::unlisted`. Two languages that both name manifests the run does not list are refused: the numbers after the listed files are one language's to give.

### What each language's manifests say

A manifest is a file the language names (`manifests()`, the last part of the path) and reads with the parser of its format. Every language answers the same relations from it: a *package* is a manifest that names one; a *workspace member* is a package the workspace manifest holds; a *dependency* is a requirement of a package on another package **of the repository**, written with the kind below. The rows name manifests as Files and never text, so a requirement on a package outside the repository (a package of PyPI, a module of the Go proxy, a package of npm) is not a row of `package_dependency`: that relation is the graph of the repository's own packages. The `target` of a requirement is the manifest of the package of the repository the requirement names, found by the name the format normalizes; a requirement none of the repository's packages answers to has no row. `package_dependency.package` is the name as the format normalizes it and `binding` the name as the manifest writes it; `condition` is the condition text as written (a marker, a build tag); `optional` is `Optional` for what the package needs only when asked for.

| | Python | Go | JavaScript and TypeScript | Rust (as built) |
|---|---|---|---|---|
| manifests | `pyproject.toml` | `go.mod`, `go.work` | `package.json`, `tsconfig.json` | `Cargo.toml` |
| parser (format) | TOML, requirements by PEP 508 | the `go.mod` line grammar | JSON, `tsconfig.json` as JSON with comments | `cargo metadata` |
| a package | `[project]` with a `name`; the name is normalized as PEP 503 does (lower case, each run of `-`, `_` and `.` one `-`) | the `module` of a `go.mod`; the name is the module path | a `package.json` with a `name`; the name is as written | a `[package]` |
| a workspace | `[tool.uv.workspace]` `members` (globs, from the directory of the file) less `exclude` | `use` directories of a `go.work` | `workspaces`: a list of globs, or `{ packages: [..] }` | `[workspace]` `members` less `exclude` |
| a member | the package in a directory the globs hold | the `go.mod` in a `use` directory | the `package.json` in a directory the globs hold | the package in a directory the globs hold |
| a dependency, its kind | `project.dependencies` is `normal`; `project.optional-dependencies.<extra>` is `optional`; `dependency-groups.<group>` (PEP 735) is `dev`; `build-system.requires` is `build` | `require` is `normal` (`// indirect` too); `replace` to a directory points the requirement at it | `dependencies` is `normal`; `devDependencies` is `dev`; `peerDependencies` is `peer`; `optionalDependencies` is `optional`; a `references` path of a `tsconfig.json` is `normal` between the packages of the two directories | `[dependencies]` is `normal`, `[dev-dependencies]` `dev`, `[build-dependencies]` `build` |
| how a requirement's target is written | a PEP 508 string, `name[extras] (>=1) ; marker`: the name is normalized and the marker is the `condition` | a module path; a `replace` path names the directory | a package name; `workspace:*` and a `file:` path name a member | a crate name or a `path` |
| what is not a row | a requirement on a package outside the repository | a module outside it | a package outside it; the `paths` of a `tsconfig.json`, which help `source_dependency` and are no dependency | a crate outside it |

Python is the first worked case: `pyproject.toml` is read with a TOML parser, each requirement with a PEP 508 parser, and the three relations `package_dependency`, `workspace_member` and `file_package` are read from the packages found (the package a file belongs to is the one whose directory is the nearest above it). The contract's types carry no manifest format: a manifest is a path and a text, what the language works out of the manifests is its own `Derived`, and the rows say Files, names and kinds from the shared words.

## A language's shape

The tree is one template followed over and over, so that it is simple to guess and to hop from one language to another. Every language has this tree; `*` marks a part present exactly when its trigger holds.

```text
languages/<lang>/
  Cargo.toml  tastemaker.toml  README.md  benches/performance.rs   (the bench is always there, placed at count 1: the corpus is the workload)
  src/
    lib.rs (link!)  cases.rs
    inbound/contracts/{parse,project}/contract.rs
    domain/
      syntax/{types,service,service_test}.rs        Kind = Node(Node) | Token(Token) | Extra(Extra)*
      nodes/{types,constants,service,service_test}.rs   one flat enum of the node standard's names; STANDARD names the standard and its version;
                                                    the standard's categories, if it has any, are functions of service.rs
      tokens/{types,constants,service,service_test}.rs  one flat enum of the token standard's names; STANDARD likewise
      extras/{types,constants,service,service_test}.rs  *  present exactly when `Kind` has an `Extra` arm, which is when the enum is non-empty;
                                                    each variant's doc says why the standard lacks it
      language/...                                  recognition (always)
      model/...                                     *  present exactly when `Parse::FileModel` is not `()`
      relations/collections/relations/{fixtures/contract.rs, <relation>/{item,item_test}.rs}
      manifests/collections/manifests/<relation>/...  *  present exactly when the language reads manifests
      <other concepts>/...                          the language's own logic, free-form
    outbound/
      syntax/{interface,factory,fixtures}.rs  syntax/fixtures/{contract,fake,fake_test,library}.rs
      syntax/adapters/<library>/{adapter,adapter_test,schemas,schemas_test,types,constants}.rs  + fixtures/{walk,oracle}.rs
      project/...                                   *  with manifests
      <other ports>/...                             free-form, where the language's logic needs IO (Rust's scheduler)
    fixtures/{kinds,relations,contracts,reading}.rs     kinds.rs keeps `sample(kind)`: fixture data, not domain
  tests/conformance.rs  tests/allocation.rs
  tests/fixtures/{standard,suite,corpus}/{README.md, NOTICE, ..}
  tests/fixtures/relations/<relation>/{input.<ext>, expected.rs}  tests/fixtures/allocation/  tests/fixtures/manifests/<case>/..
```

**The kinds are the standard's.** A language's node kinds and token kinds follow that language's standard, library-independent, where one exists, and the parser library is mapped into them exhaustively in the adapter's `schemas.rs`, so a library can be swapped. The standard is pinned under `tests/fixtures/standard/` with its version, and `STANDARD` in `nodes` and `tokens` names it.

| language | node standard | token standard |
|---|---|---|
| Python | CPython `ast` | CPython `token` module |
| JavaScript | ESTree | ECMA-262 §12 |
| TypeScript | `@typescript-eslint/ast-spec` `AST_NODE_TYPES` | `AST_TOKEN_TYPES` |
| Markdown | mdast | micromark token types |
| HTML | the DOM's nodes | the HTML §13.2.5 tokenizer |
| CSS | CSS Syntax Level 3 §5 | CSS Syntax Level 3 §4 |
| Go | `go/ast` | `go/token` |
| TOML | `toml.abnf` (both) | `toml.abnf` (both) |
| JSON | RFC 8259 `json.abnf` (both) | RFC 8259 `json.abnf` (both) |
| Rust | `ra_ap_parser` `SyntaxKind`, pinned and re-pinned with the library | the same |

**Kind spelling.** A kind word is the standard's name in kebab-case, derived mechanically: `FunctionDef` is `function-def`, `TSTypeAnnotation` is `ts-type-annotation`, rust-analyzer's `FN` is `fn`, mdast's `heading` is `heading` (its depth is data, not six kinds). When a token's name collides with a node's (Python's `NAME` and `Name`), the token takes the suffix `-token`: `name-token`. The words are consistent RyoQL, guessable from the standard's page, and the existing rules of Rust do not change.

**The shape rail.** A language's walk or oracle compares the library's tree, mapped into the standard's vocabulary, with something independent of the code that built it: either an independent walk of the library's own tree (`fixtures/walk.rs`), or a projection of the standard itself (Python's comparison with `ast.dump`). The conformance accepts either, so no language keeps a walk that compares the adapter with itself; a walk that stays only places the tokens.

**Not yet true.** "One command writes the whole tree": the scaffold writes a partial tree today. It is to be fixed, and judged for what it spares the author, as each language is built.

## Where each part goes (`languages/<lang>/`)

| part | where | placed by |
|---|---|---|
| `impl Parse`, with every syntax answer | `src/inbound/contracts/parse/contract.rs` | the `language` spec's `parse-contract` part (`languages::Parse`) |
| `impl Project`, with every manifest answer | `src/inbound/contracts/project/contract.rs` | the same (`languages::Project`) |
| the item of a relation it reads: the reader and the rows it writes | `src/domain/{relations,manifests,links}/collections/<collection>/<name>/item.rs`, and nowhere else (the rule `readers-in-items`); the item is declared with `ryo::reads!`, the only way an item gets `ryo::languages::ReadItem` (syntax relations) or `ryo::languages::ManifestItem` (manifests and links): both traits are sealed, so a reader written by hand does not compile (the ui case `fail_read_item_written_by_hand`) | the compiler: `Support::reads::<Item>()` makes the answer from the item, whose `READER` is a `Reader` of the relation the trait names; the conformance holds the rows |
| its own tastemaker.toml | `tastemaker.toml` | declares the parser libraries it reads with |
| the syntax adapter (the parser library, behind the `syntax` port) | `src/outbound/syntax/adapters/<library>/adapter.rs` | the `port` and `adapter` specs; the parser's crate is imported there and nowhere else |
| the project adapter (the tool that reads manifests) | `src/outbound/project/adapters/<tool>/adapter.rs` | the same; the only place a language touches the file system or runs a process |
| the fixture of a relation it reads | the input in `tests/fixtures/relations/<relation>/input.<ext>`, and the typed rows it must give in `tests/fixtures/relations/<relation>/expected.rs`, which `src/fixtures/relations.rs` includes (`fixtures::expected()`; no table file: `no-tsv`) | the conformance, below |
| the fixture of its manifests | `tests/fixtures/manifests/<case>/` (a repository) and the typed rows of its manifest relations in `src/fixtures/<case>.rs` | the conformance, below |
| the allocation test | `tests/allocation.rs` | the template `language-allocation-test`, closed (the engine's test, written once; its samples are the fixtures and `tests/fixtures/allocation/`, its bound `read_tree` through a builder that builds nothing) |
| the conformance test | `tests/conformance.rs` | the template `language-conformance-test`, closed: a language holds its answers and its fixtures, and every test is the engine's |
| the line that links it to a program | `src/lib.rs`, `pub fn linked()`, made with `ryo::link!("<directory>", parse: .., project: ..)`, which names the language by the directory name it is given and builds it by its crate's version; the conformance asserts the name is the directory's | `cli/src/languages.rs`, written by `cli/build.rs` from `languages/` |

## The spec

`tastemaker/specs/language` (the package `crates/ryo/tastemaker`) places a language's two tests and, with the rules of the product's `tastemaker.toml` over `languages/*` (a rule of a spec cannot see the files that `nu-hex` nests inside a language: its use cases, ports and concepts), holds it, beside its `crate` and `nu-hex` entries, by one entry over
`languages/{lang}`. Each concept a language fulfils is held first by the compiler, then by the conformance, then by Tastemaker,
and each rule branch has a failing fixture in its rule:

| concept | compiler | conformance | Tastemaker (spec fixture) |
|---|---|---|---|
| it answers every relation | `Parse`, `Project`: one required method for each, which a scaffold leaves out so that the compiler lists them | `Reads` has its fixture and its full rows, `NotInThisLanguage` has no fixture with the construct and a required `Reason` | `language-parts` (`no-parse`, `missing-project-use-case`) |
| its kinds are one vocabulary | the adapter maps every library name onto the variant of the same name; `sample(kind)` is a match with no wildcard arm, so a kind the grammar gains is a compile error until it is shown | `kind_count` is above 0 and fits a walk; every kind with a sample is read by some relation's `kinds:` or marked `Sample::Unread(reason)` (the unread check), and a sample's tree holds an element of its kind | `language-parts` (`missing-kinds`, `kind-directory`) |
| it reads with a parser library | | | `language-parts` (`missing-syntax-adapter`), `client-at-adapter` |
| it reads a project | | the full typed rows of each `manifests/<case>/` | `language-parts` (`missing-project-adapter`, `missing-manifests-fixtures`) |
| it is held to fixtures | | `tests/fixtures/{relations,manifests,allocation}` | `language-parts` (`missing-relations-fixtures`, `missing-allocation-fixtures`) |
| a row is numbers | | the allocation test | `no-string-fields` |
| it touches no file or process but in its project adapter | | | `io-at-boundary` |
| it is linked | | | `language-parts` (`lib-without-linked`) |
| it proves itself | `autotests = false` and a `[[test]]` for `conformance` and `allocation` in its manifest | `tests/conformance.rs`, `tests/allocation.rs` | the templates (`missing-conformance-test`, `missing-allocation-test`) |

A reader is a declaration. `ryo::reads!` takes the relation, the names its code uses (`|tree, at, symbols|`), the kinds it looks
at (`kinds: [..] as kind_bits`, or `every`) and statements: `require`, `given`, `row { column: form }` with the forms `node`,
`file`, `span`, `text` and `value`, `partial`, `when <function>(..)` for one branch per shape a model function answers, and
`each (<list>) as (<ordinal>, <item>)`. The row is the relation's own, so a column left out or one it does not have fails to
compile at the row. A reader of trees that would search the file again for each element it is given (the header an entry belongs to, the entries of a table) works the answer out once for each file with `model <name>: <Type> = (<expression>);` as its first statement after `kinds:`; the expression sees the tree, and the statements see `<name>` as a reference to the value. A manifest or link reader names the model it reads (`model <name>: <Type> = (<function>(input..))`) and writes its
rows from the model alone. The functions a statement calls are the language's model, in `domain/<concept>/service.rs`: they answer
values and never write a row. A rule of Rust that needs a loop is a model function that answers an iterator.

## What a query may name

A word compared with a column that holds a closed set of words is one of them: `kind = "headline"` on `defines` is refused when the query is compiled, with the words there are (`` `headline` is not a kind of defines; the kinds are function, method, .. ``), and the rule does not run. The columns are `defines.kind`, `parameter.kind`, `entry.kind`, `sequence-element.kind`, `mapping.kind` and `package-dependency.kind`.

A word a rule compares the kind of a node or a token with (`syntax-tree.kind`, with `=`, `in` or a glob) names a kind of some linked language, or the run fails before a file is read, naming the word and the languages (`` kind `function-declaration` names no kind of rust, toml, markdown, python ``). A language declares the names of its kinds with `kind_names()`, each once, as many as `kind_count()`; the conformance holds both. The refinement and the field of a node are the language's own words and have no list yet, so a word compared with them is not checked.

## What a reader may rely on

A reader is given the tree: the kinds and the positions of its elements, their text, and the roles of the children under their parents (`languages::field_at`, `field_name_at`) when the run asks for them, which it does only when a rule reads `syntax-tree.field`. A language whose own relations need the role of an element works it out from the tree as Rust does (`fields::field_of`), or writes it with `write_fields` in `complete`; it does not count on the run asking.

A language held to an official suite whose files cannot all be read (written for another version of the language, or valid only to a semantic checker) leaves those files out by name in the suite's `NOTICE` with the reason for each; it never changes a file, and never counts a file it leaves out as read.

## The keyed relations

A data document (TOML, JSON, YAML) is read as three relations, each a different row of the same tree. These are the decisions a language
fulfils; each has an example, written for TOML.

| decision | rule |
|---|---|
| the root | The document is the file's root element (position 0). A document whose top level is a table of keys has one root `mapping` row whose `id` is the root element, whose `kind` is `standard` and whose `owner` is the root element itself. A document whose top level is a sequence has no root mapping; its `sequence_element` rows have the root element as `owner`. |
| the anchor of a mapping | `mapping.id` is the element where the mapping is first written: its header (`[a]`, `[[a]]`), the first dotted key that names it (`a.b = 1` names `a`), or the first longer header that implies it (`[a.b]` implies `a`). A mapping never written by itself is `implicit` and anchored at that first longer header's segment, so the same mapping has the same id however many later headers pass through it. |
| `entry.ordinal` | The place of the key among the keys of its owner, from zero, in the order the keys are first written, counting each key once (a table reopened by a second header continues the count). |
| `mapping.ordinal` | The place among the mappings of a sequence of mappings (`[[a]]`), from zero; zero for every other mapping. |
| `sequence_element.ordinal` | The place in its sequence, from zero. |
| `owner` | `entry.owner` is the mapping that holds the key (the root mapping for a top-level key); `mapping.owner` is the entry or sequence element whose value the mapping is (the root element for the root mapping); `sequence_element.owner` is the entry or element that holds the sequence. |
| value kinds | `string`, `integer`, `float`, `boolean`, `datetime`, `sequence`, `inline-mapping`, `mapping`, `sequence-of-mappings`, and `null` for the value that says there is none (JSON; TOML never writes it). |
| `decoded` | The text the language's library decoded for a string value (`decoded_token` in the table builder), empty for every other kind; `text` is the value as written. |

Example, `tests/fixtures/relations/entry/input.toml`:

```toml
name = "a"
[server]
port = 80
[[bins]]
path = "x"
```

| relation | rows (the ids are the starts of the elements named) |
|---|---|
| `mapping` | `root` (kind `standard`, ordinal 0, owner `root`); `server` (`standard`, 0, owner the entry `server`); the first `[[bins]]` (`sequence-member`, 0, owner the sequence element 0 of `bins`) |
| `entry` | `name` (kind `string`, ordinal 0, owner `root`, decoded `a`, text `"a"`); `server` (`mapping`, ordinal 1, owner `root`); `port` (`integer`, ordinal 0, owner the `server` mapping, text `80`); `bins` (`sequence-of-mappings`, ordinal 2, owner `root`); `path` (`string`, ordinal 0, owner the first `bins` mapping) |
| `sequence_element` | element 0 of `bins` (kind `mapping`... as a sequence member: owner the entry `bins`, ordinal 0) |

Dotted keys and implicit tables: `a.b.c = 1` gives the mappings `a` and `a.b` (kind `dotted`, anchored at the segments `a` and `b` of that key)
and the entry `c` owned by `a.b`; `[x.y]` alone gives `x` (kind `implicit`, anchored at the segment `x` of the header) and `x.y`
(`standard`, anchored at the header); a later `[x]` opens the mapping already there and keeps its anchor and its kind of first writing.

## Values a language cannot resolve

When a construct's target cannot be resolved statically (Python `from m import *`, JavaScript `require(variable)`, a dynamic `import()`, a subscripted decorator), the reader writes the row with an explicit unknown value that carries a reason from the language's closed list. It never omits the row and never marks the whole file partial.

| part | rule |
|---|---|
| the value | a name column holds either a name or an unknown value: `unknown_symbol(UnknownReason(n))` is the same word in every file, `n` being the place of the reason in the language's list |
| the list | `Parse::unknown_reasons()` is the language's closed list of `UnknownName`s, each named once; a language with no unresolvable construct declares `&[]` (Rust does) |
| the fixtures | every declared reason is shown by a fixture row whose value is `unknown("<reason>")`, and a fixture row may carry no reason the language does not declare; the conformance holds both |

Example, a Python language declaring `["wildcard-import", "dynamic-import"]`, for `from m import *` in `tests/fixtures/relations/static_import/input.py`. The module is known and is the target, resolved as any other import's; only the names it binds are not, so the binding is the unknown value, and nothing the file says is lost:

```rust
StaticImportExpectedRow {
    occurrence: node(3),
    owner: node(0),
    binding: unknown("wildcard-import"),
    target: name("m"),
    file: file("static_import/input.py"),
    span: span((1, 1), (1, 19)),
},
```

The unknown value is a word of its own among the words of the closed sets (`terms`, block `Unknown`), so it compares equal to itself in every file. The conformance shows it by the reason's name; showing the name in an engine listing from the language's list is not wired yet (a listing shows the word with no text).

## A file is read, never refused

Bytes that are not valid text are read: the valid text keeps its tree, and the rest is an error token with a diagnostic, at the offsets it has in the file (the Rust language reads each byte that is part of no character as a control character the lexer reports). `ParseError` has no case for them: a refusal is for a language that does not read the file (`Unsupported`) and for a library that fails (`Library`), and the shared rails treat a refusal of a file of the suite or the corpus as a failure.

## What a language works out of a file once

A language that would search a file again for each element a reader is given (TOML: the header an entry belongs to, the ordinal of an entry) works it out once, when it parses the file, and its readers read it. `Parse::FileModel` is the type (`()` for a language that needs none; it is `Default`) and `Parse::file_model(tree, model)` refills it: the engine makes one model for each worker, keeps it with the worker's table between files and gives it back, so a file costs no allocation the last did not pay, and the language clears what it keeps and writes the file's. The model and the language that makes it may be private to their crate: the associated type of a crate-private type is crate-private, and the engine holds the model behind an interface that never names it (`tests/ui/pass_crate_private_file_model.rs`); a reader is declared `Item => Relation, model(FileModel) |tree, model, at, symbols| { .. }`. The text a library decodes from a token (the value of a string with its escapes resolved) is written by the adapter with `decoded_token`, and a reader reads it with `decoded_at`; no language decodes text a second time.

## Per-language notes

- JavaScript and TypeScript follow oxc's source-type defaults: JSX only in `.jsx` and `.tsx`, `.mjs` is a module, `.cjs` is CommonJS, `.js` is unambiguous. JSX in a `.js` file is a `syntax_error` row, as oxc reports it.

## The conformance

`ryo::languages::contract::conformance(linked, manifest_dir, fixtures)` holds each answer to the fixtures and the typed rows
the language states (`Fixtures`: its kinds, the rows of each relation, the rows of each project), the same for every language:

- a relation answered `Reads` has `relations/<relation>/` with an input, and the rows the language writes for the input, each in
  full (a node as its place in the table, a name as its text, a span as `line:column` on each end), are the typed rows the language
  states, no more and no fewer; a mismatch prints the rows the language wrote in the syntax of `expected.rs`
  (`span((1, 1), (1, 8))`), so a wrong expectation is corrected by copying them, and `cargo test --test conformance -- --bless` writes them as the `expected.rs` of each relation, to be reviewed in the diff;
- a relation answered `NotInThisLanguage` has no `relations/<relation>/` (a fixture with the construct present
  fails the answer) and says why in its required `Reason`; a scaffold answers nothing, so the compiler lists what is
  left to answer, and no default can pass for an answer;
- `--bless` writes the `expected.rs` of each relation from what the language wrote. Where the expected rows do not compile (a column was added) or are not there, build the language without them: a language leaves them out of its build under `#[cfg(bless)]` (Rust: `fixtures.rs`), and `RUSTFLAGS='--cfg bless' cargo test -p <crate> --all-features --test conformance -- --bless` writes them from scratch;
- the rails hold every input of the language (the inputs of its relations' fixtures, `tests/fixtures/suite/valid/`, `tests/fixtures/suite/invalid/` and `tests/fixtures/corpus/`, each required to hold a file) to four checks: (1) the tokens of the tree, in order, spell the file byte for byte; (2) the tree has the shape of the library's own tree (nesting, the kind the language maps each kind of the library to, spans), by the walk of its library the language supplies (`Fixtures.walk`, giving `Walked`); (3) the rows say what the library's own semantic model says, by the oracle the language supplies (`Fixtures.oracle`, given the text and the rows of every relation it reads from trees, answering what differs; the oracle is not run on the suite's invalid files); (4) the suite's `valid` files give no syntax error and its `invalid` files at least one. The suite is the language's pinned official one (for Rust, rust-analyzer's `crates/parser/test_data/parser/{ok,err,inline}` from the pinned `ra_ap_parser`; a file written for an edition the language does not read is left out, with the reason in `corpus/NOTICE`); the corpus is real files, whose origin and licence `corpus/NOTICE` states;
- `syntax_tree` and `syntax_error` are relations like the others: the input of the `syntax_error` fixture is the
  broken file the contract reads, and a language that answers `syntax_error` `NotInThisLanguage` has none (a broken input a
  tool of the repository cannot read is left out of it by name, once: the `rules-exclude` of the product's `tastemaker.toml`
  and the `check-toml` exclude of `.pre-commit-config.yaml`, both matching `languages/*/tests/fixtures/relations/syntax_error/input.toml`, and a language's pinned suite, `languages/*/tests/fixtures/suite/`, which the file hooks leave byte for byte);
- a manifest relation answered `Reads` gives rows from `manifests/<case>/`, and the rows of every case, in full, are
  the typed rows the language states for the case;
- every kind of the language's trees is shown (`sample`): a sample whose tree holds an element of the kind, or `Sample::Unread(reason)`;
  a kind with a sample that no relation reads is a finding that says how to read it (add it to a reader's `kinds:`) or mark it; a kind a reader reads only through `kinds: every` still needs `Sample::Unread(reason)` or a sample read by a reader that names it, because `every` is not a reason to show a kind;
- a folder that names no relation read from trees is a failure, and so is a `kind_count` above `KIND_LIMIT` or 0 for a language that reads trees.

Moving a relation from `Reads` to `NotInThisLanguage` is therefore a visible change in the fixtures, not only in
a method body.

## A new language

A new language is a directory under `languages/` made with `tm scaffold`. It joins when its author starts it: add it to `members`, give `cli/Cargo.toml` its optional dependency and its feature (the build script says which is missing), and the
compiler then lists the relation methods its `Parse` and `Project` lack, one error for each, until each is answered.

There are no unwritten languages in the tree: an empty scaffold is deleted, and a language comes back with `tm scaffold` when its author starts it.

A scaffold carries no behaviour test of its own: the behaviour of a language is the conformance, which the engine owns and the closed `tests/conformance.rs` runs, and the crate spec asks for a BDD runner only of a crate that has a feature file.

What the rules cannot hold, and review does: that the library parse of a language is the library parse and not more (the allocation bound is its `read_tree` through a builder that builds nothing, so a language that allocates there on purpose raises its own bound), and that a language adds no dependency or allow-list entry that would let it name the engine's crates (`language-depends-on-ryo-only` holds the dependency of its manifest, not a path put in its own `imports-allow`, which a language grants itself).

Copy nothing: the template of a `languages::Parse` and of a `languages::Project` use case renders no relation method, so the
compiler lists the ones left; the work is to answer each, as a `Reads` (item, fixture) or as `NotInThisLanguage` with its `Reason`. Add the directory `languages/<lang>/`: the one entry `language` places it by the `{lang}` capture (its package is `idiomatic-sh-ryo-<lang>` by interpolation, nothing lists it): the `language` spec places the whole tree, every part at count 1 (`tm scaffold language --at languages/<lang> --set library=<parser crate> --set ext=<extension> --set relation=syntax_tree` writes it), including its allocation and conformance tests, the parser and tool crates it reads with are declared once in its own
`tastemaker.toml` (`imports-allow` for its imports, `client-at-adapter` for where it names them) and in its own `Cargo.toml`, and the command line links it by the Cargo
feature of its name (none is a default: a build names its set, `--features rust`, or `--all-features` where every language is exercised; the crate's own tests and benches name `rust` through a dev-dependency on itself): Cargo forces that line in `cli/Cargo.toml`, the build script of `cli` fails the build when a
directory of `languages/` has none, and writes `cli/src/languages.rs`, the one file that links them.
