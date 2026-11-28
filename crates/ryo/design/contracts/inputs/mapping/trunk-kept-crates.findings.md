# Plan: write trunk-kept-crates.md (mapping job, bead ryo-fmx3.1.13.23.3)

Status: research about 80 percent done, read-only through git at T=2a207464c. The output file
`/workspaces/ryo-poc-evidence/next/build/ports/stage2/mapping/trunk-kept-crates.md` is NOT written yet, because plan mode
blocks every write except this file. On approval I write that one file from the findings below.

Line convention: Rust lines without tests/fixtures/cases, as a directory plus its sibling `<dir>.rs`. The brief's
numbers drop the sibling file (plan_lowering 2927 here is 2951).

## Findings so far (evidence = path under crates/ryo/)

### query_language (14,184)
- The crate has only two use cases: `typecheck` (538) and `describe` (115). The validate use case is NOT here: it is
  `ryo_core::inbound::use_cases::validate::ValidateUseCaseImpl` (core is outside my scope). It calls
  `TypecheckUseCase` and the language port. So `inbound/validate` takes core's validate plus query_language's typecheck.
- Use case rows: typecheck (`inbound/use_cases/typecheck/{use_case 178, service 113, errors 72, keeper 63, ...}`) ->
  inbound/validate half, with the checking flow in ryoql/plan; describe (106+) -> inbound/describe (query-language
  half; `DescribeUseCase::describe`, uses `SyntaxPort::grammar`).
- `domain/plan_lowering` (2951): imports only itself and `ryo_values::domain::finish_plans`; no other query_language
  module. Callers: executor (`check_session.rs:91`, `lower.rs:4`, `request_input.rs:2`) and `typecheck/keeper.rs:4`.
  -> `domain/plan`, high. The plan data type itself is `ryo_values::domain::finish_plans` (not in the tree: see NO HOME).
- `domain/lowering` (641, errors 278 + types 309) and `domain/planning` (271) belong to a SECOND lowering: program to
  `steps` for the validate check (`assembling::lower_with_observations`, used at `use_case.rs:` imports). They are used
  by assembling, calling, combining, comparing, idioms, iterating, joining, reshaping, selecting, spanning, rendering,
  substituting, traversing, refusing, remapping, rewriting, resolving. Not imported by plan_lowering. -> ryoql
  (typed checks), medium; alternative `domain/plan`. Owner decides.
- `domain/validation` (516, one fn `validate_with_observations`), `checking` (295), `syntax` (811: operator
  name/shape tables, `operators_used`, `relations_read`), `linking`, `expanding`, `binding`, `remapping`, `rooting`,
  `observation_inputs` -> ryoql, high/medium.
- `domain/kind_check` (197): `asked`, `unknown_word`, `names_kind` take a `FinishPlan` (the plan type). -> ryoql per brief,
  but it makes ryoql import plan types: an order point (ryoql -> plan, allowed domain to domain; plan must not import ryoql).
- `domain/syntax/constants.rs` (291): `operator_name`/`operator_named` map `QueryOperator` to text. Fits ryoql and
  operators; report as two-home, low.
- `outbound/syntax` (1634): `SyntaxPort` (5 methods) with one adapter, winnow (adapter 1160 + constants 376), which
  reads RyoQL text into `ryo_values::domain::program` types. -> ryoql (parsing). The tree has no outbound home for a
  parser port; the port is a contract, so report it, never drop it. Domain would import the `winnow` crate directly.
- `outbound/catalog` (79): `CatalogPort::relations()` (1 method) returns `relations::service::all()`. One adapter
  (`CatalogLanguageAdapter`). -> ryoql, or domain/relations read directly; report as a port with one trivial adapter.
- Dependencies in Cargo: relations, language, matching, values. `ryo_language` and `ryo_matching` are imported by a
  domain crate today: check (direction conflict) which module uses them (not done).

### sdk (6,667)
- Public surface: `lib.rs` re-exports `languages, linter, operator, query, relation` (`resources.rs:12`). Surface =
  `Linter::new(languages,pool,cache)` + `Linter::lint` (`resources/linter/lint/entry.rs`, 97), public types
  (`lint/types.rs` 347, `errors.rs` 159, `schemas.rs` 59), `domain/*` types (cell, file, host, language, observation,
  parameter, place, project, query, relation, operator, capacity).
- Wiring that is NOT public surface (belongs to inbound/lint): `lint/session/entry.rs` (911: `Live` session, `admit`
  250 lines, `epoch`, `deliver`, `render_receipt`, error mapping), `session/call_cache.rs` (338: path-glob `PathFilter`
  with `globset`, interning, call cache), `session/sdk_request.rs` (94: observation inputs), `session/modules.rs` (23),
  `session/measure.rs` (75, behind `bench-hooks`), `session.rs` (62: `natives`, imports `ryo_executor::check_session::*`,
  `ryo_executor::bench_support`), `lint/executor.rs` (59: `Execute` fn-pointer hook `in_use()` linking a session),
  `lint/composition.rs` (80: builds the rayon scheduler, `matching::factory::globset`, `LanguageConnection`),
  `lint/connections.rs` (9), `lint/rows.rs` (68: memory budget via `ryo_core::...memory::factory::sysinfo`, then
  `execute_rows`), `lint/walk.rs` (124: corpus walk via `ryo_core::outbound::corpus::factory::ignore`, store-skip path
  normalising). Total about 2,000 of 2,523 in `resources/linter` are wiring or session; about 520 is public types.
- Glob compile in `call_cache.rs` (PathFilter) is "mechanism": path globs belong to outbound/corpus per the brief.
- `resources/query` (911): `run` (316) builds a `LintRequest` and calls `lint::rows::answer` (wiring into lint);
  `validate` (424; `schemas.rs` 318 maps public types to core types; `composition.rs` builds
  `ValidateUseCaseImpl` + `TypecheckUseCaseImpl` + winnow + catalog = composition root for inbound/validate);
  `describe` (143, composition builds `DescribeUseCaseImpl` + winnow).
- `resources/relation` (432) and `resources/operator` (357): each verb is composition + schema mapping over a core use
  case (`ListRelations`, `DescribeRelation`, `ListOperators`, `DescribeOperator`). Public types + mapping stay; the
  composition files move with the inbound crate (describe).
- `resources/languages` (713): `languages.rs` 402 = about 200 `pub use` lines of `ryo_language` (139) and `ryo_relations`
  (56) internals (the language-author contract; allowed in `[workspace.metadata.api] allow`); `macros/*` (492: `link!`,
  `fills!`, `reads!`, `statements`, `catalog`, `constants`); `contract.rs` 208 (fixtures feature only; re-exports
  `ryo_core::...lint::coordinates` and fixtures). The macros are the contract languages use -> they go with
  outbound/language (the SDK re-export stays thin).
- SDK imports internal crates: core, execution, executor, matching, values, relations, language, query_language
  (`Cargo.toml`). It owns `ryo_executor::check_session` wiring: that is the largest import-direction conflict
  (edge importing executor internals).

### cli (3,344)
- Binary `ryo` (`main.rs` 42), lib `ryo_cli`. Commands (`resources.rs`): `operator list|describe`,
  `query run|validate`, `relation list|describe`; global `--root`, `--timings`, `--format`, `--quiet`, `--no-color`.
- Only `query run` calls the SDK (`ryo_sdk::query::run`, `resources/query/run/command.rs:50`). The other five commands
  build `ryo_core` use cases directly (`*/composition.rs`: `ListRelationsUseCaseImpl`, `DescribeRelationUseCaseImpl`,
  `ListOperatorsUseCaseImpl`, `DescribeOperatorUseCaseImpl`, `ValidateUseCaseImpl` + `TypecheckUseCaseImpl` + winnow
  + catalog), bypassing the SDK. Also `languages.rs` (118) lists the nine linked languages three times.
- More than call-and-render: `timings.rs` 398 + `timings/phases.rs` 223 (tracing subscriber, wall-attribution tree,
  Chrome trace); `output/` 516 (pager, progress via indicatif, dry-run, JSON/text schemas); `outcome/remedy.rs` 130
  (error -> next step; `Remedy` trait, 1 method); `resources.rs` 183 (clap tree, panic hook, `Edge`); `cases.rs` 139
  (test support). `query/run/rendering.rs` 58 writes rows through `ryo_core` `RowSinkPort`.
- Direct deps: sdk, execution, relations, core, language, values, query_language and the nine languages (optional).

### language (11,259)
- Contract (provider traits): `ParseUseCase` (11 methods, `parse/interface.rs:11`), `ProjectUseCase` (7 + one method
  per manifest relation by macro, `project/interface.rs:27`), `WalkUseCase: Fillers` (`walk/interface.rs:25`, 9
  methods + assoc const/type), `Fillers` (`domain/fillers/types.rs:157`, one method per relation by macro + 8),
  `FillItem` (53), `KeyedFillItem` (80), `ReadItem` (`domain/answers/types.rs:38`), `Visiting` (64, 1 method),
  `Semantics` (macro, one per relation, `answers/types.rs:112`), `ManifestItem` (`project/types.rs:117`),
  `FactSource` (`domain/facts/types.rs:109`, 6 methods), `SemanticSession` (`domain/semantics/types.rs:10`, 1),
  engine-side `ParseEngine` (`parse/types.rs:140`), `WalkEngine` (`walk/types.rs:98`), `ProjectEngine`
  (`project/types.rs:183`), `FileVisit` (`walk/types.rs:30`), `FileProject`, `NumberedFacts`, `FileRelations`,
  `NumberedVisit`, `NumberedReader` (`parse/named.rs`), sealed markers `Syntax`, `Manifest`, fixtures `ExpectedRow`.
  Total 24 pub traits. Implementors: blanket `ParseEngine for P: ParseUseCase`, `WalkEngine for W: WalkUseCase`,
  `ProjectEngine for P: ProjectUseCase`; the nine languages implement `ParseUseCase`/`FileRelations`/`Fillers`
  through the SDK macros. Callers: `ryo_executor`, `ryo_core`, SDK `Language`.
- Row/column-shaped in signatures: `WalkUseCase::walk(.. builder: &mut NodeTableBuilder, ..)`;
  `ProjectUseCase::file_rows(.. out: &mut dyn RowWriter)`; `Visiting::visit(.. out: &mut dyn Rows<Row>)`;
  `FillItem::fill(.. rows: &mut dyn Rows<..>)`; `FactSource::fill(.. out: &mut FilledColumns)`,
  `notes(.. &NodeTable ..)`. `NodeTable` (node_table 1986: struct-of-arrays columns of kind, parent, span, field, children;
  builder `enter_node`, `token`, readers `kind_of`, `span_of`) is table layout -> `outbound/store`, but the language
  contract names its builder: a contract conflict to show the owner.
- Modules: contract (answers 397, fillers 691, facts 726, semantics 121, project_context 118, fixed_names 145, reasons
  22, files 89, targets 162, notes 67, names 175, expansions 185, diagnostics 256, decoded 349, symbols 423, words 146,
  keys 326, shapes 356, trees 127, completion 169, builders 395, elements 866, flow 336, flow_table 137, kind_index 62)
  are what a language writes into or reads. `elements` (866): the element table of a file; `flow`/`flow_table`
  write control-flow graphs into the table. Who owns them is per-module; outside users: executor, core, sdk,
  execution_domain, values (grep done for 25 modules; `symbols`, `files`, `project_context` are used by core, executor,
  values: they act as shared values, not only the contract).
- `inbound/use_cases` (2351): parse 731 (+2590 fixtures), project 637, walk 1254 (numbered.rs 356). These are the
  provider traits plus their blanket engine impls.
- Imports: `ryo_relations` only (Cargo: one dependency). Imported items: `relations::types::*`, `terms`, `catalog`.
  Not checked: exact list per module (do `git grep -o "ryo_relations::[a-z_:]*"` once).

### relations (9,822)
- `domain/relations` 8,341: about 80 relation items (`collections/relations/<name>/item.rs`, 62-140 lines each; row struct,
  column enum, class, scope, plus storage hints `form Filled(Unique(Id))`, `size`, `keys`, `notes`) + `types.rs` 492
  + `service.rs` 351 + `constants.rs` 190 + `collections/` 121.
- Catalog proper: relation items, `catalog/` (556), `labels` (64), `terms` (281), `kinds` (169), `*_kind` (6 small
  modules, about 280 total), `answers` (68: `AbsenceCause`), `elements` (65: origin/mapping types), `value_kind` (44).
- Shape/table types in `domain/relations/types.rs`: `Value` (l.177, 16-byte cell), `CellKind` (220), `RowWriter` (261,
  generated, one `Rows<Row>` per relation), `Rows<Row>` (277), `FactColumn` (358), `ColumnsView` (363), plus layout
  types `FactForm` (401), `FilledKey` (420), `NodeColumn` (431), `NodeExtent` (466), `RelationSize` (485); in
  `catalog/service.rs`: `fact_form`, `node_extent`, `effective_extent`, `relation_size`, `key_columns`, `needs_notes`
  (storage hints). `CellKind` is column kind (stays: catalog needs it). Row-shaped: `Rows`, `RowWriter`, per-relation
  `*Row`. Column-shaped: `FactColumn`, `ColumnsView`.
- Each item also names an `expected { FunctionExpectedRow }` type (the language crate's test fixture rows): the catalog
  declaration depends on language test fixtures (`parse/fixtures/expected_rows`)? Verify direction: relations has no
  Cargo dependency, so the type must be defined in relations: not checked.
- Pub traits: `RelationsItem` (+sealed `Sealed`) `collections/relations/interface.rs:22`; `Rows` (277); `RowWriter` (261);
  `Term` `terms/types.rs:54`.

### Language crates (nine)
- All nine depend on the SDK only (`ryo = idiomatic-sh-ryo-sdk`, `../../crates/sdk`). Python also has `ryo_relations`
  optional (feature `fixtures`). Language-side items used: `ryo::languages::*` (python 343, typescript 154, rust 173,
  go 148, html 49, css 54, markdown 59, json 46, toml 46 occurrences), macros `ryo::link!` (1 each), `ryo::fills!`
  (python 49, ts 31, go 19, html 8, css 6, md 8, json 7, toml 7, rust 2), `ryo::reads!` (python 8, ts 6, rust 13, go 6).
  Never `ryo_language` or `ryo_relations` directly in `src`.
- Entry points: `pub fn linked() -> ryo::languages::Language` (all nine; python also `linked_with_settings(settings::
  PythonParserSettings)`), `pub fn walk() -> impl ryo::languages::Walk` (python, typescript, rust only; python also
  `walk_with_settings`). Types named: `ryo::languages::Language`, `ryo::languages::Walk` (alias of
  `ryo_language::...WalkUseCase`), python `settings::PythonParserSettings`.

### Benchmark side
- `crates/benchmark` (35,008; package `idiomatic-sh-benchmark`, lib `benchmark`, bin `benchmark-collector`): depends on
  `idiomatic-sh-profile-tools` (path `profile_tools`), `idiomatic-sh-benchmark-contracts`, `idiomatic-sh-ryo-sdk` (as
  `ryo`), `idiomatic-sh-ryo-values` (as `ryo_values`). Internal items named: `ryo_values::domain::table_words::constants::
  {RESOURCE_ROUTES_TARGET, EXECUTION_TARGET}` (`outbound/engine/adapters/local/writer.rs:23,39`),
  `ryo_values::domain::runs::constants::SYNTAX_REJECTED` (`domain/corpus_discrepancy/service.rs:284`); the string
  target `"ryo::node_counters"` (`application/profile_evidence.rs:325`, `writer.rs:24`) couples to the SDK session's
  tracing event `target: "ryo::node_counters"`. All other uses are SDK: `ryo::linter::*`, `ryo::query::*`, `ryo::types::{
  RequestJson, ReportBytes}` (check that last path: not in SDK `lib.rs`; may be a bench-hooks or fixtures path).
- `crates/benchmark_contracts` (3,654; `idiomatic-sh-benchmark-contracts`): depends on the SDK only; names only
  `ryo::linter::*`.
- Profile tools: `crates/benchmark/profile_tools`, package `idiomatic-sh-profile-tools`, lib `profile_tools`, 1141 lines,
  no Ryo dependency; used only by `benchmark`. Also `tools/install.py` and `tools/lints` exist at repo top (not mine).
- Linter benchmark crates: `idiomatic-sh-ryo-python-ruff-benchmark`, `idiomatic-sh-ryo-rust-clippy-benchmark`,
  `idiomatic-sh-ryo-typescript-oxlint-benchmark`, each dev-deps only. Ruff: `ryo_executor`, `ryo_python`,
  `benchmark-contracts`, `benchmark`, SDK, plus serde/toml/dunce. Internal items named: `ryo_executor::request_input`
  (bench.rs, tests.rs in all three), `benchmark::inbound::use_cases::run_ryo::{..}` (`composition/commands.rs` in all
  three). Python additionally mentions `ryo_core`/`ryo_values` types only inside a JSON evidence string (not code).
  `executor/benches/lint_image` (`ryo-lint-images`, own `[workspace]`): SDK with `bench-hooks`, python/typescript/rust.
  Not checked: clippy and oxlint crate dev-dependency lists beyond the Ruff one (same names expected).

## Totals per destination (draft, lines)
To compute when writing: per file ranges and the module totals above. Not yet summed.

## NO HOME (draft)
- `ryo_values` crate (the plan data type `finish_plans`, `program`, `query_tokens`, `descriptions`, `runs`): every port in
  query_language and the SDK names it; the tree has no home for the shared values. Out of my scope; flagged.
- query_language `outbound/syntax` and `outbound/catalog` ports (see above).
- `cli/timings*` (621) and `cli/output` (516): CLI-only concerns; the tree has `edges/(cli)`, so they stay there; they are
  "more than call and render" and are listed, not NO HOME.
- language `node_table` + `NodeTableBuilder` in the language contract: two homes (outbound/language vs outbound/store).
- relations layout types (`FactColumn`, `ColumnsView`, `FactForm`, `FilledKey`, `NodeColumn`, `NodeExtent`).
- SDK `session/call_cache.rs` globs: corpus vs lint.

## Import-direction conflicts (draft)
- SDK (edge) -> executor/core/execution/matching internals: `resources/linter/lint/session.rs`, `composition.rs`.
- cli (edge) -> core use cases and `ryo_query_language::outbound::*` directly (`*/composition.rs`).
- language (outbound) -> relations (domain): allowed. Language contract names `RowWriter`/`NodeTable*` (layout).
- query_language -> language (Cargo) and matching: ryoql/plan (domain) importing an outbound crate: find the module.
- benchmark/linter benchmarks -> `ryo_executor::request_input`, `benchmark` internals.

## Not yet checked
- Callers/implementors of `SyntaxPort`, `CatalogPort`, `TypecheckUseCase`, `DescribeUseCase` beyond the SDK/CLI
  composition roots; `Remedy` and `Encoded` (cli) implementors.
- Why query_language Cargo lists `ryo_language` and `ryo_matching`.
- Exact `ryo_relations` import list of `language`; `ryo::types::RequestJson` path; clippy/oxlint dev-dependencies.
- "No caller found" section: needs a grep per public item (not started).

## Steps once writing is allowed
1. Write `/workspaces/ryo-poc-evidence/next/build/ports/stage2/mapping/trunk-kept-crates.md` with the six sections in
   the brief's order, one row per module, using the numbers above.
2. Final message: at most 12 lines (lines per destination, NO HOME count, three findings: five of six CLI commands bypass
   the SDK; the SDK holds the executor session wiring; the language contract names column-shaped `NodeTableBuilder` and
   `RowWriter`; plan_lowering is a clean `domain/plan` cut but a second step-lowering family sits beside it).
