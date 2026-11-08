# The language contract

Ryo owns the concepts: the relations of the catalog. A language fulfils them. A language implements two
traits of `ryo::languages`, `Parse` and `Project`, and that is all of ryo it implements: every relation of the
catalog is one required method of one of them, with no default body, so a relation added to the catalog is a
compile error in every language until the language has answered it. The crate does not compile with a relation
left out, and no engine code lists a language or a language's file names.

## What a language answers

| trait | what it says | per relation |
|---|---|---|
| `Parse` | `recognizes(request)`: whether it reads the file; `skips()`: the names of the directories it never reads, at any depth (`node_modules`, `target`; empty when its ecosystem has none), declared as `SKIPS` in `domain/language/constants.rs` and not entered by the walks of ryo and Tastemaker; `kind_count()`: how many kinds its trees have (at most `KIND_LIMIT`, 1024); `kind_names()`: the names of those kinds, each once, the words a query may write for a kind; `unknown_reasons()`: its closed list of reasons for a value it cannot resolve (empty when it has none); `file_model(tree, model)`: refills the per-worker model of the file whose tree it is, which the readers of the file read; `read_tree(request, builder)`: reads the file with its parser library into the builder the engine gives, and nothing else; `complete(request, completion)`: names the kinds and writes the notes. The engine composes `build` from them, and the allocation row's bound is `read_tree` through a builder that builds nothing, so no body can be written beside the library to be compared with | one method for each syntax relation (`function`, `comment`, `syntax_tree`, `mapping`, ...): `Support<Reader<R>>` |
| `Project` | `manifests()`: the names of the files it reads as manifests; `resolve(request)`: reads them through its own tools and answers the bytes the readers read and the run keeps | one method for each manifest relation (`package_dependency`, `workspace_member`, `source_dependency`, `analyzed_source`): `Support<ManifestReader<R>>` |

The relations the engine gives for every file (`filesystem.file`, `filesystem.line_count`) are in neither trait: a
language cannot write them.

Each answer is `ryo::languages::reads::<Item>()` (`reads_manifests::<Item>()` for a manifest relation) or `ryo::languages::absent(reason)`: a `Reads` is made only from an item that implements `ReadItem` or `ManifestItem`, never from a reader written beside it. The reason is one of
`NoSuchConstruct`, `NotCode`, `NotAKeyedDocument`, `NotADocument`, `NotYetRead`, `NoManifests` (see "Why a language does not answer a relation"); `ryo relation describe <relation>` lists, for
every linked language, `reads` or the reason.

A reader gets the rows of its own relation to write and no other: `Reader<R>` is given `&mut dyn Rows<R's row>`,
`ManifestReader<R>` the same. The engine reads the tree once for the readers of every wanted relation, so a
language never receives the writer of the whole store.

## Scopes

Every scope has a row of `scope`, including the top-level ones: a function, a class or any other item that makes a scope at the top of a file has the file's module scope for its parent. The module scope itself has a row too, with its own node for its parent (the root of a tree is its own parent in the element table, and here too), so the shape has no parentless row: a scope with no parent is not a shape. It is the only row of its file with that. The conformance holds it: of the scope rows of each sample file, exactly one has the same node for parent and child, and every other has a parent that is a child of a row of the file. A file with nothing in it still gives that one row. Rust's scopes are the module of the file, a nested module, an `impl`, a trait and a function.

## Documents

A language of documents (Markdown, HTML) says what a document declares with the shared kinds of `defines`: a titled section is `defines { kind: section }`, nested through `scope` as functions nest in classes, and an anchor, an id or a label is `defines { kind: label }`. Code languages write no sections; Go writes labels, one for each statement label (`Again: for { ... }` is `defines { kind: label, name: Again }`). Markdown reads them: a heading is a section (named by its title, held by the section around it, scoped from the heading to the next heading as shallow or shallower), and the definition of a link reference, of a footnote and an `id` or `a name` of raw HTML are labels.

A link target is `reference.target`, and the language resolves it to a run-relative path as it resolves an import (`static_import.target`): `[text](../guide.md#setup)` in `docs/a.md` has the target `guide.md#setup`, where `docs/` is the document's own directory; a fragment alone is the document's own path with it (`docs/a.md#top`), a path from the root (`/x.md`) is the run's own `x.md`, and a destination with a scheme or a host (`//cdn.example.org/x.js`) stays as written. The language does it where it builds the tree: `read_tree` is given the request with the file's path, and writes the resolved target as the decoded text of the first marker of the link, which `named_by` reads back (Markdown's adapter shows how). HTML resolves the same way the value of each attribute HTML defines as a URL (`href`, `src`, `action`, `poster`, `cite`, `data`, `formaction`, an SVG `href`), after it strips the white space around it as HTML does: `href="../guide.html#setup"` in `docs/a.html` has the target `guide.html#setup`. Its adapter writes the resolved target on the attribute's node as a property of the language's own that is no DOM property and has no node, and the attribute's `value` keeps what the file writes, so no part of the written link is lost. RyoQL has nothing for it: a rule compares the target with a path or a `defines.name` as it compares any two names.

The text a reader reads is `prose { id, owner, text, file, span }`: one row for each run of a document: a paragraph, the title of a heading, a cell of a table, the text an item of a list holds itself (an item of paragraphs gives the rows of its paragraphs). The `text` joins the markup of the run and masks what a reader does not read as prose: inline code, math and raw HTML become a space, a break is a space, and the white space between words is one space. The `owner` is the section that holds the run, or the document. A language that is not a document answers `NotADocument`.

## Directives

An instruction to the build tool that a file carries is `directive { file, span, kind, text }`: `kind` is the directive's name as the language writes it and `text` its argument text as the tool reads it. Go's `//go:build linux && amd64` is `kind: go:build, text: linux && amd64` and a header `// +build linux` is `kind: +build, text: linux`; the line is also a `comment` row, and it is no `prose` row, which is the text a reader of a document reads. The kinds are the language's words and ryo keeps no list of them. A language with no such instruction answers `NoSuchConstruct`.

A comment is documentation (`comment.doc` raised) when the language's own tools read it as the documentation of what follows or encloses it: Rust's `///`, `//!`, `/** */` and `/*! */`; JavaScript's and TypeScript's `/** */`; Go's comment directly above a declaration, with no blank line between. Every other comment is not, and neither is any comment of CSS, HTML, Markdown, TOML or JSON, which have no such convention. A Python docstring is a string the language reads as `__doc__`, not a comment: it is a `string-literal` row, and Python's `#` comments are never documentation.

## Declarations: visibility, implementations, decorations, annotations

The visibility a declaration is written with is `visibility { occurrence, declaration, text, file, span }`: `declaration` is the node the
declaration's `defines.id` is, `text` the visibility as written with its white space one space (`pub`, `pub(crate)`, `pub(in crate::a)`
in Rust; `export` and `export default` in TypeScript and JavaScript, written on the declaration an export holds, on each declarator of an
exported variable declaration; `public`, `private`, `protected` on a TypeScript class member). An export of names (`export { a }`)
declares nothing and has no row. A language with no syntax for it answers `NoSuchConstruct`: Python (a leading underscore is a naming
convention) and Go (an identifier is exported by its first letter). The words are the language's and ryo keeps no list of them.

A row of `function` is a declared function, as its catalog summary says ("A function declaration."): a function declaration, and a
method, named by its name or by its key; a method's row is the node of its body where the language's tree splits the two
(JavaScript's and TypeScript's `MethodDefinition` holds a `FunctionExpression`). A function written as a value is no row, whatever
the language calls it: Rust's closure, Python's `lambda`, Go's function literal, JavaScript's and TypeScript's arrow function and
function expression (`n: function () {}`); its parameters are still `parameter` rows ("A parameter of a function, method or
closure."). A method whose key is computed or is not a name has the unknown value for its name.

A block that implements a type, or a contract for a type, is `defines { kind: implementation }`: Rust's `impl`, refinement `impl`,
named by its header without its generic parameters, generic arguments and where clause (`Display for Point` of
`impl<T: X> Display for Point<T> where T: Y`), its id the node of its `scope` row. No other linked language has the construct.

A decoration's arguments are `decorates.arguments`: what its delimiters hold, as written (`Clone, Debug` of `#[derive(Clone, Debug)]`,
`"/", methods=["GET"]` of `@app.route("/", methods=["GET"])`), the value of Rust's `#[name = value]`, empty when it passes none.
`decorates.declaration` is the declaration's `defines.id`, and the file's own node for an inner attribute (`#![..]`), the node of the
file's module scope. Rust's `annotates` reads the type of a `const` and a `static`, owned by the item's node, its `defines.id`.

## Why a language does not answer a relation

| reason | said by | when |
|---|---|---|
| `NoSuchConstruct` | any language | the language has no such construct (Markdown has no `function`) |
| `NotCode` | a data format | the language is data: it reads no relation of code (TOML says it of `function`). A language that reads any relation of code never answers it |
| `NotAKeyedDocument` | a language of code | it has no keyed documents |
| `NotADocument` | a language that is not a document | it has no prose; one that reads `prose` never answers it |
| `NoManifests` | a language with no manifest | it names no manifest file |
| `NotYetRead` | a language being written | the construct is in the language and its reader is not written yet. It is a stage, never an answer for a finished language: a finished language answers `NoSuchConstruct` (the construct is not there) or reads it. A language the command line ships by default (a default feature of `cli/Cargo.toml`) fails its conformance for each relation it answers `NotYetRead` |

The conformance holds each reason against the class of the relation and against the other answers of the language.

## Manifests are files

A column that names a manifest holds the kind File, never text: `workspace_member.workspace` and `package`, `file_package.package`, `package_dependency.source` and `target`. A File is the number of the file among the files the run lists, in the order of their paths, so a rule joins a manifest with `filesystem.file.path` as it joins any file. A manifest the run does not list (a workspace names a member the scope leaves out) is still a file the rows name: its number is the one after the last file the run lists, in the order of the paths of such manifests, and the language says which they are in `Resolved::unlisted`. Two languages that both name manifests the run does not list are refused: the numbers after the listed files are one language's to give. A language that reads files beyond the manifests it is handed (Rust reads the Cargo configurations it finds above and in the repository, and the manifests it reads from the disk) names each in `Resolved::consulted`, by its path from the root (`../` above it): the engine digests them and reads the project again only when one is gone or changed. A language that reads only what it is handed answers none.

### What each language's manifests say

A manifest is a file the language names (`manifests()`, the last part of the path) and reads with the parser of its format. Every language answers the same relations from it: a *package* is a manifest that names one; a *workspace member* is a package the workspace manifest holds; a *dependency* is every requirement a package declares, written with the kind below, whether the repository holds the package or not: no declared dependency is dropped. The `target` of a requirement is the manifest of the package of the repository the requirement names, found by the name the format normalizes or the directory it names; a requirement none of the repository's packages answers to (a package of a registry, a git repository, an archive, a directory outside the repository) has an empty `target` (`NO_FILE`, shown as empty text, which joins no file), so a join on `target` keeps the graph of the repository's own packages. `origin` says where the manifest says the package comes from, one of `registry`, `path`, `git`, `url` and `workspace`, as written: a version range is `registry` even where a workspace member of that name answers it, and a `workspace` origin is a member found by its name (a manifest that inherits a dependency's declaration from its workspace says the origin of the workspace's entry). `package_dependency.package` is the name as the format normalizes it and `binding` the name as the manifest writes it; `condition` is the condition text as written (a marker, a build tag); `optional` is `Optional` for what the package needs only when asked for. `requirement` is the version requirement as the manifest writes it, operators included (`=1.2.3`, `^4.6.0`, `==0.115.0`, `v1.4.0`), empty when it writes none (a path or a git revision alone); `exact` is raised when the requirement admits one version only in the ecosystem's own grammar, and lowered for a range, a wildcard, a minimum and an absent requirement. The grammar of a requirement is the language's to parse; the two columns carry nothing of it but the text and the one test. The text a manifest writes about a dependency besides its requirement is a column each, as written and empty when it writes none: `branch`, `tag` and `revision` are the revision of a git repository the dependency names, by what the manifest says it is (`revision` when it does not say whether it is a branch, a tag or a commit: Cargo's, Poetry's and uv's `rev`, Pipenv's `ref`, the `@` of a PEP 508 git URL, the `#` of a package.json git spec); `interpreter` the versions of the language's interpreter the dependency applies under, written on the dependency itself (Poetry's `python`); `catalog` the shared table the requirement is taken from instead, spelled as written (`catalog:`, `catalog:<name>`, Cargo's `workspace = true`), `requirement` then holding what the table writes; `replacement` what the manifest puts in the package's place (a go.mod `replace` target with its version, a Cargo `[patch]` or `[replace]` directory, an `overrides` or `resolutions` spec of package.json, a uv `override-dependencies` requirement).

| | Python | Go | JavaScript and TypeScript | Rust (as built) |
|---|---|---|---|---|
| manifests | `pyproject.toml` | `go.mod`, `go.work` | `package.json`, `tsconfig.json` | `Cargo.toml` |
| parser (format) | TOML, requirements by PEP 508 | the `go.mod` line grammar | JSON, `tsconfig.json` as JSON with comments | TOML |
| a package | `[project]` with a `name`; the name is normalized as PEP 503 does (lower case, each run of `-`, `_` and `.` one `-`) | the `module` of a `go.mod`; the name is the module path | a `package.json` with a `name`; the name is as written | a `[package]` |
| a workspace | `[tool.uv.workspace]` `members` (globs, from the directory of the file) less `exclude` | `use` directories of a `go.work` | `workspaces`: a list of globs, or `{ packages: [..] }` | `[workspace]` `members` less `exclude` |
| a member | the package in a directory the globs hold | the `go.mod` in a `use` directory | the `package.json` in a directory the globs hold | the package in a directory the globs hold |
| a dependency, its kind | `project.dependencies` is `normal`; `project.optional-dependencies.<extra>` is `normal` and `optional`; `dependency-groups.<group>` (PEP 735) is `dev`; `build-system.requires` is `build` | `require` is `normal` (`// indirect` too); `replace` to a directory points the requirement at it | `dependencies` is `normal`; `devDependencies` is `dev`; `peerDependencies` is `peer`; `optionalDependencies` is `normal` and `optional`; a `references` path of a `tsconfig.json` is `normal` between the packages of the two directories | `[dependencies]` is `normal`, `[dev-dependencies]` `dev`, `[build-dependencies]` `build` |
| how a requirement's target is written | a PEP 508 string, `name[extras] (>=1) ; marker`: the name is normalized and the marker is the `condition` | a module path; a `replace` path names the directory | a package name; `workspace:*` and a `file:` path name a member | a crate name or a `path` |
| the version requirement, exact | the version specifier as written, after the name and extras and before the marker (`==0.115.0`, `>=1,<2`), empty for a URL or a name alone; exact for a PEP 440 `==` without `.*`, or `===`, in any clause. Poetry's constraint string or `version` key as written, exact for a bare version (Poetry reads `1.2.3` as `==1.2.3`) or a PEP 440 pin, never with `\|\|`; Pipenv's string or `version` key, read as PEP 440 (`*` is no pin); a requirements file line as PEP 508; uv's tables add sources, not requirements | the `require` version as written (`v1.4.0`); never exact: a `require` is the least version minimal version selection accepts | the spec as written (`^4.6.0`, `workspace:*`, `npm:other@1.2.3`), a `catalog:` spec once resolved the range its catalog writes; exact when the version past a `workspace:` or an `npm:` alias is a whole one (`1.2.3`, `=1.2.3`, `v1.2.3`); a directory, a git repository and a URL are not | the `version` as written (the workspace's for one taken from it), empty for a path or git dependency with none; exact for `=` with a major, a minor and a patch in any comparator (a bare `1.2.3` is a caret) |
| what is not a row | a requirement on a package outside the repository | a module outside it | a package outside it; the `paths` of a `tsconfig.json`, which help `source_dependency` and are no dependency | a crate outside it |
| `source_dependency.written`: the text from the row's `span` to its end as the file writes it, its whitespace left out | the alias of the import (`workers` of `from . import workers`) | the import spec, its name included (`"example.com/app/internal/greet"`) | the declaration or the expression that imports (`import{helper}from'./lib/helper';`, `require('./lib/util.cjs')`) | the path (`util::double`; a leaf of a `use` tree as the leaf writes it); a path that starts with the crate a dependency on another package of the repository names (the dependency's key when it renames the package, else the library's name) names that library's file (`helper::ready`) |

Python is the first worked case: `pyproject.toml` is read with a TOML parser, each requirement with a PEP 508 parser, and the three relations `package_dependency`, `workspace_member` and `file_package` are read from the packages found (the package a file belongs to is the one whose directory is the nearest above it). The contract's types carry no manifest format: a manifest is a path and a text, what the language works out of the manifests is its own `Derived`, and the rows say Files, names and kinds from the shared words.

Each ecosystem's own tool is the reference these rows are held to, in `cli/tests/replay/manifests/`: one test for each tool reads a project with ryo and with the tool and compares the members and the dependencies the two say (`cargo metadata` over the pinned targets; `go list -m -json` and `go mod edit -json`, `npm ls --json`, `pnpm ls --json` and `uv workspace metadata` over a fixture project of `cli/tests/fixtures/manifests/` or the Go language's own; Bun's `bun.lock`, which the fixture holds as Bun writes it).

## A language's shape

The tree is one template followed over and over, so that it is simple to guess and to hop from one language to another. Every language has this tree; `*` marks a part present exactly when its trigger holds.

```text
languages/<lang>/
  Cargo.toml  tastemaker.toml  README.md  benchmarks/provider/bench.rs   (the bench is always there, placed at count 1: the corpus is the workload)
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
      <other ports>/...                             free-form, where the language's logic needs IO (Rust's job_scheduling)
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

**The scaffold.** `tm scaffold language` writes part of a language's tree today; the author writes the rest.

## Where each part goes (`languages/<lang>/`)

| part | where | placed by |
|---|---|---|
| `impl Parse`, with every syntax answer | `src/inbound/contracts/parse/contract.rs` | the `language` spec's `parse-contract` part (`languages::Parse`) |
| `impl Project`, with every manifest answer | `src/inbound/contracts/project/contract.rs` | the same (`languages::Project`) |
| the item of a relation it reads: the reader and the rows it writes | `src/domain/{relations,manifests,links}/collections/<collection>/<name>/item.rs`, and nowhere else (the rule `readers-in-items`); the item is declared with `ryo::reads!`, the only way an item gets `ryo::languages::ReadItem` (syntax relations) or `ryo::languages::ManifestItem` (manifests and links): both traits are sealed, so a reader written by hand does not compile (the ui case `fail_read_item_written_by_hand`) | the compiler: `reads::<Item>()` makes the answer from the item, whose `READER` is a `Reader` of the relation the trait names; the conformance holds the rows |
| its own tastemaker.toml | `tastemaker.toml` | declares the parser libraries it reads with |
| the syntax adapter (the parser library, behind the `syntax` port) | `src/outbound/syntax/adapters/<library>/adapter.rs` | the `port` and `adapter` specs; the parser's crate is imported there and nowhere else |
| the manifest reading adapter (the tool that reads manifests) | `src/outbound/manifest_reading/adapters/<tool>/adapter.rs` | the same; the only place a language touches the file system or runs a process |
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
| it answers every relation | `Parse`, `Project`: one required method for each, which a scaffold leaves out so that the compiler lists them | `Reads` has its fixture and its full rows, `NotInThisLanguage` has no fixture with the construct and a required `AbsenceCause` | `language-parts` (`no-parse`, `missing-project-use-case`) |
| its kinds are one vocabulary | the adapter maps every library name onto the variant of the same name; `sample(kind)` is a match with no wildcard arm, so a kind the grammar gains is a compile error until it is shown | `kind_count` is above 0 and fits a walk; every kind with a sample is read by some relation's `kinds:` or marked `Sample::Unread(reason)` (the unread check), and a sample's tree holds an element of its kind | `language-parts` (`missing-kinds`, `kind-directory`) |
| it reads with a parser library | | | `language-parts` (`missing-syntax-adapter`), `client-at-adapter` |
| it reads a project | | the full typed rows of each `manifests/<case>/` | `language-parts` (`missing-project-adapter`, `missing-manifests-fixtures`) |
| it is held to fixtures | | `tests/fixtures/{relations,manifests,allocation}` | `language-parts` (`missing-relations-fixtures`, `missing-allocation-fixtures`) |
| a row is numbers | | the allocation test | `no-string-fields` |
| it touches no file or process but in its manifest reading adapter | | | `io-at-boundary` |
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

A word compared with a column that holds a closed set of words is one of them: `kind = "headline"` on `defines` is refused when the query is compiled, with the words there are (`` `headline` is not a kind of defines; the kinds are function, method, .. ``), and the rule does not run. The columns are `defines.kind`, `parameter.kind`, `entry.kind`, `sequence-element.kind`, `mapping.kind`, `package-dependency.kind` and `package-dependency.origin`. A flag column is compared with the word it is shown as: `import = "false"` on `string-literal`, `doc = "true"` on `comment`, `boolean = "true"` on `entry` and `sequence-element`, `exact = "false"` on `package-dependency`.

A word a rule compares the kind of a node or a token with (`syntax-tree.kind`, with `=`, `in` or a glob) names a kind of some linked language, or the run fails before a file is read, naming the word and the languages (`` kind `function-declaration` names no kind of rust, toml, markdown, python ``). A language declares the names of its kinds with `kind_names()`, each once, as many as `kind_count()`; the conformance holds both. The refinement and the field of a node are the language's own words and have no list yet, so a word compared with them is not checked.

## What a reader may rely on

A reader is given the tree: the kinds and the positions of its elements, their text, and the roles of the children under their parents (`languages::field_at`, `field_name_at`) when the run asks for them, which it does only when a rule reads `syntax-child.field`. Markup is reached the same way, through the tree and its roles: an HTML element's or attribute's name, namespace, prefix and value are nodes under it in the roles `name`, `namespace`, `prefix` and `value`, an attribute is in the role `attribute` under its element; a CSS declaration's name, the component values of its value and the `important` of its `!important` are in the roles `name`, `value` and `important`; a Markdown heading opens a `section` node that holds it and the blocks it heads, so a block's parent is its section. A language whose own relations need the role of an element works it out from the tree as Rust does (`fields::field_of`), or writes it with `write_fields` in `complete`; it does not count on the run asking.

A language held to an official suite whose files cannot all be read (written for another version of the language, or valid only to a semantic checker) leaves those files out by name in the suite's `NOTICE` with the reason for each; it never changes a file, and never counts a file it leaves out as read.

## The keyed relations

A data document (TOML, JSON, YAML) is read as three relations, each a different row of the same tree. These are the decisions a language
fulfils; each has an example, written for TOML.

| decision | rule |
|---|---|
| the root | The document is the file's root element (position 0). A document whose top level is a table of keys has one root `mapping` row whose `id` is the root element, whose `kind` is `standard` and whose `owner` is the root element itself. A document whose top level is a sequence has no root mapping; its `sequence_element` rows have the root element as `owner`. |
| the anchor of a mapping | `mapping.id` is the element where the mapping is first written: its header (`[a]`, `[[a]]`), the first dotted key that names it (`a.b = 1` names `a`), or the first longer header that implies it (`[a.b]` implies `a`). A mapping a longer header implies before it is written by itself is `implicit`, and stays so when a later header writes it, anchored at that first longer header's segment, so the same mapping has the same id however many later headers pass through it. |
| `entry.ordinal` | The place of the key among the keys of its owner, from zero, in the order the keys are first written, counting each key once (a table reopened by a second header continues the count). |
| `mapping.ordinal` | The place among the mappings of a sequence of mappings (`[[a]]`), from zero; zero for every other mapping. |
| `sequence_element.ordinal` | The place in its sequence, from zero. |
| `owner` | `entry.owner` is the mapping that holds the key (the root mapping for a top-level key); `mapping.owner` is the entry or sequence element whose value the mapping is (the root element for the root mapping); `sequence_element.owner` is the entry or element that holds the sequence. |
| value kinds | `string`, `integer`, `float`, `boolean`, `datetime`, `sequence`, `inline-mapping`, `mapping`, `sequence-of-mappings`, and `null` for the value that says there is none (JSON; TOML never writes it). |
| `decoded` | The text the language's library decoded for a string value (`decoded_token` in the table builder), empty for every other kind; `text` is the value as written. |
| `boolean`, `integer`, `float`, `datetime`, `radix` | The value decoded with its type: `boolean` is raised for `true` and lowered for `false`; `integer` is the whole number (`0x1F` is 31, `1_000` is 1000) and `radix` the base it is written in (16 for `0x1F`, 8, 2, 10); `float` is the number in the shortest decimal form that reads back as it (`1e3` is `1000.0`, `+inf` is `inf`); `datetime` is the date, time or both as RFC 3339 writes it (`1979-05-27 07:32:00Z` is `1979-05-27T07:32:00Z`). Each is lowered, zero or empty for a value of another kind, which `kind` tells apart, and `integer` is zero for an integer the engine's numbers cannot hold; `text` keeps every value as written. A rule compares them as any flag, number or text: `key = "strict" and boolean = "true"`, `kind = "integer" and integer > 3`, `radix = 16`, `float_text_gte(float, "0.5")`. |

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
| the value | a name column holds either a name or an unknown value: `unknown_symbol(UnknownReason(n))` is the same word in every file, `n` being the number of the reason in the language's list |
| the list | `Parse::unknown_reasons()` is the language's closed list of `UnknownName`s, each a number and a name, each named once; a language with no unresolvable construct declares `&[]` (Rust does). The numbers of the languages linked together are apart, so a shown value names its reason whichever language wrote it: JavaScript and TypeScript use 0 (the same reason, the same name), Python 16 to 31, Go 32 to 47, CSS 48 to 63; a new language takes the next free range of 16, and the replay test holds one number to one name |
| the answer | a query's answer shows an unknown value as `unknown: <reason>` (`unknown: dot-import`), never as an empty cell; a reason no linked language declares is shown as `unknown` |
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

The unknown value is a word of its own among the words of the closed sets (`terms`, block `Unknown`), so it compares equal to itself in every file. The conformance shows it by the reason's name, and an engine listing shows it by the same name from the languages' lists.

## A file is read, never refused

Bytes that are not valid text are read: the valid text keeps its tree, and the rest is an error token with a diagnostic, at the offsets it has in the file (the Rust language reads each byte that is part of no character as a control character the lexer reports). `ParseError` has no case for them: a refusal is for a language that does not read the file (`Unsupported`) and for a library that fails (`Library`), and the shared rails treat a refusal of a file of the suite or the corpus as a failure.

## What a language works out of a file once

A language that would search a file again for each element a reader is given (TOML: the header an entry belongs to, the ordinal of an entry) works it out once, when it parses the file, and its readers read it. `Parse::FileModel` is the type (`()` for a language that needs none; it is `Default`) and `Parse::file_model(tree, model)` refills it: the engine makes one model for each worker, keeps it with the worker's table between files and gives it back, so a file costs no allocation the last did not pay, and the language clears what it keeps and writes the file's. The model and the language that makes it may be private to their crate: the associated type of a crate-private type is crate-private, and the engine holds the model behind an interface that never names it (`tests/ui/pass_crate_private_file_model.rs`); a reader is declared `Item => Relation, model(FileModel) |tree, model, at, symbols| { .. }`. The text a library decodes from a token (the value of a string with its escapes resolved) is written by the adapter with `decoded_token`, and a reader reads it with `decoded_at`; no language decodes text a second time.

## Per-language notes

- JavaScript and TypeScript follow oxc's source-type defaults: JSX in `.js`, `.jsx` and `.tsx`, `.mjs` is a module, `.cjs` is CommonJS, `.js` is unambiguous. JSX in a `.mjs`, `.cjs`, `.ts`, `.mts` or `.cts` file is a `syntax_error` row, as oxc reports it.

## Semantic syntax projections and advertised optional capabilities

### Semantic syntax projections

`semantic-syntax` and `semantic-child` are explicit projections, separate from
native lossless syntax. Providers without embedded expressions reuse the shared
native projection readers. A provider that parses an embedded expression keeps
its logical token text as owned derived symbols and records its physical source
mapping independently. `embedded-expression` identifies the carrier, root,
evaluation context, and mapping fidelity. An embedded occurrence is distinct
from a macro expansion and from parser-implied syntax.

Every provider inherits conformance checks for native row and field-predicate
preservation when there are no embeddings, physical span bounds, source-coordinate
identity, and deterministic retained-table reuse. Field predicates also participate
in core demand planning: requesting a semantic edge field must request its roles,
even when that field is filtered out of the final query result.

### Advertised optional capabilities

Every parser explicitly declares its `OptionalCapability` set. The catalog currently includes
canonical identifiers, qualified-name provenance, embedded expressions, expression shapes,
static binding events/reads/uses/shadows, implicit symbols, literal properties, interpolated-
string fragments, language targets, constant keys, and percent-format parse/field facts. Every
other catalog relation remains the required baseline, including both semantic syntax
projections. Advertising a capability promises a reader; it cannot return `NotYetRead`.
Duplicate declarations fail conformance.

An unadvertised optional reader is never invoked. Its absence is `NotYetRead`, and a direct
read records missing rows rather than a known-empty relation. The conformance run prints
all unavailable capabilities even when the provider otherwise passes. Retained-reader
checks exercise unavailable → advertised → unavailable requests; native projections,
field filtering, source mapping, and reparse checks remain shared requirements.

Current availability is explicit:

| Providers | Optional promises |
| --- | --- |
| Python | Its declared semantic, literal, binding, target, constant-key, and percent-format readers |
| JSON, TOML | Identifier and qualified-name readers |
| Markdown | Qualified-name reader |
| CSS, Go, HTML, JavaScript, Rust, TypeScript | None; these optional facts remain unavailable |

Percent-format parsing and conversion-field facts are currently advertised only by Python;
other providers explicitly report `NotYetRead` for those two relations.

The parser-implied nodes of languages such as HTML retain their explicit absent source
span in the semantic projection. Shared guards accept that only for an actual implied
node, require no fabricated source-location row, and still reject unlocated ordinary nodes.

## The conformance

`ryo::languages::contract::conformance(linked, manifest_dir, fixtures)` holds each answer to the fixtures and the typed rows
the language states (`Fixtures`: its kinds, the rows of each relation, the rows of each project), the same for every language:

- a relation answered `Reads` has `relations/<relation>/` with an input, and the rows the language writes for the input, each in
  full (a node as its place in the table, a name as its text, a span as `line:column` on each end), are the typed rows the language
  states, no more and no fewer; a mismatch prints the rows the language wrote in the syntax of `expected.rs`
  (`span((1, 1), (1, 8))`), so a wrong expectation is corrected by copying them, and `cargo test --test conformance -- --bless` writes them as the `expected.rs` of each relation, to be reviewed in the diff;
- a relation answered `NotInThisLanguage` has no `relations/<relation>/` (a fixture with the construct present
  fails the answer) and says why in its required `AbsenceCause`; a scaffold answers nothing, so the compiler lists what is
  left to answer, and no default can pass for an answer;
- Every column of every relation the language reads is shown filled by its fixtures: each word of a column of a closed set of words, `true` and `false` of a flag, and a value that is not empty (not an empty name or file, not zero, not a value the language cannot resolve) of any other column. What the fixtures cannot show, because it never applies in the language, is declared in `Fixtures::unfilled` with the reason (`Unfilled { relation, column, unshown: Unshown::Filled | Unshown::Word(..), reason: Why(..) }`), citing the standard or the construct, as an absent relation cites the standard; a declaration of what a fixture shows is a finding too.
- `--bless` writes the `expected.rs` of each relation from what the language wrote. Where the expected rows do not compile (a column was added) or are not there, build the language without them: a language leaves them out of its build under `#[cfg(bless)]` (Rust: `fixtures.rs`), and `RUSTFLAGS='--cfg bless' cargo test -p <crate> --all-features --test conformance -- --bless` writes them from scratch;
- the rails hold every input of the language (the inputs of its relations' fixtures, `tests/fixtures/suite/valid/`, `tests/fixtures/suite/invalid/` and `tests/fixtures/corpus/`, each required to hold a file) to four checks: (1) the tokens of the tree, in order, spell the file byte for byte; (2) the tree has the shape of the library's own tree (nesting, the kind the language maps each kind of the library to, spans), by the walk of its library the language supplies (`Fixtures.walk`, giving `Walked`); (3) the rows say what the library's own semantic model says, by the oracle the language supplies (`Fixtures.oracle`, given the text and the rows of every relation it reads from trees, answering what differs; the oracle is not run on the suite's invalid files); (4) the suite's `valid` files give no syntax error and its `invalid` files at least one. The suite is the language's pinned official one (for Rust, rust-analyzer's `crates/parser/test_data/parser/{ok,err,inline}` from the pinned `ra_ap_parser`; a file written for an edition the language does not read is left out, with the reason in `corpus/NOTICE`); the corpus is real files, whose origin and licence `corpus/NOTICE` states;
- `syntax_tree` and `syntax_error` are relations like the others: the input of the `syntax_error` fixture is the
  broken file the contract reads, and a language that answers `syntax_error` `NotInThisLanguage` has none (a broken input a
  tool of the repository cannot read is left out of it by name, once: the `exclude` of `taplo.toml`, matching
  `languages/*/tests/fixtures/relations/syntax_error/input.toml`, and a language's pinned suite, `languages/*/tests/fixtures/suite/`,
  which the file hooks leave byte for byte; the `rules-exclude` of the product's `tastemaker.toml` leaves out the input of every
  relation, `languages/*/tests/fixtures/relations/*/input.*`, which is written in the language's words to show a construct, never as
  the product's code);
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

Copy nothing: the template of a `languages::Parse` and of a `languages::Project` use case renders every relation method, `syntax_tree`
read and each other one `absent(AbsenceCause::NotYetRead)` (`absent(AbsenceCause::NoManifests)` in `Project`); the work is to answer each, as a `Reads` (item, fixture) or as `NotInThisLanguage` with its `AbsenceCause`. Add the directory `languages/<lang>/`: the one entry `language` places it by the `{lang}` capture (its package is `idiomatic-sh-ryo-<lang>` by interpolation, nothing lists it): the `language` spec places the whole tree, every part at count 1 (`tm scaffold language --at languages/<lang> --set library=<parser crate> --set ext=<extension> --set relation=syntax_tree` writes it), including its allocation and conformance tests, the parser and tool crates it reads with are declared once in its own
`tastemaker.toml` (`imports-allow` for its imports, `client-at-adapter` for where it names them) and in its own `Cargo.toml`, and the command line links it by the Cargo
feature of its name (none is a default: a build names its set, `--features rust`, or `--all-features` where every language is exercised; the crate's own tests and benches name `rust` through a dev-dependency on itself): Cargo forces that line in `cli/Cargo.toml`, the build script of `cli` fails the build when a
directory of `languages/` has none, and writes `cli/src/languages.rs`, the one file that links them.
