# outbound/language
STATUS: drafted (28 Nov 2026; not reviewed, not agreed). Bead ryo-fmx3.1.13.23.4.13.

Revisions: MAIN = f4ae3bd802 (release 0.2.0, live). TIP = 2a207464c (perf branch, build 139). Read only through
`git -C /workspaces/idiomatic-public-benchmark show|grep|ls-tree|log|diff`. A quote is one line, two spaces, `@rev:path:line`.
`L/` below is `crates/ryo/crates/language/src/`. Lines that name a `ryo_table` type are UNPROVEN until
`domain/table/spike.md` records a pass. The nine language crates have their own sheet (`languages/`); this sheet is the
contract they implement and the engine reads.

## 1. Owns
The contract a language implements: how a file is recognised, walked into the tree, and its relations filled when the
engine asks, and how a repository's manifests are read; plus the one value a linked language is. One thing. The draft
takes out what is not the contract (section 5): the tree's layout (to the store), the run's words (to symbols), the
spike counters, and the three duplicate per-file hand-offs that sit beside the one the engine calls.

## 2. On main (f4ae3bd802)
- **Where.** The crate `crates/ryo/crates/language` (package `idiomatic-sh-ryo-language`, library `ryo_language`):
  21,648 Rust lines with tests, 7,670 without (tests, `fixtures`, `cases.rs`). One Cargo dependency inside Ryo:
  `ryo_relations`. Beside it, two more homes of the concept: core's `LanguagePort` and `LanguageConnection`
  (`core/src/outbound/language`), and the SDK's `Language` plus the author surface (`sdk/src/resources/languages.rs`,
  375 lines, 131 `pub use ryo_language` and 53 `pub use ryo_relations` lines; `resources/languages/*`, 641 lines;
  macros `link!`, `reads!`, `selects!`).
- **Public items: 280** (279 `pub struct|enum|trait|fn|const|type|static` outside tests and fixtures, plus 1
  `#[macro_export]`). 12 `pub trait` lines, no crate-internal trait.
- **The contract, verbatim.** One trait carries everything a language does per file: 15 methods, one associated type,
  and one generated reader method per baseline relation (39 on main), with the tree built through a builder:
  ```
  pub trait ParseUseCase: Send + Sync {  @f4ae3bd802:crates/ryo/crates/language/src/inbound/use_cases/parse/interface.rs:34
      fn read_tree(  @f4ae3bd802:crates/ryo/crates/language/src/inbound/use_cases/parse/interface.rs:111
      ryo_relations::each_relation!(@declared ryo_relations::baseline_relations, syntax_methods);  @f4ae3bd802:crates/ryo/crates/language/src/inbound/use_cases/parse/interface.rs:121
  pub trait ProjectUseCase: Send + Sync {  @f4ae3bd802:crates/ryo/crates/language/src/inbound/use_cases/project/interface.rs:27
  ```
  A reader is a declared item visited once per element, writing rows one at a time through `dyn`:
  ```
  pub trait ReadItem: declared::Syntax {  @f4ae3bd802:crates/ryo/crates/language/src/domain/answers/types.rs:38
  pub trait Visiting<R: RelationsItem, M>: Sync {  @f4ae3bd802:crates/ryo/crates/language/src/domain/answers/types.rs:64
          out: &mut dyn Rows<<R as RelationsItem>::Row>,  @f4ae3bd802:crates/ryo/crates/language/src/domain/answers/types.rs:72
  ```
  The engine reads a language through the erased twin, a file at a time, rows through `dyn RowWriter`:
  ```
  pub trait ParseEngine: Send + Sync {  @f4ae3bd802:crates/ryo/crates/language/src/inbound/use_cases/parse/types.rs:145
      fn read_file(  @f4ae3bd802:crates/ryo/crates/language/src/inbound/use_cases/parse/types.rs:213
          out: &mut dyn RowWriter,  @f4ae3bd802:crates/ryo/crates/language/src/inbound/use_cases/parse/types.rs:219
  ```
  Rendering is a batch: `fn render_batch(`  `@f4ae3bd802:crates/ryo/crates/language/src/domain/semantics/types.rs:16`
  is on `SemanticSession`. The 12 traits: `declared::{Syntax, Manifest}`, `ReadItem`, `Visiting`, `Semantics`,
  `SemanticSession`, `ExpectedRow` (fixtures), `ParseUseCase`, `ParseEngine`, `ProjectUseCase`, `ManifestItem`,
  `ProjectEngine`.
- **The linked language, two shapes on main.**
  ```
  pub trait LanguagePort: Send + Sync {  @f4ae3bd802:crates/ryo/crates/core/src/outbound/language/interface.rs:17
  pub struct LanguageConnection {  @f4ae3bd802:crates/ryo/crates/core/src/outbound/language/adapters/use_case/connection.rs:8
  pub struct Language {  @f4ae3bd802:crates/ryo/crates/sdk/src/domain/language/types.rs:35
  ```
  `LanguagePort` has 11 methods on main (`relations`, `linked`, `kind_vocabularies`, `unknown_reasons`, `supports`,
  `reads_manifest`, `parse`, `walk`, `begin_semantics`, `project_entries`, `project`).
- **Who implements it.** The nine languages, through the SDK (`ryo::languages::{Parse, Project}` and `reads!`); the
  crate's own fake languages in `fixtures`.
- **Who calls it** (files naming `ryo_language` outside the crate): core 157, execution 100, execution_runtime 41,
  operators 33, execution_domain 26, sdk 17, values 14, cli 12, query_language 2.
- **Proof.** 51 `#[test]` in 43 files; 2 acceptance features with 3 scenarios (parse refuses; parse keeps the flow
  facts; project succeeds); 14 compiler cases (12 `fail_*`, 2 `pass_*`) under `tests/ui/`; an allocation test run
  alone with `dhat`, case `"allocation::language::reading_allocates_as_the_parser_library_does",`
  (`@f4ae3bd802:crates/ryo/crates/language/tests/allocation.rs:16`, on its own line below).
  ```
          "allocation::language::reading_allocates_as_the_parser_library_does",  @f4ae3bd802:crates/ryo/crates/language/tests/allocation.rs:16
  ```
- **Tastemaker entries** (`crates/ryo/tastemaker.toml` at main): 16 `library-surface`, 25 the languages'
  internal-package allowlist (SDK only), 32-33 the crate's own allowlists (relations only), 44
  `no-default-trait-methods`, 61 `collection-members` (item trait `ryo::languages::ReadItem`), 70 `compiler-ui`, 84-88
  the compile-fail, ui-case (min 8), ui-stderr, ui-pass and allocation-test templates, 130 `crate-language`, 191
  `nu-hex-language`.

## 3. On the perf branch (2a207464c)
- **Where.** The same crate: 19,327 lines with tests, 11,259 without. The concept now has **four** homes: this crate,
  core (`LanguagePort` with 9 methods, `LanguageConnection` with a third engine), the SDK (`Language` with an optional
  walk; the author surface: `languages.rs` 402 lines with 139 + 56 re-export lines, `resources/languages/*` 713 lines,
  macros `link!`, `fills!`, `reads!`, `selects!`), and the proof-of-concept's `Linked` behind a 19-method `Source`.
- **Public items: 392** (387 + 5 `#[macro_export]`). **24 `pub trait` lines**, no crate-internal trait.
- **The contract actually called, and the three beside it.** A language now implements a walk over its fillers:
  ```
  pub trait WalkUseCase: Fillers {  @2a207464c:crates/ryo/crates/language/src/inbound/use_cases/walk/interface.rs:25
          builder: &mut NodeTableBuilder<'_>,  @2a207464c:crates/ryo/crates/language/src/inbound/use_cases/walk/interface.rs:44
          visit: &mut dyn FileVisit,  @2a207464c:crates/ryo/crates/language/src/inbound/use_cases/walk/interface.rs:46
  pub trait Fillers: Send + Sync + 'static {  @2a207464c:crates/ryo/crates/language/src/domain/fillers/types.rs:157
              rows: &mut dyn ::ryo_relations::domain::relations::types::Rows<  @2a207464c:crates/ryo/crates/language/src/domain/fillers/types.rs:222
  pub trait FactSource {  @2a207464c:crates/ryo/crates/language/src/domain/facts/types.rs:109
          out: &mut FilledColumns,  @2a207464c:crates/ryo/crates/language/src/domain/facts/types.rs:118
  ```
  But the engine never calls `walk` (nor `WalkEngine::walk_file` or `buffers`: no caller outside doctests and the
  python bench). It reads every language through a second hand-off, the numbered visit, reached through a third
  trait, `ParseUseCase`, whose 11 methods are mostly the walk's again:
  ```
  pub trait ParseUseCase: Send + Sync {  @2a207464c:crates/ryo/crates/language/src/inbound/use_cases/parse/interface.rs:11
      fn file_relations(&self) -> &dyn super::named::FileRelations;  @2a207464c:crates/ryo/crates/language/src/inbound/use_cases/parse/interface.rs:17
  pub trait FileRelations: Send + Sync {  @2a207464c:crates/ryo/crates/language/src/inbound/use_cases/parse/named.rs:99
  pub trait NumberedVisit: Send + Sync {  @2a207464c:crates/ryo/crates/language/src/inbound/use_cases/parse/named.rs:139
      fn open(&self) -> Box<dyn NumberedReader + '_>;  @2a207464c:crates/ryo/crates/language/src/inbound/use_cases/parse/named.rs:151
  pub trait NumberedReader {  @2a207464c:crates/ryo/crates/language/src/inbound/use_cases/parse/named.rs:155
  pub trait NumberedFacts {  @2a207464c:crates/ryo/crates/language/src/inbound/use_cases/parse/named.rs:74
  ```
  The one adapter from the first to the second, which every language builds inside its `Parse` contract:
  ```
  pub struct NumberedWalk<W: WalkUseCase> {  @2a207464c:crates/ryo/crates/language/src/inbound/use_cases/walk/numbered.rs:46
          self.facts.fill(relation, wanted, words, columns)  @2a207464c:crates/ryo/crates/language/src/inbound/use_cases/walk/numbered.rs:204
  ```
  So one fill crosses `NumberedFacts::fill_columns` (dyn), then `FactSource::fill` (dyn), then the generated
  `Fillers` function (static), then `Rows::write` **once per row through `dyn`** into `ColumnsWriter`, which pushes
  each cell (`impl<R: RelationsItem> Rows<R::Row> for ColumnsWriter<'_, R> {`
  `@2a207464c:crates/ryo/crates/language/src/domain/fillers/types.rs:282`).
- **Who implements it.** Each of the nine languages implements `Parse` (`ParseUseCase`), `FileRelations` (on the same
  struct: `fn file_relations(&self) -> &dyn languages::FileRelations { self }` in all nine), `Walk` with `Fillers`
  (through `fills!`), and `Project`; and builds its walk twice, once inside `Parse` and once for `link!`:
  ```
      numbered: languages::NumberedWalk<crate::domain::file_view::types::GoWalk<crate::inbound::contracts::walk::contract::WalkPorts>>,  @2a207464c:crates/ryo/languages/go/src/inbound/contracts/parse/contract.rs:20
          walk: Arc::new(GoWalk { ports: WalkPorts { syntax: Arc::new(tree_sitter_go()) } }),  @2a207464c:crates/ryo/languages/go/src/lib.rs:40
  ```
- **Who calls it.** The executor, once per file (`reader.visit(`  `@2a207464c:crates/ryo/crates/executor/src/check_session.rs:1621`),
  and once per relation on first read, always whole:
  ```
                  supplier.fill_columns(relation, l::Wanted::Whole, words, columns)  @2a207464c:crates/ryo/crates/executor/src/reader_run/packet.rs:210
              lazy.supplier.borrow_mut().fill_columns(relation, l::Wanted::Whole, words, columns)  @2a207464c:crates/ryo/crates/executor/src/reader_run/packet.rs:251
  ```
  The metadata it reads through `Linked`, which holds the three engines and asks the walk only for expansions and
  absences:
  ```
  pub struct Linked {  @2a207464c:crates/ryo/crates/executor/src/check_session/linked.rs:14
      fn scoped(&self) -> &dyn l::NumberedVisit { self.parser.file_relations().numbered() }  @2a207464c:crates/ryo/crates/executor/src/check_session/linked.rs:39
  pub trait Source: Clone + Sync {  @2a207464c:crates/ryo/crates/executor/src/check_session/source.rs:10
  ```
  Files naming `ryo_language` outside the crate: core 49, sdk 13, executor 11, values 10, cli 8, execution_domain 5,
  query_language 2, execution 1. The three shapes of one linked language:
  ```
  pub struct LanguageConnection {  @2a207464c:crates/ryo/crates/core/src/outbound/language/adapters/use_case/connection.rs:8
  pub struct Language {  @2a207464c:crates/ryo/crates/sdk/src/domain/language/types.rs:36
  pub trait LanguagePort: Send + Sync {  @2a207464c:crates/ryo/crates/core/src/outbound/language/interface.rs:15
  ```
- **Proof.** 39 `#[test]` in 28 files; 3 features with 11 scenarios (walk 8: lends a file, keyed fills, a fixed word
  is one cell in every file, growth as declared, the growth harness rejects a weaker filler, walk refuses; parse 2;
  project 1); 3 compiler cases (`fail_manifest_reader_writes_another_relation`, `fail_project_missing_a_manifest_relation`,
  `fail_relation_defined_by_a_language`), no `pass_*`; **no allocation test**. The conformance kit (routes against
  frozen rows, the growth harness) is re-exported by the SDK (`languages::contract`) and run by each language.
- **Tastemaker entries** (at the tip): 16, 22 (languages may depend on the SDK only), 29-30 (relations only), 42
  `no-default-trait-methods`, 59 `collection-members` (still `ryo::languages::ReadItem`), 67, 79-83 (ui-case still
  `count = { min = 8 }`, allocation-test still names `crates/language/tests/allocation.rs`), 120 `crate-language`, 181
  `nu-hex-language`.

## 4. What changed between them
`git diff --stat f4ae3bd802 2a207464c -- crates/ryo/crates/language`: 162 files, +8,734 / -11,056; 90 commits.

**Added**
| What | Commit | Why, from the commit |
|---|---|---|
| `WalkUseCase`, `FactSource`, the engine crate beside the old engine | 3097109506 (11-20) | "the engine contract beside today's engine (walk, facts, engine crate, RYO_ENGINE)" |
| `WalkEngine`, the object-safe handle | d7d834affd (11-20) | "a language links its walk behind an object-safe handle" |
| keyed fills that append, declared sizes and keys, the run's names (`Wanted`, `KeyedFillItem`, `WalkVocabulary`) | 8abefc84de (11-20) | "the facts contract amendment" |
| `NodeTable` written in its final layout as the walk writes it | b62faa8801, b6bf0b4fee, 4b2fdf2fae (11-22) | perf rows 33, 39 |
| the numbered visit (`NumberedVisit`, `NumberedReader`, `NumberedFacts`, `FileRelations`) | 6a774e407c and the 11-24 series | "Run a provider's scoped numbered file in the common file visit" |
| `NumberedWalk`, "the one adapter from a walk to the numbered visit" | cdcb0b5431 (11-25) | perf row 5 |
| the metadata of `ParseEngine` copied onto the walk | 7dda183b42 (11-25) | "has its destination on the walk contract" |
| fill on first use | 4b6cf1c93b (11-25) | perf row 12 |
| per-file project read (`file_entries`, `file_rows`) | 4c99a1e13b (11-26) | "no behaviour changes yet" |

**Removed.** dbad3ac7f2 (11-26, "the final contract: a language has no old reader"): 105 files, -12,491 lines, among
them `read_tree`, `complete`, the 39 reader methods, the old conformance tests, **9 of the 12 `fail_*` cases, both
`pass_*` cases and `tests/allocation.rs`**. 9b9ac52d65 (11-26): core's `LanguagePort` loses `parse`, `walk`,
`begin_semantics`. 070d4a1275 (11-27): 21 "never-run" functions, 13 files in this crate (`kind_index` tests, `semantics`
tests, symbol transport).

**Lost that should not have been.**
- The compiler cases that held the contract's shape: `fail_parse_missing_a_relation` (a relation added to the catalog
  stops every language until it answers: the tip's `Fillers` promises the same and nothing tests it),
  `fail_reader_writes_another_relation`, `fail_builder_reaches_the_table`, `pass_every_relation_answered`. The
  Tastemaker template still asks for at least 8 `fail_*` cases and the allocation test (tip lines 80 and 83); whether
  `tm check` passes at the tip is not checked.
- The allocation row (`reading_allocates_as_the_parser_library_does`): the one test that held a language's read to its
  parser library's allocations.
- Main's batch render: the tip asks one node at a time (`fn render(&mut self, request: RenderRequest) -> RenderOutcome;`
  `@2a207464c:crates/ryo/crates/language/src/domain/facts/types.rs:121`), and the executor calls it per row
  (`finish.rs:2359`).
- A parser failure is now reported as "does not read this file": the adapter maps every walk error to `Unsupported`
  (`.map_err(|_| ParseError::Unsupported)`  `@2a207464c:crates/ryo/crates/language/src/inbound/use_cases/walk/numbered.rs:141`).

## 5. The drafted public interface
The listing is `outbound/language/api.txt` beside this sheet: **344 lines** (one item, trait member or variant each;
15 traits), against 280 public items on main and 392 at the tip (counted without members or variants, so the sides
are not like for like). 201 lines are the contract, by tag 66 `[main]`, 65 `[perf]`, 70 `[changed]`, 0 `[new]`; 143
are the author library listed by name and kept as it is (all `[main]`). Nothing in it is agreed.

**The claim it starts from.** Appendix A 3.11 says "exists; two changes" (`WalkUseCase::walk` takes `FactDemand` and
`impl TreeWrite`; `FactSource::fill` takes `dyn TableFill`), keeps `ParseUseCase` (11 methods), core's
`LanguageConnection` with three engines and `LanguagePort` (9), and leaves the other 20 traits "as they are". Checked:
the counts hold (11, 9, 24). What does not hold: (1) the engine does not call `walk`; it calls the numbered visit,
which the claim does not list; (2) `ParseUseCase` is not a second contract, it is a copy: of its 11 methods,
`recognizes`/`recognizes_explicit`, `skips`, `configuration_identity`, `capabilities`, `unknown_reasons`,
`source_words` and `kind_count` are on `WalkUseCase` already, `kind_names` and `stable_fields` are `Fillers::vocabulary`,
and `file_relations` only hands the engine the same walk wrapped; (3) "two changes" leaves three `dyn` crossings and a
per-row `dyn` on the fill. The draft therefore changes more, in three moves; the one capability it adds is that a fill
receives what the plan already decides (`Demand`, 5d).

### 5a. One per-file contract (decision 1)
A language implements `WalkUseCase: Fillers` and `ProjectUseCase`, and nothing else. `ParseUseCase` (11),
`FileRelations` (4), `NumberedVisit` (3), `NumberedFacts` (5) and `ParseEngine` (10) are **replaced**, never dropped
for lack of callers: every method has a line in api.txt's NO LONGER PUBLIC naming the method that answers it now.
- `WalkUseCase` keeps its 10 methods and gains no new one: `named_declarations` replaces `FileRelations::declarations`,
  and `uses_project` becomes the one statement. Today it is on `WalkUseCase` and on `FileRelations`
  (`fn uses_project(&self) -> bool { false }`  `@2a207464c:crates/ryo/crates/language/src/inbound/use_cases/parse/named.rs:101`),
  the executor reads the second, and **the two disagree for Python**: its `FileRelations` answers
  `fn uses_project(&self) -> bool { true }`  `@2a207464c:crates/ryo/languages/python/src/inbound/contracts/parse/contract.rs:82`
  while no language overrides the walk's default `false`. The merged answer must be Python's `true` (row 4 depends on
  it). Its four default bodies go (the Tastemaker rule at tip line 42 already forbids them).
- `WalkEngine`, the erased twin, absorbs `ParseEngine`'s metadata and `NumberedVisit`: `projections()` and
  `open() -> Box<dyn NumberedReader>`. `buffers()` and `walk_file()` go: no engine code calls them, and `open` is the
  per-worker state they duplicate.
- `NumberedReader::visit` stays the per-file call, with `WalkRequest` in place of `ParseRequest` (one request type:
  `WalkRequest` gains `notes` and `fields`; `explicit(&mut self, bool)` becomes the request's own `explicit`), and
  `WalkError` in place of `ParseError` (one error; `Library` gains main's `reason`, so the adapter stops answering a
  parser failure as `Unsupported`).
- `FactSource` is what the engine holds for the life of the visit; `NumberedFacts` goes (it only forwarded), and with
  it `project_structure` (no caller: the executor reads `semantic-syntax` and `semantic-child` from the tree, as
  `projections()` declares).
- The named relations (`FileRelation`, `FileRows`, `NamedRequest`, `Fillers::named`) are kept although none of the nine
  declares one (`fn declarations(&self) -> &'static [languages::FileRelation] {` answers `&[]` in all nine): a designed
  contract is reported, not dropped. `FileRows.columns: Vec<FactColumn>` is layout and changes to a `TableFill` sink;
  its exact shape is not drafted (section 11).

### 5b. One linked language (decision 1)
`LanguageConnection` moves here from core and becomes the only shape: it replaces the SDK's `Language`
(`walk: Option<Arc<dyn WalkEngine>>,`  `@2a207464c:crates/ryo/crates/sdk/src/domain/language/types.rs:46`),
core's struct with three engines
(`pub walk: std::sync::Arc<dyn ryo_language::inbound::use_cases::walk::types::WalkEngine>,`
`@2a207464c:crates/ryo/crates/core/src/outbound/language/adapters/use_case/connection.rs:20`), and the executor's
`Linked` with its `Source` (`impl Source for Linked {`  `@2a207464c:crates/ryo/crates/executor/src/check_session/linked.rs:36`).
Each of `Source`'s 19 methods maps to one call of `WalkEngine` or `ProjectEngine` (api.txt NO LONGER PUBLIC). It holds
two engines, not three (`parse` folds into `walk`); its fields are private and only `link!` makes one, as the SDK's
`Language` is today. `LanguageName` and `LanguageBuild` are defined once here (today in the SDK, again in core
(`LanguageBuild`) and in `values::descriptions` (`LanguageName`)). `LanguagePort` moves here from core (application/project's
draft already names `ryo_language::LanguagePort`) with `walks()` changed to `connections()`; `Languages`, the SDK's list
type, implements it and replaces core's `LanguageUseCaseAdapter`.

### 5c. The author surface moves here
`link!`, `fills!`, `reads!` (and the statements of `reads!`), the catalog names the SDK writes for each relation, and the
conformance kit (`languages::contract`) move from the SDK to this crate; each language then imports this crate alone
(owner: languages import the language contract, not the SDK). `link!` keeps one arm: `link!(name, walk: .., project: ..)`.
`reads!` keeps its manifests arm (its syntax arm has no user at the tip). `kinds!`, `Bytes`, `NameNote` arrive from
`relations` (the relations sheet's decision 5); `Words` goes to the store, not here.

### 5d. Changed lines on the hot path (decision 4, decision 5)
- `Fillers::<relation>`, `FillItem::fill`, `KeyedFillItem::fill_key`: `rows: &mut dyn Rows<Row>` becomes
  `rows: &mut impl Rows<Row>`, so the writer is fixed at compile time and a row costs no `dyn` call. `Fillers` is never
  used as a trait object, so the generic costs nothing elsewhere.
- `FactSource::fill(relation, wanted: &ryo_operators::Demand, words: &mut ryo_symbols::SymbolLayer, out: &mut dyn ryo_table::TableFill) -> FactAnswer`.
  `out` was `&mut FilledColumns` (`pub struct FilledColumns(pub Vec<FilledColumn>);`
  `@2a207464c:crates/ryo/crates/language/src/domain/facts/types.rs:265`, a column layout); the language's writer
  gathers each column in its own reused buffer and hands it over with one `TableFill::column` call. That adds one copy
  per column against today's lent buffers: UNPROVEN, the table spike measures it. `wanted` was the language's own
  `Wanted { Whole, Key(Column, CellWord) }` (`pub enum Wanted {`  `@2a207464c:crates/ryo/crates/language/src/domain/facts/types.rs:354`);
  it becomes the plan's `Demand` (`Columns`, `Exists`, `Key`), so the columns a rule reads reach the language
  (row 3) and one type says what is wanted (decision 4). `words` follows the symbols sheet's decision 3; if that is
  refused, `FileWords` stays here and the line reads `&mut FileWords`.
- `FactSource::render` becomes a batch over the operators' `RenderRequests`, as main's `render_batch` was.
- `WalkUseCase::walk(.., tree: &mut ryo_store::Tree, .., visit: &mut dyn FileVisit)`, `NumberedReader::visit(.., tree: &mut ryo_store::Tree, ..)`,
  `FileFacts.tree`, `NumberedFile.tree`, `FactSource::notes(.., &ryo_store::Tree, ..)`: the tree is the store's type,
  written through `ryo_table::TreeWrite`, read through `TreeRead`, handed in by the caller (decision 2). Today the
  language owns it and names its layout (`builder: &mut NodeTableBuilder<'_>` above, and
  `let writer = &mut builder(tree, W::KIND_COUNT);`  `@2a207464c:crates/ryo/crates/language/src/inbound/use_cases/walk/numbered.rs:132`).

### 5e. One statement of keys (decision 3)
Three statements today (the relations sheet's finding): `is_key`'s list in relations, the item's `keys [..]`
(`const KEYS: &'static [Column];`  `@2a207464c:crates/ryo/crates/relations/src/domain/relations/collections/relations/interface.rs:60`),
and the language's own:
```
    const KEYS: &'static [Column];  @2a207464c:crates/ryo/crates/language/src/domain/fillers/types.rs:82
/// assert_eq!(<TokenLinkFill as KeyedFillItem>::KEYS, key_columns(Relation::TokenLink));  @2a207464c:crates/ryo/crates/language/src/domain/fillers/types.rs:78
            const KEYS: &'static [$crate::languages::Column] = &[ $( $column ),+ ];  @2a207464c:crates/ryo/crates/sdk/src/resources/languages/macros/fills.rs:62
        Wanted::Key(column, key) if I::KEYS.contains(&column) => {  @2a207464c:crates/ryo/crates/language/src/domain/fillers/service.rs:93
```
The draft settles it: **a language reads keys from the catalog item, `<I::Relation as RelationsItem>::KEYS`, a constant
known when the language compiles; `KeyedFillItem::KEYS` goes**. `filled_keyed` tests a wanted key against the catalog's
list; `fills!` checks each key clause's column against the same list at compile time (a const assertion in the
expansion) instead of writing a third copy. Today the 14 key clauses (all in python, for 13 relations) equal the catalog's
13 declarations exactly (for example
`key(languages::Column::TokenLink(languages::TokenLinkColumn::Source)) |file, words, key, rows| {`
`@2a207464c:crates/ryo/languages/python/src/inbound/contracts/parse/readers/token_link/composition.rs:71` against
`keys [Source, Target],`  `@2a207464c:crates/ryo/crates/relations/src/domain/relations/collections/relations/token_link/item.rs:30`),
so nothing changes in what any language fills. Two facts the owner should see: no engine code asks a key at the tip
(both fills above ask `Wanted::Whole`), and go, rust and typescript fill `function`, `inherits`, `reference`, `resolves`,
`scope` with no key clause although the catalog declares keys there, which `FillExtent::Whole`'s doc calls illegal
("Legal only for a column the relation's catalog item does not declare among its keys"). The draft keeps today's
behaviour (a filler without key bodies answers the whole relation and says so) and leaves the tightening to decision 3.

### 5f. What stops being public, and the counts
| Side | Public | Traits |
|---|---|---|
| main | 280 | 12 |
| tip | 392 | 24 |
| drafted | 344 lines (contract 201, library 143) | 15 (14 of the tip's 24, and core's `LanguagePort`) |

Leaving the crate: `node_table` (54 items) to the table and store sheets (`TreeRead`/`TreeWrite` name 15 of them);
the run's words in `facts` (11 items: `BaseWords`, `FileWords`, `intern`, `text`, ...) to symbols (its decision 3);
`CellWord` to the table; `Wanted` replaced by the plan's `Demand`; `FilledColumns`/`FilledColumn` (layout) replaced by
`TableFill`. Private: the spike counters (`fill_begun`, `fill_counted`, `FillCounts`, ...), whose own doc says so:
```
/// SPIKE (2026-11-21, never a contract): what the fills of one relation came to in a run counted under `RYO_X_FILLS`.  @2a207464c:crates/ryo/crates/language/src/domain/fillers/service.rs:101
```
and 61 library items no language and no engine file names (by a word grep; list in api.txt). Replaced: the traits of 5a, `ReadItem`,
`Visiting`, `Semantics`, `SemanticAnswers`, `Reader`, `declared::Syntax` (the per-element readers, which no language
implements at the tip; the fill contract answers them), `SemanticSession` (batch render on `FactSource`).

**How it is built.** `link!` in each language builds a `LanguageConnection` from its `WalkUseCase` (wrapped once in
`NumberedWalk`) and its `ProjectUseCase`; an edge collects them into `Languages` and hands them in. No other
constructor; nothing global.

## 6. Proof it brings
| Proof | Source | Fate |
|---|---|---|
| walk feature, 8 scenarios (lends a file, keyed fills, fixed word, growth, harness rejects, refuses) | `L/inbound/use_cases/walk/acceptance.feature` at the tip | carried; steps naming `NodeTable`/`FileWords`/`FilledColumns` restated against the store's tree, `SymbolLayer`, a `TableFill` double |
| parse feature, 2 scenarios | `L/inbound/use_cases/parse/acceptance.feature` | restated against `WalkUseCase` (the same facts: refuses; says what the language is) |
| project feature, 1 scenario | `L/inbound/use_cases/project/acceptance.feature` | carried |
| routes against frozen rows (4) and frozen (3) | `L/inbound/use_cases/parse/fixtures/{routes,frozen}.rs` | carried; it is the "one law for every language" each language runs |
| `only_a_leaf_has_text_whatever_the_kind` | `L/inbound/use_cases/walk/numbered.rs` | carried, restated on the store's tree |
| facts, fillers, fixed_names service tests (6), walk use case (3), project use case (2) | the tip | carried; the facts words tests go with symbols if its decision 3 is taken |
| `node_table/service_test.rs` (1) | the tip | to the store (its sheet) |
| the 3 compiler cases at the tip | `tests/ui/` | carried |
| `fail_parse_missing_a_relation`, `fail_reader_writes_another_relation`, `fail_builder_reaches_the_table`, `pass_every_relation_answered` | main `tests/ui/` | **restored and restated**: `fail_fillers_missing_a_relation`, `fail_filler_writes_another_relation`, `fail_walk_reaches_the_tree_layout`, `pass_every_filled_relation_answered`. Without them the Tastemaker minimum of 8 fails |
| the allocation row | `"allocation::language::reading_allocates_as_the_parser_library_does",`  `@f4ae3bd802:crates/ryo/crates/language/tests/allocation.rs:16` | **restored**, restated on the walk: a walk into the store's tree allocates as the parser library does |
| a key clause on an undeclared column fails to compile | — | **MISSING** (new with decision 3) |
| a parser failure is answered `Library`, not `Unsupported` | — | **MISSING** (the bug at `numbered.rs:141`) |
| a fill under `Demand::Columns` writes only those columns; under `Exists` at most one row | — | **MISSING** (new with decision 4) |
| the growth harness runs for all nine languages | — | not checked (section 11) |

## 7. Perf gains it carries
Every row of `perf-gains.tsv` whose crate column names language (10 rows; rows 3, 31, 33, 45 name `languages/python`
and are listed because the brief asks for every row naming language):
| Row | Gain (measured) | What in the draft keeps it |
|---|---|---|
| 3 | general relations instead of Python-only ones (a regression: fastapi +12%, dagster +16 to +27%) | `FactSource::fill` takes the plan's `Demand`, so `Columns` (decided at load, `PhysicalPlan::demands`) reaches the language and its writer keeps only those columns. This is room the tip lacks (`Wanted` has no column set); it does not by itself remove the cost of computing the unread fields |
| 4 | project environment only for languages that use it (+25% CPU, +70 MiB without) | `WalkEngine::uses_project()`, declared once at link time, read before any file; the duplicate on `FileRelations` goes |
| 5 | one numbered column visit for all nine (Go 15.78 -> 14.40, hono 1.38 -> 1.29) | `NumberedReader::visit` once per file, one hand-off shape for all nine; columns cross through `TableFill::column`, never a row writer across the boundary |
| 12 | fill on first use (not measured) | `NumberedFile.facts: &mut dyn FactSource` lent for the whole visit; `fill` called per relation when a step first reads it; `fills_are_complete` kept |
| 24 | one provider call per file (dagster 13.18 -> 12.33) | one `visit` per file; every relation filled from the same lent view (`Fillers::File`) |
| 31 | binding rules and import sorting read provider facts (nine rules 1.82 -> 0.87 s) | `Fillers` has one function per `Filled` relation of the catalog, generated (`each_relation!`): a fact the language holds is a relation it fills; nothing language-specific in the contract |
| 33 | child ids placed by parent; model and fact columns deferred (first half 1.83 -> 0.70) | `walk` writes the tree in one pass through `TreeWrite` (`enter_node`/`token`/`leave_node`); fact and model columns are built only inside a fill (the per-file view's model "built on first need") |
| 39 | native reads as columns; text cells to symbol ids through a word cache (Django 16.29 -> 13.93) | whole columns, one `TableFill::column` call each; texts interned into the run's one `SymbolLayer` (symbols decision 3), which removes the per-file translation the cache exists for |
| 44 | facts as columns; one parse per file (Django 32.46 -> 28.08) | one `visit` per file holds the parse; `fill` writes columns |
| 45 | 21 rules as per-candidate checks (19.93 -> 15.00) | nothing here forces a whole-file form; the contract is neutral to rule text |

Room for gains not yet had: per-row `dyn` removed from the fill (5d) is a cost the tip pays on every row filled
(not measured); `Demand::Exists` lets a filler stop at the first row (the tip's executor has `fill_len` for this,
outside the contract).

## 8. Benchmarks that cover it
| Benchmark | What it names here today | After the draft |
|---|---|---|
| python `fillers` (tip only; per relation `ns-per-element`, `ns-per-element-4n`, `growth-percent`; held by Asymptote) | `    self, CellWord, Column, Count, FactAnswer, FillExtent, FilledColumns, NodeTable, Relation,` (line 13) and `Walk, Wanted` (line 14) of `crates/ryo/languages/python/benchmarks/provider/performance/fillers/bench.rs`, and `ryo_python::walk()` | same measures and names; `NodeTable` -> the store's tree, `FilledColumns` -> a `TableFill` that collects, `Wanted` -> `Demand`, `CellWord` from the table. Its walk fixture is the only caller of a language's `walk()` |
| python `manifest_reading` (main and tip) | `Project` through the SDK | unchanged |
| the provider bench of each language (`benchmarks/provider/bench.rs`: nine on main, go only at the tip), typescript `embedded` and `manifest_reading` (main only) | the SDK's `Parse` | the languages sheet's to restore or retire; each would call `WalkEngine` |
| the lint images (`vec`, `vec_ts`, `vec_rust`, `tm`) | the SDK only | untouched; they are the measure of rows 3 to 44 |
| the allocation test (main only, a test not a bench) | `tests/allocation.rs` | restored (section 6) |
The bars this crate answers for: the fill share of CPU (13% on the latest split, plan 10.4) and the per-file parse; no
bar cell of its own.

## 9. Checks
IMPORTS: table, symbols, relations, operators, store
CROSSED: per file
- **Imports.** Today `relations` only (Tastemaker tip line 29). Drafted: `table` (`TableFill`, `TreeRead`/`TreeWrite`,
  `CellWord`, `Count`, `KindBits`), `symbols` (`SymbolLayer`), `relations` (the catalog), `operators` (`Demand`,
  `RenderRequests`, `RenderOutcomes`: decision 4 would move these to `relations` and drop this import), `store` (its
  tree type: decision 2; outbound may import outbound; the store sheet is not written yet). The languages' allowlist
  (tip line 22, SDK only) gets a new version naming `idiomatic-sh-ryo-language` only.
- **No layout word in a signature.** Removed: `NodeTableBuilder`, `NodeTable`, `FilledColumns`/`FilledColumn`,
  `FileRows.columns` (`pub columns: Vec<FactColumn>,`  `@2a207464c:crates/ryo/crates/language/src/inbound/use_cases/parse/named.rs:47`),
  the `node_table` readers handing out `&[u16]`. Kept and logical: `Relation`,
  `Column`, `CellWord`, the store's tree as an opaque type.
- **How often it is crossed.** Per request: the metadata (`recognizes`, `skips`, `vocabulary`, `absence_of`, ...),
  once at load. Per worker: `open`. **Per file: `NumberedReader::visit`, one `dyn` call**; per relation within a file
  (a set): `FactSource::fill`, one `dyn` call, and one `TableFill::column` call per column. Per row: nothing crosses;
  rows are written inside the language through a generic writer. The `dyn` at the file and at the fill is forced by
  nine languages held in one list at run time; decision 5 asks the owner to accept it.
- **The project path** (`ProjectEngine::project`, `file_rows`, `ManifestReader`) still writes rows through
  `dyn RowWriter`, per request and per file of a language that uses the project; not on the per-file hot path of
  the nine (only python, go, rust, typescript have project relations), and left as it is: not measured.
- **Default methods.** The rule at tip line 42 forbids them in this crate; the tip has at least 13 in the traits a
  language implements or the engine reads. The draft has none.

## 10. Decisions for the owner
**1. One per-file contract and one linked language.**
Example: go builds its parser twice and its walk twice (`contract.rs:20` and `lib.rs:40`); the engine reads it through
`Parse::file_relations().numbered().open()` while `Walk::walk` is never called; the same language is three structs
(`Language`, core's `LanguageConnection`, `Linked`) and its metadata is answered twice (`ParseUseCase` and `WalkUseCase`).
Why: "a single contract, never two"; the copies drift (the two `uses_project`, the error mapping at `numbered.rs:141`).
What we get: a language implements two traits (`Walk`, `Project`) and links with one call; the crate's 24 traits become 14, plus core's `LanguagePort` moved in; every
replaced method has a named successor. **My pick:** yes, as 5a and 5b; keep the named-relation mechanism although no
language uses it, until you say it goes.

**2. Where the tree's type is fixed.**
Example: `walk(.., builder: &mut NodeTableBuilder<'_>, ..)` names the language's own layout; each per-worker reader owns a
`NodeTable`. Options: (a) every engine-facing trait generic over `T: TreeRead + TreeWrite`, so `LanguageConnection<T>`
and each language's `linked<T>()`; (b) the store's concrete, opaque tree type (`ryo_store::Tree`) named here, written
through `TreeWrite`, handed in by the caller (outbound may import outbound); (c) the tree stays in this crate, as at the
tip, and implements the table's traits. Why: the layout belongs to the store, and the hot reads must stay static.
What we get with (b): no generics through nine languages and the edges, no `dyn` per node, one layout in one crate;
the cost is that a language compiles against the store. **My pick:** (b). It depends on the table sheet's open question
whether the tree is the table crate's second concept.

**3. Keys: one statement, read from the catalog.**
Example: the doctest at `fillers/types.rs:78` is all that holds the language's `KEYS` equal to the catalog's; no engine
code asks a key (`packet.rs:210, 251`); go, rust and typescript fill five keyed relations without key bodies. Why: one
definition per concept, and the relations sheet asks it. What we get: `KeyedFillItem::KEYS` goes, `fills!` checks its
clauses against `RelationsItem::KEYS` at compile time, the relations sheet derives `is_key` from the same declarations.
**My pick:** yes; and decide separately whether a language that fills a keyed relation must have its key bodies (today
three do not, and nothing asks for keys); I would not require them until a plan asks for a key.

**4. One "what is wanted" type, and where it lives.**
Example: the language has `Wanted { Whole, Key(Column, CellWord) }`; the operators draft has `Demand { Columns, Exists,
Key }` "changed: the tip's language `Wanted::Key`"; the language's `RenderRequest`/`RenderOutcome` and the operators'
are two definitions of one thing. Why: one definition per concept; the language must name the plan's type to receive
the columns a rule reads (row 3). What we get: the plan's demand reaches the fill unchanged. **My pick:** the language
names the operators' types now (no redefinition), and the owner moves `Demand`, `ColumnSet` and the render types down
to `relations` (they speak only of relations, columns, keys and nodes), so the language contract does not import
`operators`.

**5. The two `dyn` crossings that remain.**
Example: today a fill crosses three `dyn` calls and one more per row (`fillers/types.rs:222`). The draft keeps one
`dyn` per file (`visit`) and one per relation fill (`FactSource::fill`), and one `TableFill::column` per column. Why: the
rule says per file or per set, with the type fixed at compile time; nine languages in one run-time list cannot be
fixed at compile time without the edge enumerating them as a closed type. What we get: no per-row or per-cell `dyn`.
**My pick:** accept the two; measure the column copy in the table spike before the build.

## 11. Not checked
- `kind_names` (from `ParseUseCase`) equal to `Fillers::vocabulary().kinds` in all nine languages: assumed from go's
  code (both from `kinds()`), not compared. Same for `stable_fields` against `vocabulary().fields` (no language
  overrides `stable_fields`, and `NumberedWalk::fields` always answers `Some`, so the executor never reaches it).
- Whether the growth harness (`grown`) runs for every language, and so whether the key rule of 5e is held anywhere for
  go, rust and typescript.
- The shape of `FileRows` with a `TableFill` sink (no user to draft it against).
- Whether `TreeWrite` needs a "finished" call (today `node_table::service::finished`) before the tree is lent; the table
  sheet lists it as open.
- The cost of one copy per column in `TableFill::column` against today's lent `FilledColumns` buffers (the executor
  keeps spare buffers per relation, `packet.rs:231`): not measured.
- The author library (elements, builders, keys, shapes, flow, flow_table, trees, names, notes, decoded, targets, words,
  files, expansions, diagnostics, completion, project_context, per-file symbols): two trees live there (`Elements`, the
  old element table, and `NodeTable`); which modules still serve the walk and which only the deleted readers was not
  sorted. The 143 kept and 61 private names come from a word grep over `languages/*/src` and the engine crates, which
  over-counts use (common words) and may mis-mark a type named in a kept signature as private.
- Cross-sheet: the relations listing moves `Symbol`, `NameText`, `NO_SYMBOL` to `ryo_symbols`, the symbols listing says
  it does not touch them and uses `ryo_symbols::Symbol` for the run's number; this sheet writes the per-file `Symbols`,
  `Symbol`, `NameText`, `ByteSlice` where they are today until the two agree. The relations listing also moves `Bytes`
  here while application/execute writes `ryo_relations::Bytes`.
- `Expansions` (8 word hits in engine code) and the six other types of core's `language/types.rs` (`ProjectNotes`,
  `NoteWriting`, `SourceSelection`, `FileToRead`, `FileReading`, `RoleWriting`), left from main's port methods: not read.
- Whether `tm check` passes at the tip given the ui-case minimum, the allocation-test template, the collection-members
  item trait and the default-method rule (all four contradict the tip's files).
- Build 133: not read for this crate. Per-language losses (tests 840 -> 500 across the nine) are the languages sheet's.
- No `tm check` on a skeleton of the draft; no compile of the generic `Rows` change.
