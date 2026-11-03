# Ryo: the design

This is the one document ryo is built from. It states the design as it stands, once, with no history. The performance work that shaped it is not part of the design and is not published here. Ryo is built in `crates/ryo` with Tastemaker's scaffold; the specs, scaffolds and rules in `tastemaker/` and `crates/ryo/tastemaker/` define the tree, and where this page and they disagree they win.

# A. The design

## A1. Crates, and why each is its own

A crate exists where there is a real boundary: something outside it calls it through its use case, which shows the crate's entry point, or it has a lifecycle of its own (a database, an add-on installed separately). Nothing else is a crate; inside a crate everything is a module of one of the Tastemaker primitives.

Packages are `idiomatic-sh-ryo-<crate>` with lib `ryo_<crate>`; the SDK is `idiomatic-sh-ryo-sdk` with lib `ryo`; the CLI is package `idiomatic-sh-ryo` with binary `ryo`. The `[lib]` and `[[bin]]` names above are set in each manifest.

| path | package / lib | kind | why it is a crate |
|---|---|---|---|
| `cli` | package `idiomatic-sh-ryo`, binary `ryo` | cli + crate | an edge: parses the command line, calls one use case, prints (the CLI and the SDK are separate edges) |
| `crates/sdk` | package `idiomatic-sh-ryo-sdk`, `[lib] name = "ryo"` | sdk + crate | an edge: what people install and build on |
| `crates/core` | `idiomatic-sh-ryo-core`, lib `ryo_core` | nu-hex | the engine: use cases, domain, outbound ports and their adapters |
| `crates/query_language` | `idiomatic-sh-ryo-query_language`, lib `ryo_query_language` | nu-hex | RyoQL is a standalone language with its own use cases, shipped with ryo as a package of its own |
| `crates/values` | `idiomatic-sh-ryo-values`, lib `ryo_values` | nu-hex | the value objects the crates name across their boundaries: plain data and pure functions, no port and no use case |
| `crates/language` | `idiomatic-sh-ryo-language`, lib `ryo_language` | nu-hex | holds `ParseUseCase`, the one contract every language implements |
| `languages/<lang>` | `idiomatic-sh-ryo-<lang>`, lib `ryo_<lang>` | nu-hex with `implements` | an add-on, installed separately: a language is not part of ryo's core architecture |

Languages built: rust, python, typescript, javascript, go, css, html, markdown, toml, json. Each is ported as it is today behind `ParseUseCase`: the same library, kinds, relations and expected findings; a language crate has the `parse` use case only, with no `link` use case and no semantic relations, and nothing semantic is in the SDK. The order of work is: ryo works; then Tastemaker's new version on the three SDK calls; only then semantic analysis. The library each `outbound/syntax/adapters/<lib>` names is the one the language's parser already used.

## A2. How crates call each other

- A crate that is called has `inbound/use_cases/`; callers go through it.
- A crate reaches another of ours through an outbound port of its own, whose adapter calls the other crate's use case; the caller's tests use the port's fake. Core reaches `query_language` through `outbound/query_language/` (`QueryLanguagePort`); the adapter calls query_language's use cases.
- Core reaches every language through `outbound/language/`, one generic adapter over `ryo_language::ParseUseCase` and `ProjectUseCase`, the traits languages implement as `ryo::languages::Parse` and `ryo::languages::Project`.
- A language crate is a nu-hex that depends on the SDK and whose `parse` and `project` use cases implement `ryo::languages::Parse` and `ryo::languages::Project`, which `ryo::link!` makes one `ryo::languages::Language`; it never names `ryo_language`. The nu-hex param is `implements = [{ use_case, crate, interface }]`, here `[{ use_case = "parse", crate = "ryo", interface = "languages::Parse" }, { use_case = "project", crate = "ryo", interface = "languages::Project" }]`, default `[]`, set only by the `languages/*` entry. A use case it lists has no `interface.rs`, `types.rs` or `errors.rs`; its `use_case.rs` renders `impl <crate>::<Interface> for <Uc>UseCaseImpl`; its `use_case_test.rs` runs that crate's fixtures/contract. A crate without the param keeps the normal shape; implementing another crate's interface is the exception.
- The `ryo query validate` and SDK `query::validate` edges call core's `validate` use case once; it validates through `QueryLanguagePort` and checks the kinds the query names against the linked languages.
- The edge opens the clients (`connections.rs`), wires adapters (`composition.rs`) and calls one use case. Edge flags that choose behaviour are injected adapters, not request fields.
- The CLI and the SDK do not share an implementation; a CLI verb may call an SDK entry when it happens to need exactly that.

## A3. The tree

`<name>` is a placeholder. `?` marks a file present only when the primitive's spec makes it optional; every other file is required.

```
crates/ryo/
  tastemaker.toml                   places every crate below
  Cargo.toml  Cargo.lock  tastemaker.lock  asymptote.toml  deny.toml  README.md
  cli/                             package ryo
    Cargo.toml  README.md  benches/
    benches/performance/<resource>.rs   benches/performance/<resource>/<verb>.rs   benches/performance/<resource>/<verb>/bench.rs
    src/main.rs  src/lib.rs  src/cases.rs  src/resources.rs     the cli spec's fixed shell: output, outcome (with remedy), pager, dry_run, progress, timings
    src/resources/<resource>.rs
    src/resources/<resource>/<verb>.rs
    src/resources/<resource>/<verb>/{command.rs, command_test.rs, composition.rs, composition_test.rs?, schemas.rs, schemas_test.rs, fixtures.rs?, connections.rs?}
    tests/command_tree.rs  tests/command_line.rs
  crates/sdk/                      package idiomatic-sh-ryo-sdk, lib ryo
    Cargo.toml  README.md  src/lib.rs  src/cases.rs  src/resources.rs
    src/resources/<resource>.rs    exposes the types and traits its verbs use (languages.rs: `Language`, `Parse`, `Project` and the types their methods take)
    src/resources/<resource>/<verb>.rs
    src/resources/<resource>/<verb>/{entry.rs, entry_test.rs, composition.rs, composition_test.rs?, schemas.rs, schemas_test.rs, fixtures.rs?, connections.rs?}
  crates/values/                   package idiomatic-sh-ryo-values, lib ryo_values: a nu-hex of domain concepts only
    Cargo.toml  README.md  src/lib.rs  src/cases.rs  src/domain.rs  src/domain/<concept>.rs  src/domain/<concept>/{types.rs, service.rs?, service_test.rs?, constants.rs?}
  crates/core/ crates/query_language/ crates/language/ languages/<lang>/     each a nu-hex:
    Cargo.toml  README.md  src/lib.rs  src/cases.rs
    languages/<lang>/benchmarks/provider/bench.rs   the language's all-relations provider workload
    src/inbound.rs
    src/inbound/use_cases.rs
    src/inbound/use_cases/<use_case>.rs
    src/inbound/use_cases/<use_case>/{interface.rs, use_case.rs, use_case_test.rs, types.rs?, errors.rs?}
    src/domain.rs
    src/domain/<concept>.rs
    src/domain/<concept>/{service.rs, service_test.rs, types.rs?, errors.rs?}
    src/domain/<concept>/collections.rs
    src/domain/<concept>/collections/<collection>.rs
    src/domain/<concept>/collections/<collection>/{interface.rs, types.rs?, errors.rs?, fixtures.rs?, fixtures/contract.rs?}
    src/domain/<concept>/collections/<collection>/<item>.rs
    src/domain/<concept>/collections/<collection>/<item>/{item.rs, item_test.rs}
    src/outbound.rs
    src/outbound/<port>.rs
    src/outbound/<port>/{interface.rs, types.rs?, errors.rs?, schemas.rs?, schemas_test.rs?, fixtures.rs}
    src/outbound/<port>/fixtures/{contract.rs, fake.rs, fake_test.rs}
    src/outbound/<port>/{adapters.rs?, factory.rs?}
    src/outbound/<port>/adapters/<implementation>.rs
    src/outbound/<port>/adapters/<implementation>/{adapter.rs, adapter_test.rs, settings.rs?, types.rs?, errors.rs?, schemas.rs?, schemas_test.rs?}
    tests/bdd.rs  tests/<use_case>.feature  tests/bdd/<use_case>.rs
```

Primitives (the same design primitives repeat throughout the tree): interface, use_case, service, item, adapter, schemas, composition, command, entry, factory, settings, fixture (contract, fake), types, errors, the test beside each logic file, the feature and its runner, the module file beside each directory. A file with no primitive is a finding. Test files are `CASES` tables run by the four case words. There is no `steps/`, no crate-level `types.rs` or `errors.rs`, no `lib/`, `common/`, `utils/`, `shared/` or `helpers/`.

### The concrete layout

| crate | use cases | domain concepts | outbound ports (adapters) |
|---|---|---|---|
| `cli` | none (edge); resources `query/{validate,run}`, `relation/{list,describe}`, `operator/{list,describe}` | none | none |
| `crates/sdk` | none (edge); resources `query/{validate,run}`, `relation/{list,describe}`, `operator/{list,describe}`, `linter/lint`; `languages.rs` | none | none |
| `crates/core` | `lint` (which `query run` runs too), `open`, `validate`, `list_relations`, `describe_relation`, `list_operators`, `describe_operator` | the current scaffold's concepts minus the 18 operator and comparison concepts and minus `compile` (compilation is `query_language`'s, reached through `QueryLanguagePort`), plus `operator` (A6) | `corpus`, `facts`, `lint_store`, `rows`, `scheduler`, `memory`, `digest`, `matching`, `language`, `query_language` (A5) |
| `crates/query_language` | `typecheck`, `compile` | `syntax`, `validation`, `lowering` | `syntax` (adapter `winnow`), `catalog` (adapter `language`, which calls `crates/language`'s `relations` use case) |
| `crates/values` | none | `descriptions`, `files`, `hosted`, `plan_parts`, `plans`, `program`, `records`, `runs`, `stored_facts`, `stores` | none |
| `crates/language` | `parse` (`ParseUseCase`), `project` (`ProjectUseCase`), `relations` (the relation catalog); each with `types.rs`, `errors.rs`; `parse` and `project` with `fixtures/contract.rs` | none | none |
| `languages/<lang>` | `parse` (with `implements`) | the language's kinds | `syntax` (adapter named for the library the language parses with) |

## A4. The CLI and the SDK

**CLI**. Six commands, `<tool> <resource> <verb>`:

| command | purpose |
|---|---|
| `ryo query validate` | check that a query is correct |
| `ryo query run` | explore: one query or several (`--text` repeated), rows of their own columns, run as the rules of a lint |
| `ryo relation list` / `ryo relation describe` | print the relations ryo has (ryo's own catalog only) |
| `ryo operator list` / `ryo operator describe` | print the operators ryo has |

Every other of today's 26 commands is deleted: developer checks become tests, and the reference doc, if kept, is written by a test. There is no `ryo language list`: a language is a plugin, so ryo does not enumerate them; `relation list` shows ryo's own catalog and has no `languages` column.

**SDK**. The library is `ryo`; entries are `ryo::<resource>::<verb>(request)`, mirroring the CLI's resource then verb. Eight verbs, each entry a function of its resource module (`ryo::linter::lint`, `ryo::query::validate`, ...):

| verb | enforces or explores | note |
|---|---|---|
| `query::validate`, `query::describe` | validates RyoQL text; describes the language's grammar | `validate` calls core's `validate` use case; `describe` composes `ryo_query_language`'s use case directly; Tastemaker calls both |
| `query::run` | explore | one query, run as the one rule of core's `lint` use case, answered as its own columns |
| `relation::list`, `relation::describe` | catalogue | Tastemaker calls both |
| `operator::list`, `operator::describe` | catalogue | Tastemaker calls both |
| `linter::lint` | rules in, findings and unread files out; checks the memory budget before it reads | Tastemaker calls it |

Names per verb: request `<Verb>Request`, report `<Verb>Report`, error `<Verb>Error`; the same names in core and SDK; the entry's types are the use case's own, exposed by the resource module. `languages.rs` exposes `Parse`, which is `ryo_language::ParseUseCase` re-exported under the plain name, `Project`, its manifest counterpart, and `Language`, the opaque handle `link!` makes of the two (plain nouns for the outside world). The request and report fields beyond what is stated here are drafted with the tests.

**`ryo::languages`, the surface for language authors** (anyone can add a language easily: plain nouns outside). `Language` is not sealed.

| name | what it is |
|---|---|
| `Parse` | `recognizes`, `skips`, `kind_count`, `kind_names`, `unknown_reasons`, `file_model`, `read_tree`, `complete`, and one method for each syntax relation; implemented by a language's `parse` use case (`ryo_language`'s `ParseUseCase`, re-exported under the plain name) |
| `Project` | `manifests`, `resolve(request) -> Result<_, ProjectError>`, and one method for each manifest relation; implemented by its `project` use case (`ProjectUseCase`) |
| the pair `Parse` and `Project` | what a language implements: `Parse` says which files it reads and builds their trees and answers one `Support<Reader<R>>` for each syntax relation of the catalog; `Project` says which files are its manifests, reads them and answers one `Support<ManifestReader<R>>` for each manifest relation; the generic adapter in core calls both; there is no third trait |
| `ParseRequest` | the source file (path, bytes) and the columns the plans read |
| `ProjectRequest`, `ProjectError` | the manifests of the repository in, an error when the tool that reads them fails; the rows are written by the readers the language answers, and a language without manifests answers `absent(NoManifests)` for each manifest relation |
| the row writer | what a reader writes rows through (`Rows`, and `TableBuilder` for `read_tree`; `SourceFile` on the request side) |
| `contract::parse(&impl Parse)`, `contract::project(&impl Project)`, `contract::conformance(&Language, manifest_dir, &Fixtures)` | the contract every language passes, one entry per trait and one over the linked language and its fixtures, behind the SDK's `fixtures` feature (test-only, H-16) |

Our ten languages implement this same interface in `languages/<lang>`, depending on the SDK like any third party; `crates/language` is internal and the SDK re-exports it. A third-party language is a crate that depends on `idiomatic-sh-ryo-sdk` alone, implements `ryo::languages::Parse` (`recognizes`, `skips`, `kind_count`, `kind_names`, `unknown_reasons`, `file_model`, `read_tree`, `complete` and one reader for each relation of the catalog) and `ryo::languages::Project` (`manifests`, `resolve` and one reader for each manifest relation), and links the two with `ryo::link!`, as `languages/json/src/lib.rs` does; `tm scaffold language` writes such a crate whole:

```rust
pub fn linked() -> ryo::languages::Language {
    use std::sync::Arc;

    ryo::link!(
        domain::language::constants::NAME,
        parse: Arc::new(ParseContract::new(Arc::new(jsonc_parser()))),
        project: Arc::new(ProjectContract::new()),
    )
}
```

A program links languages by handing their `linked()` to an entry, `ryo::linter::Languages(vec![ryo_json::linked()])`; a language's contract tests call `ryo::languages::contract::{parse, project, conformance}` behind the SDK's `fixtures` feature.

**Tastemaker's calls**: `ryo::query::validate`, `ryo::query::describe`, `ryo::relation::{list, describe}`, `ryo::operator::{list, describe}` and `ryo::linter::lint`. Query compilation is cached inside the run; recognition becomes the report's `unread`. The record, capture, catalog, languages-list and corpus calls disappear. An `imports` allow-list in `crates/tastemaker/tastemaker.toml` enforces it, applied at Tastemaker's migration.

**`connections.rs` rule**: in a verb directory (CLI and SDK alike) `connections.rs` is present exactly when that verb's composition uses a client, and absent otherwise; Tastemaker checks both directions. `composition.rs` is pure wiring over the clients the connections give.

## A5. Core's ports and adapters

| port | adapter | note |
|---|---|---|
| `corpus` | `ignore`, `memory` | `ignore` walks the repository; `memory` serves a corpus the caller gives |
| `facts` | `memory`, `spill` | `memory` keeps the rows and the words on plain columns of words, built from the catalog inside the adapter; `spill` keeps the words in a file |
| `lint_store` | `postcard`, `memory`, `discard` | the one file `lint.store` in the cache directory the edge names, for the build it names; `memory` for a run that keeps nothing between processes. Beside the files' rows it keeps, as rows of the same shape, the files the project's lookup read beyond the manifests (a `.cargo/config.toml` above the root, as `../.cargo/config.toml`) and, in `consulted_absent`, the places it looked in and found none of: a warm run stats each, digests it with blake3 when its size or time changed, and reads the project again and makes the rules that read it again only when one is gone or its digest changed; `discard` keeps nothing and reads back as empty, for a run that must not leave a store. `stored` hands back the store as read, a `ReadStore`; its `tables`, `handed` and `keep` are of that store, so a run keeps against the store it read whatever another run kept meanwhile |
| `rows` | `collect` | |
| `scheduler` | `rayon`, `sequential` | |
| `memory` | `sysinfo` | |
| `digest` | `blake3` | |
| `matching` | `globset` | |
| `language` | `use_case` | one generic adapter over `ParseUseCase` and `ProjectUseCase`; reaches every `languages/<lang>` |
| `query_language` | `use_case` | calls query_language's use cases |

Core has no `syntax` port: parsing RyoQL is `crates/query_language`'s, behind its `syntax` port (adapter `winnow`).

## A6. The operator collection

`crates/core/src/domain/operator/collections/operators/<op>/item.rs` with `impl OperatorItem for <Op>Item`, one contract (`fixtures/contract.rs`) over all 17: select, project, extend, join, union, intersection, difference, antijoin, group-count, group-digest, capture, words, rank, axis, rows, closure, cycles. They replace 18 concept directories. The six comparison families (affix, equality, ordering, float_text, membership, glob, as the scaffold names them) are not items of it: they are the `comparisons` collection of the `comparing` concept; the operators are 17.

## A6a. Language crates

Ryo owns an abstracted layer of relations and names no language. A language crate maps its language onto it in three layers, so a better parser library swaps in without touching the rest:

| layer | where | what |
|---|---|---|
| 1. library adapters | `outbound/syntax/adapters/<lib>` | the parser library behind the `syntax` port, swappable; each adapter passes the language's syntax contract |
| 2. the language's model | `domain/syntax/types.rs` (`NodeKind`, the language's one vocabulary of kinds, seeded once from the first parser's grammar; never a directory for each kind), and `scopes`, `declarations`, `references`, `imports`, `bindings`, `literals`, `refinements` | the language's own concepts over its kinds |
| 3. the mapping onto ryo | `domain/relations/collections/relations/<relation>/item.rs`, `domain/manifests/collections/manifests/<relation>/item.rs`, and `inbound/contracts/{parse,project}/contract.rs` (the answers) | the only places that name the catalog (`Relation`, `CatalogRelation`, the `*Row` types, `Rows`, `Reader`, `ManifestReader`), and only through `ryo::languages` |

Tree of one language (`<lang>` = rust, python, typescript, javascript, go, css, html, markdown, toml, json; a programming language has all of it, css, html, markdown, toml and json have `language`, `syntax` without keywords and operators, `unknowns` and `relations`):

```
languages/<lang>/src/
  inbound/contracts/
    parse/        contract.rs, contract_test.rs      implements ryo::languages::Parse (no interface, types or errors)
    project/      contract.rs, contract_test.rs      implements ryo::languages::Project; absent answers where the language has no manifests
    link/         later: count 0 in tastemaker.toml (source dependency, module export, call resolution)
  domain/
    language/     service.rs                         recognition (file extensions)
    syntax/       service.rs, types.rs               NodeKind in types.rs (one file of kinds); the service also moves about a file's tree (there is no `tree` concept: B6 item 2)
    scopes/ declarations/ references/ imports/ bindings/        service.rs, types.rs     (programming languages)
    refinements/  collections/refinements/<name>/{item.rs,constants.rs,item_test.rs}     the words a row carries a language's own distinction as (programming languages)
    manifests/    collections/manifests/<relation>/item.rs   the rows a project's manifests give (`package_dependency`, `workspace_member`), from the `manifest_reading` port
    relations/    service.rs, types.rs               collections/relations/<relation>/{item.rs,item_test.rs}
  outbound/
    syntax/       interface.rs, fixtures/{contract.rs,fake.rs}     adapters/<lib>/{adapter.rs,adapter_test.rs}
    manifest_reading/ the same shape                  interface.rs, types.rs, errors.rs     adapters/<manifest reader>      go (gomod), python (pep621, uv, poetry, pip, pipenv), typescript (npm, pnpm, bun, tsconfig), javascript (npm, pnpm, bun); rust reads Cargo manifests through `manifest` (adapter `toml`) and the repository through `repository` (adapter `native`)
```

Rules, held by the placement and the product's `tastemaker.toml`:
- A library is named only in its adapter (`client-at-adapter`, whose `libraries` lists every parser library). The `imports` of the `languages/*` entry deny everything but `ryo::languages`, `crate`, `std`, `thiserror`, `strum` and the parser libraries; `std::fs`, `std::net`, `std::env` and `std::process` are reached only in `outbound/manifest_reading/adapters/**`.
- `domain` names no library. Layer 2 (the language's model) uses ryo's table primitives only; only layer 3 (`domain/relations`, `domain/manifests`, `domain/language`) names the catalog, and only through `ryo::languages` (the model cannot exist without the table it reads; what it must not know is the catalog it is mapped onto).
- Every relation of the catalog is a method of `ryo::languages::Parse` (syntax relations) or `ryo::languages::Project` (manifest relations), with no default body, generated from the catalog's one list (`each_relation!` in `crates/relations/src/domain/relations/service.rs`): a language answers `ryo::languages::reads::<Item>()` (`reads_manifests::<Item>()` for a manifest relation) or `ryo::languages::absent(reason)` for each, so a relation added to the catalog stops every language compiling until it answers. `docs/language-contract.md` states the moving parts.
- The contract every adapter passes holds the kinds as a match with no wildcard arm in `src/fixtures/kinds.rs`, and the conformance holds each answer to `tests/fixtures/relations/<relation>/{input.<ext>,expected.rs}`, the full typed rows (no table file: `no-tsv`).
- Per-file rows keep a name that does not resolve in the file as written, with a scope id local to the file. Resolution across files is the later `link` use case.
- `project` is called once per run by core's generic `language` adapter, beside `parse`; it reads the manifests and writes `package-dependency` and `workspace-member` rows, which Tastemaker's `crate-dependency-cycles` and `workspace-membership` rules read.
- The contract has no library-swap row yet: it returns when a language has a second adapter.
- Names follow nu-hex: file names are primitives, directories are domain concepts, names carry the kind suffixes (`<Item>Item`, `<Port>Port`, `<UseCase>UseCaseImpl`).

## A7. Naming

| thing | name |
|---|---|
| a use case's trait | `<Dir>UseCase`; its struct `<Dir>UseCaseImpl`; request, report, error `<Dir>Request`, `<Dir>Report`, `<Dir>Error` |
| an outbound port's trait | `<Dir>Port` |
| an exposed port's trait (the exception) | the plain noun (`Language`), so that it is easy for the outside world; the nu-hex param `exposed` lists such ports |
| a collection's trait / an item's struct | `<Dir>Item` / `<Item>Item` |
| an adapter / its settings | `<Port><Dir>Adapter` / `<Port><Dir>Settings` |
| a wire shape / its refusal | `<Name>Schema` / `<Name>SchemaError`, only in `schemas.rs` |
| a fake | `<Port>Fake`, in `outbound/<port>/fixtures/fake.rs` (file unprefixed) |
| a part's error | `<Dir>Error` |
| a CLI clap struct | `<Command>Arguments` in `command.rs`; `schemas.rs` converts it |
| the SDK | published package `idiomatic-sh-ryo-sdk`, `[lib] name = "ryo"`; the SDK may be called Ryo SDK in local code but its library is named as plain `ryo` |

Files and directories are the snake_case of the same stem. The `Interface` suffix is retired. In writing, "golden" is "the expected findings" or "the expected output".
