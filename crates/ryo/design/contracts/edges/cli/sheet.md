# edges/cli
STATUS: drafted (28 Nov; not reviewed, not agreed)

Folder today `crates/ryo/cli`; package `idiomatic-sh-ryo`; library `ryo_cli`; binary `ryo`. Bead ryo-fmx3.1.13.23.4.22.
Revisions: MAIN = f4ae3bd802 (release 0.2.0), TIP = 2a207464c (build 139), B133 = f0219b3b3 (not needed on this sheet).
Every read was `git -C /workspaces/idiomatic-public-benchmark show|grep|ls-tree|log` at a fixed revision.

## 1. Owns

One `ryo` command line turned into one call of an inbound use case, its answer printed in the format the line chose.
One thing: the shell. Composition (building the ports and use cases a command calls) is part of it because the agreed
rule puts composition at the edges (Appendix A section 2: "the worker pool, the memory probe, the file walker, the store
and the cache are built in the SDK and again in the command line, and handed in"). Printing (`output`), exit codes
(`outcome`) and `--timings` are the shell's own parts, placed by the shared Tastemaker spec `idiomatic.arch/cli`; none
of them is a concept another crate needs.

## 2. On main (f4ae3bd802)

**Where.** `crates/ryo/cli`: 3,441 Rust lines in `src` without tests, 2,316 lines of unit tests beside them (counted with
`git show` over `ls-tree` of `src/**.rs`, split on `_test.rs`). Six commands, `<resource> <verb>`: `operator list|describe`,
`query run|validate`, `relation list|describe`; global `--root`, `--timings`, `--format`/`--json`/`-q`, `--no-color`.
No bench (`benches/performance/` was deleted by f4ae3bd802 itself, see section 4); `[[test]] smoke` installs from
crates.io and runs `query run`:
```
name = "smoke"  @f4ae3bd802:crates/ryo/cli/Cargo.toml:92
```

**Its public items: 58, plus 24 public modules** (grep of `^\s*pub (fn|struct|enum|trait|const|...)` in `src`, tests
excluded): `cases` 7 (test support), `edge::Edge` 1, `linked_connections` 1 (feature `fixtures`), `outcome` 4,
`output` 12, `output::dry_run` 3, `output::progress` 2, `resources` 3, each resource 6 (x3 = 18), `timings` 7. The ones
the binary and the tests name:
```
pub fn command() -> clap::Command {  @f4ae3bd802:crates/ryo/cli/src/resources.rs:43
pub fn edge_at(  @f4ae3bd802:crates/ryo/cli/src/resources.rs:73
pub fn run(  @f4ae3bd802:crates/ryo/cli/src/resources.rs:96
pub struct Edge {  @f4ae3bd802:crates/ryo/cli/src/edge.rs:7
pub build: ryo_core::outbound::lint_store::types::BuildIdentity,  @f4ae3bd802:crates/ryo/cli/src/edge.rs:17
pub fn linked_connections() -> Vec<LanguageConnection> {  @f4ae3bd802:crates/ryo/cli/src/languages.rs:40
```
**Its two public traits** (both in the ledger's count; both implemented only inside the crate):
```
pub trait Remedy {  @f4ae3bd802:crates/ryo/cli/src/outcome/remedy.rs:40
pub trait Encoded: std::fmt::Display {  @f4ae3bd802:crates/ryo/cli/src/output.rs:87
```
No crate-internal (`pub(crate)`) trait.

**What it calls.** All six commands compose `core` use cases themselves; none goes through the SDK (the SDK is a
dev-dependency only, below `[dev-dependencies]`). `query run` builds core's lint use case with fourteen ports and
streams rows to a sink the command owns:
```
ryo_sdk = { package = "idiomatic-sh-ryo-sdk", path = "../crates/sdk" }  @f4ae3bd802:crates/ryo/cli/Cargo.toml:56
use ryo_core::inbound::use_cases::lint::interface::LintUseCase as _;  @f4ae3bd802:crates/ryo/cli/src/resources/query/run/command.rs:3
pub(crate) fn build(  @f4ae3bd802:crates/ryo/cli/src/resources/query/run/composition.rs:23
service: LintUseCaseImpl::new(  @f4ae3bd802:crates/ryo/cli/src/resources/query/run/composition.rs:68
Arc::new(crate::output::schemas::rows(out, edge.format)),  @f4ae3bd802:crates/ryo/cli/src/resources/query/run/composition.rs:86
impl TryFrom<RunArgumentsSchema> for LintRequest {  @f4ae3bd802:crates/ryo/cli/src/resources/query/run/schemas.rs:74
answer: AnswerShape::Rows(RowTotal(schema.limit)),  @f4ae3bd802:crates/ryo/cli/src/resources/query/run/schemas.rs:108
pub trait LintUseCase: Send + Sync {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/lint/interface.rs:7
```
So on main the command itself turned each `--text` into a rule ("1", "2", ...) and the lint use case answered rows of the
rules' own columns, written row by row as produced.

**Who calls it.** Its own binary (`main.rs`), its integration tests and nothing else: no `Cargo.toml` in the repository
depends on `idiomatic-sh-ryo` (grep of every `Cargo.toml`: only the crate itself and a TOML fixture), and `ryo_cli::`
appears outside the crate only in the test text of a Tastemaker rule that forbids importing it.

**Proof.** 73 `#[test]` in `src`, 81 in `tests/` (the generated command-line and command-tree contracts, the README test
and the replay suite, whose modules lint through `ryo_core` and `ryo_sdk`: 72 and 66 paths).

**Tastemaker entries on it** (`crates/ryo/tastemaker.toml`): `library-surface` (line 16), `internal-package-allowlist`
for `cli/Cargo.toml` (20), `edges-apart` (39), `no-negated-feature-cfg` (45), `no-language-names` (47), `cli-crate` (124)
and the `idiomatic.arch/cli` spec at version 0.13.0 (125):
```
{ alias = "edges-apart", rule = "idiomatic.arch/layer-order", include = ["cli/Cargo.toml"]  @f4ae3bd802:crates/ryo/tastemaker.toml:39
```

## 3. On the perf branch (2a207464c)

**Where.** Same folder: 3,319 Rust lines in `src` without tests, 2,272 lines of unit tests, plus `benches/performance/`
(12 files, the `commands` bench, still placed by cli spec 0.12.3) and the frozen answers of `query run`
(`src/resources/query/run/command_test/frozen/00.txt` to `44.txt` and `index.txt`).

**Its public items: 59, plus 24 public modules**: main's 58 and
```
pub fn linked_sdk() -> ryo_sdk::linter::Languages {  @2a207464c:crates/ryo/cli/src/languages.rs:72
```
The same two public traits, and no crate-internal trait. `Edge` lost `build` and keeps the machine's thread count as
values' type:
```
pub trait Remedy {  @2a207464c:crates/ryo/cli/src/outcome/remedy.rs:40
pub trait Encoded: std::fmt::Display {  @2a207464c:crates/ryo/cli/src/output.rs:87
pub struct Edge {  @2a207464c:crates/ryo/cli/src/edge.rs:7
pub machine: ryo_values::domain::runs::types::MachineWorkers,  @2a207464c:crates/ryo/cli/src/edge.rs:19
```

**What it calls.** `query run` now goes through the SDK as a normal dependency; the other five still build `core` use
cases:
```
ryo_sdk = { package = "idiomatic-sh-ryo-sdk", path = "../crates/sdk" }  @2a207464c:crates/ryo/cli/Cargo.toml:22
let report = ryo_sdk::query::run(  @2a207464c:crates/ryo/cli/src/resources/query/run/command.rs:50
pub(super) fn write(report: &RunReport, sink: &dyn RowSinkPort) -> Result<(), RunSchemaError> {  @2a207464c:crates/ryo/cli/src/resources/query/run/rendering.rs:33
impl RowSinkPort for RowsSchema {  @2a207464c:crates/ryo/cli/src/output/schemas.rs:202
pub(crate) fn build() -> impl ValidateUseCase {  @2a207464c:crates/ryo/cli/src/resources/query/validate/composition.rs:22
service: ValidateUseCaseImpl::new(  @2a207464c:crates/ryo/cli/src/resources/query/validate/composition.rs:33
service: ListOperatorsUseCaseImpl,  @2a207464c:crates/ryo/cli/src/resources/operator/list/composition.rs:24
service: DescribeRelationUseCaseImpl::new(Arc::new(crate::languages::linked())),  @2a207464c:crates/ryo/cli/src/resources/relation/describe/composition.rs:27
#[arg(long, default_value_t = ryo_core::inbound::use_cases::lint::constants::DEFAULT_LIMIT)]  @2a207464c:crates/ryo/cli/src/resources/query/run/command.rs:24
```
The SDK's `query::run` does the shaping that main's command did: each text a rule, the headings checked once more
through ryoql, a lint with no cache, the findings turned back into rows with `rule`, `line` and `column` columns:
```
pub fn run(  @2a207464c:crates/ryo/crates/sdk/src/resources/query/run/entry.rs:20
let linter = sdk::Linter::new(languages, pool.clone(), sdk::Cache::None);  @2a207464c:crates/ryo/crates/sdk/src/resources/query/run/entry.rs:69
let report = sdk::lint::rows::answer(&linter, lint, usize::try_from(request.limit.0).unwrap_or(usize::MAX), memory)  @2a207464c:crates/ryo/crates/sdk/src/resources/query/run/entry.rs:70
```
The command then copies the SDK's report into core's `LentCell` only to hand it to its own `RowSinkPort`. The error
classes carry both families, core's dead one (its `LintUseCase` has no implementor and no caller at the tip) beside the
SDK's:
```
impl Remedy for ryo_core::inbound::use_cases::lint::errors::LintError {  @2a207464c:crates/ryo/cli/src/outcome/remedy.rs:71
impl Remedy for ryo_sdk::query::RunError {  @2a207464c:crates/ryo/cli/src/outcome/remedy.rs:87
```

**Three language lists** (generated by `build.rs` from the directories of `languages/`):
```
pub(crate) fn linked() -> impl LanguagePort {  @2a207464c:crates/ryo/cli/src/languages.rs:25
pub fn linked_connections() -> Vec<LanguageConnection> {  @2a207464c:crates/ryo/cli/src/languages.rs:40
pub(crate) fn skipped() -> SkippedDirectories {  @2a207464c:crates/ryo/cli/src/languages.rs:97
```
plus `linked_sdk` above: one list of `LanguageConnection` for core, one of the SDK's `Language` for `query run`, one of
skipped directories.

**Who calls it.** The binary, the contract tests, the replay suite (also `edge_at`) and the `commands` bench (also the
six `command::run` and their argument types: 13 distinct `ryo_cli::` paths). No other crate (same grep as on main).
```
ryo_cli::resources::run(here, machine)  @2a207464c:crates/ryo/cli/src/main.rs:41
let mut tree = ryo_cli::resources::command();  @2a207464c:crates/ryo/cli/tests/command_line.rs:14
ryo_cli::linked_sdk()  @2a207464c:crates/ryo/cli/tests/fixtures/replay/base.rs:76
ryo_cli::resources::edge_at(  @2a207464c:crates/ryo/cli/benches/performance/measure.rs:15
```

**Proof.** 74 `#[test]` in `src`, 80 in `tests/`: `command_line.rs` 7, `command_tree.rs` 11, `readme.rs` 1, the replay
suite 60 in `tests/replay/` plus 1 in `tests/fixtures/replay/spans.rs`. The replay suite names `ryo_sdk::` 111 times and
`ryo_core::` 9 times. The 45 frozen answers back `query run`'s command test.

**Tastemaker entries on it**: lines 16, 18 (allowlist), 36 (`edges-apart`), 37 (`distribution-wiring`), 43, 45, 114
(`cli-crate`) and 115 (cli spec 0.12.3):
```
{ alias = "edges-apart", rule = "idiomatic.arch/layer-order", include = ["cli/Cargo.toml"]  @2a207464c:crates/ryo/tastemaker.toml:36
{ spec = "idiomatic.arch/cli", include = ["cli"], params = { library = "ryo_cli", markdown = "offered", product = "ryo", writes = [] }  @2a207464c:crates/ryo/tastemaker.toml:115
```
FINDING: the tip's normal dependency on the SDK (`Cargo.toml:22`) is what `edges-apart` refuses ("depends on no crate of
the SDK"), and the allowlist at line 18 says "the SDK only for its tests". I did not run Tastemaker; either the check
fails at the tip or it is not run there.

## 4. What changed between them

- **Added at the tip**: `query run` through `ryo_sdk::query::run` (2bde413a94, 26 Nov: "query run is the SDK's resource
  alone ... the command line calls the SDK entry and renders"; before it 7360de99c4 and ac485fd204: "6 of 54 lines
  differ: the executor keeps one row per key"); `linked_sdk` (one more list); `rendering.rs`; the 45 frozen answers
  (3b38eaa45b: "query run's old path is deleted ... the 45-case test reads the frozen answers alone").
- **Removed at the tip**: `query/run/composition.rs` (main's fourteen-port composition, 3b38eaa45b); `Edge.build`
  (f2d1845a4f: "the lint store port is deleted ... the cli keeps its store directory name as a plain string"); the
  dependencies on `ryo_execution_runtime` and `ryo_operators` (6f29f71647).
- **On main and not on the tip**: f4ae3bd802 moved the crates.io smoke test into the crate and deleted the `commands`
  bench, under cli spec 0.13.0 ("(owner, option 1)" further on the same line). The tip branched before (merge base
  f4a32acb56) and still has the bench.
  ```
  The never-run `commands` bench is gone:  @f4ae3bd802:tastemaker/specs/cli/0.13.0/CHANGELOG.md:5
  ```
- **Changed in kind**: on main `query run` wrote each row as the engine produced it; at the tip the SDK collects at most
  `--limit` decoded rows (default 20) and the command writes them after the run. The answer is the same shape; every row
  is still counted.
- **Lost that should not have been**: nothing of the command line's own. What is lost is a boundary: `edges-apart`, and
  "each command composes its own service" for `query run`; the draft restores both.
- 52 commits touch `crates/ryo/cli` between main and the tip (`git log --oneline f4ae3bd802..2a207464c -- crates/ryo/cli`).

## 5. The drafted public interface

The listing is `edges/cli/api.txt`: 58 public items and 24 public modules. Its paths are the real module paths, because
they are fixed by the shared spec (decision 3).

**What stays, and why it is not smaller.** Every public item the tip has except two. The shape (`resources`, `edge`,
`outcome`, `output`, `timings`, `cases`, one module per command, `run(arguments, edge: &Edge) -> Outcome`) is written by
the templates of `idiomatic.arch/cli` and `idiomatic.arch/command`, which Tastemaker's and Asymptote's command lines
follow too. The contract tests are generated and name the tree by path. Only the binary and the crate's own tests call
these items, and no crate may import `ryo_cli`, so the size of this surface costs no other crate anything (aim 3).

**What changes (the 7 `[changed]` lines and the 1 `[new]` line of api.txt).**
- `resources::edge_at`, `resources::run`, `edge::Edge`: the types in them move to their owners. The roots become the
  language contract's `ryo_language::RootPath` (outbound/language api.txt), the skipped directories the corpus'
  `ryo_corpus::SkippedDirectories` (outbound/corpus api.txt), the machine's thread count the scheduler's
  `ryo_scheduler::WorkerCount` (outbound/scheduler api.txt maps values' `MachineWorkers` to it). Signatures otherwise
  unchanged:
  ```
  pub fn edge_at(  @2a207464c:crates/ryo/cli/src/resources.rs:68
  pub fn run(  @2a207464c:crates/ryo/cli/src/resources.rs:87
  fn run_from(  @2a207464c:crates/ryo/cli/src/resources.rs:100
  ```
  (`run_from` stays private; it is listed only to say that it is NOT made public: with the bench gone nothing outside
  needs it.)
- `resources::query::run::command::run`: same signature; its body calls inbound/lint instead of the SDK:
  ```
  let report = lint.rows(request, ryo_lint::RowLimit(limit), &ryo_lint::memory::sysinfo(budget))?;  @new
  ```
  where `lint` is a `ryo_lint::LintUseCaseImpl` the command's own `composition.rs` builds from `ryo_lint::LintParts`
  (the factories listed at the end of api.txt: `ryo_programs::LoadUseCaseImpl` over ryoql's parse, plan's lowering and
  operators' plan; `ryo_screen::Screen`; `ryo_execute::ScanUseCaseImpl`; `ryo_project::CrossFileUseCaseImpl`;
  `ryo_corpus::factory::ignore` and `globset`; `ryo_cache::memory()`, since `query run` keeps nothing; `ryo_scheduler::rayon`
  with `workers_to_start(WorkersAsked(0), edge.machine)`; one `ryo_store::Stores` per call with the call's worker count;
  `ryo_ryoql::CheckUseCaseImpl`). This is main's shape (main composition.rs:23-91) on the new crates. `--limit`'s default
  becomes the command's own private constant, the value unchanged, as the lint api.txt's NO LONGER PUBLIC list says:
  ```
  pub const DEFAULT_LIMIT: u64 = 20;  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/lint/constants.rs:4
  ```
- Three `impl Remedy` lines: one for `ryo_lint::LintError` in place of core's dead `LintError` and the SDK's `RunError`;
  `ryo_validate::ValidateError` and the four `ryo_describe` errors in place of core's. The `Remedy` trait and its
  no-wildcard rule are unchanged.
- `linked` (`[new]`, feature `fixtures`): `pub fn linked() -> ryo_language::Languages  @new`, the one list of the
  languages this build links, replacing `linked_connections` and `linked_sdk` (decision 4).

**What the other commands call** (private compositions, against the drafted inbound/validate and inbound/describe
listings): `query validate` -> `ryo_validate::ValidateUseCaseImpl::new(check, lower, languages.kind_vocabularies())`
with ryoql's `CheckUseCaseImpl` and plan's `LowerUseCaseImpl::new(ryo_operators::CompareUseCaseImpl::new())`;
`relation list|describe` -> `ryo_describe::{ListRelationsUseCaseImpl, DescribeRelationUseCaseImpl}::new(languages)`,
`languages` being the one `ryo_language::Languages` (it implements `LanguagePort`); `operator list|describe` ->
`ryo_describe::{ListOperatorsUseCaseImpl, DescribeOperatorUseCaseImpl}::new(ryo_operators::CatalogUseCaseImpl::new())`.
At the tip the operator commands call core's unit structs and validate takes core's typecheck and `LanguagePort`.

**Printing `query run`.** The command writes a `ryo_lint::RowsReport` straight to its writer. Core's `RowSinkPort` (one
production implementor, this crate's `RowsSchema`) leaves core and is not lint's (lint sheet, decision 5); here it becomes
private code, not a port, and the copy in `rendering.rs` goes. `UNLOCATED` and `location` (relations api.txt lines 192
and 195) still decide how an absent span prints.

**What stops being public**: `linked_connections` and `linked_sdk` (api.txt, NO LONGER PUBLIC).

## 6. Proof it brings

- **Carried as they are** (path changes only): the unit tests of `outcome` (4), `output` and its parts (22),
  `timings` (14), the command and schema tests of operator and relation (12); `tests/command_line.rs` (7),
  `tests/command_tree.rs` (11) with `surface.txt`, `tests/readme.rs` (1); these are generated or placed by the cli spec.
- **Restated** (they name a type that moves): `outcome/remedy_test.rs` (8: core's and the SDK's errors become lint's,
  validate's, describe's); `query/run/command_test.rs` (5, with the 45 frozen answers, which must still match byte for
  byte) and `query/run/schemas_test.rs` (3); `query/validate/*_test.rs` (4); `languages_test.rs` (2: one list).
- **The replay suite** (61 tests): restated only in `base.rs` and `languages.rs`, which call `linked` in place of
  `linked_sdk`. It keeps linting through the SDK's `Linter::new`/`lint` as a dev-dependency, which the allowlist permits
  (decision 5).
- **From main**: `tests/smoke.rs` (main's crates.io smoke, `test = false`) replaces the tip's `../distribution` test and
  bench entries, as cli spec 0.13.0 places it:
  ```
  name = "distribution"  @2a207464c:crates/ryo/cli/Cargo.toml:103
  ```
- **Missing**: a test that `query run` builds no SDK type (the `edges-apart` rule covers the manifest; nothing covers a
  dev-dependency leaking into `src`; `fixtures-only-in-tests` is the nearest).

## 7. Perf gains it carries

Rows of `perf-gains.tsv` whose crate column names cli: 37 and 52.

- **Row 37** (allocator swap; jemalloc -14 to -17% whole process, `engine/STATE.md:3366`). The command-line part is that
  the binary picks the allocator and no library does:
  ```
  static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;  @2a207464c:crates/ryo/cli/src/main.rs:22
  static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;  @f4ae3bd802:crates/ryo/cli/src/main.rs:22
  static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;  @2a207464c:crates/ryo/cli/tests/replay/main.rs:34
  ```
  KEPT BY: `main.rs` moves path-only; the cli spec's `cli-main` template requires these statics on both sides (a main
  without them is a `cli-main#template` finding, cli spec 0.13.0 CHANGELOG line 49). The pools (columns, tables,
  per-file buffers) are the store's; the command line hands the store the call's worker count through `LintParts` and
  nothing else.
- **Row 52** (Tastemaker's check: teardown skipped by forgetting the composed services after the output; CPU 52.4 ->
  29.7 s). Its files are Tastemaker's command line and adapter, not Ryo's: Ryo's command line has no `mem::forget` or
  `process::exit` today (grep of `crates/ryo/cli` at the tip). KEPT BY / ROOM: each command composes its own use case
  inside its own `composition.rs` and drops it when `run` returns, so forgetting it after the answer is written would
  land in this crate alone. `query run` composes `ryo_cache::memory()`, so nothing waits on a teardown to reach disk
  (lint's contract: every record on disk before `lint` returns). Not proposed now: not measured for `ryo`.
- **Aim 3, the history.** Under this draft the changes of section 10.3 that would have landed inside the command line
  alone: the binary's allocator (row 37's `main.rs` part), and a teardown skip like row 52's. The 26 Nov move of
  `query run` onto the executor (7360de99c4, 2bde413a94; no figure of its own) crossed the command line and the SDK; with
  decision 2 it lands in inbound/lint alone. Still crossing into this crate: any change that adds or changes a part of
  `ryo_lint::LintParts` (row 54's per-worker stores, row 13's parked frozen plan shared by every worker, a cache mode as
  in rows 2, 14 and 53): the command line composes, so it edits its `composition.rs`, as the SDK edits its own. That is
  the agreed price of composing at both edges; it touches no other crate.

## 8. Benchmarks that cover it

- **The three linter benchmarks** (Python Ruff, TypeScript Oxlint, Rust Clippy) and `benchmark-collector`: they call
  nothing in this crate today (no manifest depends on `idiomatic-sh-ryo`; they lint through the SDK) and nothing after.
  The allocator they run under is the collector's own, not this binary's.
- **The `commands` bench** (tip only): six workloads, one per command, CPU per round in Bencher format, run by Asymptote's
  `performance` tool:
  ```
  run = ["cargo", "bench", "--locked", "--workspace", "--bench", "commands"]  @2a207464c:crates/ryo/asymptote.toml:211
  ```
  It reaches for `ryo_cli::resources::edge_at`, `ryo_cli::output::Format::Quiet`, `ryo_cli::outcome::Outcome::Success`,
  the six `command::run` and their argument types, and, outside the crate, `ryo_language::...::RootPath` and
  `ryo_values::...::MachineWorkers(1)`. Main deleted it (cli spec 0.13.0, "owner, option 1"). Since the new branch takes
  main's five commits, the draft has no `commands` bench and keeps no public item for it. Four lock files under
  `crates/ryo/.asymptote/operations/` name `query/run` or `operator/list` (not read); they go with the bench. If the
  owner wants it back, every path it calls is still public under this draft, with `RootPath` and `WorkerCount` from
  their new crates.
- **The smoke test** (main's `tests/smoke.rs`, run when named): it runs the installed binary, `query run` of a comment
  query over one `smoke.rs`; it calls no item of the library.
- **The replay suite**: correctness, not speed; it lints through the SDK and links what the command line links.

## 9. Checks

IMPORTS: lint, validate, describe, language, corpus, cache, scheduler, store, programs, ryoql, plan, operators, screen, execute, project, relations, table, languages
CROSSED: per request

- The direction table lets an edge import inbound and the factories of what it composes. Of the list, only `lint`,
  `validate` and `describe` are called; the rest are named in compositions (`query run` composes lint's parts) and, for
  `relations` and `table`, in printing an absent span. `languages` is the nine language crates, each optional behind its
  feature, as today. No other edge: the SDK is a dev-dependency only, for the replay suite.
- No layout word in a signature: none of the 58 items names a table, column, row slot or store type.
- Crossed once per command run: one composition, one use-case call, one print. Nothing in this crate runs per file.
- check-graph.py, run on 28 Nov once the languages sheet appeared: 21 sheets, 0 failures.

## 10. Decisions for the owner

1. **The command line composes `query run` itself, or calls the SDK.** Example: at the tip
   `let report = ryo_sdk::query::run(` (command.rs:50) with the SDK as a normal dependency (Cargo.toml:22), against
   `edges-apart` (tastemaker.toml:36); on main the command built core's lint use case with fourteen ports
   (composition.rs:23-91). Why: two edges that share nothing is the existing rule, and "built in the SDK and again in
   the command line" is Appendix A section 2. What we get: the command line names 15 Ryo crates in its composition
   instead of one, and a constructor change in `LintParts` is made in both edges. My pick: compose here, as main did.
2. **Where `query run`'s table is shaped.** Example: the tip's SDK `query::run` (entry.rs:20-106) turns texts into rules,
   checks the headings a second time, runs the lint and turns findings back into rows with `rule`, `line` and `column`;
   main did the same split between the command (texts to rules, schemas.rs:74-110) and core's lint (rows of the rules'
   columns). The lint draft's `rows` (its decision 2) answers a `LintReport` and a total, and the SDK draft keeps the
   shaping as "the edge's own" (`ryo::query::RunReport`, edges/sdk api.txt line 113). Why: with decision 1, that
   shaping (about 85 lines) would be written twice, in the SDK and here, and a change to how `query run` answers would
   cross both edges. What we get: one place; both edges only print. My pick: ask the lint sheet to answer the finished
   table from `rows` (columns, rows of `Slot`, total, shown, unread), taking the root and the query texts; this crate
   then prints only. If the owner keeps the SDK draft as it is, this crate carries its own private copy of the shaping
   (main's split: texts to rules here, as schemas.rs:74-110 did) and nothing in api.txt changes.
3. **Keep the surface the cli spec writes (58 items), or cut it with a new spec version.** Example: `output::present`,
   `timings::install`, `outcome::remedy::Remedy` and the six `command::run` are `pub` because the spec's templates write
   them so, and the generated contract tests name `ryo_cli::resources::command()` (command_line.rs:14). The smallest
   surface the callers need is 3 items (`resources::run`, `resources::command`, `edge_at`) plus test support. Why: the
   spec is shared by Ryo's, Tastemaker's and Asymptote's command lines; changing it is a new version of a shared
   definition for all three. What we get by keeping: no Tastemaker work, and no cost to modularity, since no crate may
   import `ryo_cli`. My pick: keep it; do not cut a spec version for this build.
4. **One language list.** Example: `linked()` (languages.rs:25), `linked_connections()` (:40), `linked_sdk()` (:72) and
   `skipped()` (:97), all written by `build.rs`. Why: three lists of one thing, two of them public for tests. What we get:
   one `connections()` building `ryo_language::Languages`, the skips read from each connection's walk engine
   (`WalkEngine::skips`, outbound/language api.txt line 95), one test-support function `linked()`. My pick: one list;
   `build.rs`'s generated text changes with it.
5. **Where the replay suite lives.** Example: 61 tests in `tests/replay/` that lint through `ryo_sdk::Linter` (111 SDK
   paths), placed in the command line by the cli spec's `replay-suite` template (the same entry at 0.13.0 not checked):
   ```
   { template = "replay-suite", file = "tests/replay/{suite}.rs", exclude = ["tests/replay/main.rs"], count = { min = 0 }  @2a207464c:tastemaker/specs/cli/0.12.3/spec.toml:24
   ```
   Why: what they prove is lint's behaviour through the SDK's call, not the command line's ("tests live in the owning
   crate"). What we get by keeping: no spec change, no move during the build; the cost is a dev-dependency on the SDK.
   My pick: keep them here for this build, restated only where they name `linked_sdk`; move them to the SDK afterwards
   if the owner wants the rule applied.

## 11. Not checked

- The languages sheet appeared at the end and was not read; each language's `linked() -> LanguageConnection` is
  Appendix A 3.21's claim. The scheduler has an api.txt (read) and no sheet. The SDK has an api.txt and no sheet. Of the lint,
  validate and describe sheets only the parts named here were read (their listings, lint's decisions 2 and 5,
  validate's constructor); they appeared while this sheet was being written.
- Whether Tastemaker passes at the tip (the `edges-apart` finding of section 3 is from reading, not from a run). No test,
  bench or check was run.
- Tastemaker's and Asymptote's command lines against the cli spec (decision 3 assumes they follow it; not read).
- The four `.asymptote/operations/*.lock` files that name `query/run` or `operator/list`: not opened.
- `build.rs` in detail (only its doc comments): how its generated text changes for decision 4.
- `timings.rs`, `output/pager.rs`, `output/progress.rs` bodies: read only for what they import (tracing, indicatif;
  no Ryo crate).
- The replay suite's 61 tests one by one: counted, and their crate paths counted, not read.
- Whether `ryo_store::Stores` implements `ryo_table::TablePort` as `LintParts` needs: the store sheet's claim; UNPROVEN
  until the table spike passes.
- Line counts exclude `benches/` and `tests/`; the plan's 3,344 for the tip differs from my 3,319 by method (not
  reconciled).
