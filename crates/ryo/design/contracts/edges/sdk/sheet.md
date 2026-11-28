# edges/sdk
STATUS: drafted (28 Nov 2026; not reviewed, not agreed by the owner)

Revisions: MAIN = f4ae3bd802 (release 0.2.0, live). TIP = 2a207464c (perf branch, build 139). Read through
`git -C /workspaces/idiomatic-public-benchmark show "REV:PATH"` only. Every quote is `line  @rev:path:line`.
Bead ryo-fmx3.1.13.23.4.21. The listing is `api.txt` beside this sheet.

## 1. Owns
What a user of Ryo calls, and the composition behind it: the SDK builds every port and use case once and hands them
down ("the edge owned by the sdk composes", owner, 2 Oct; memory `sdk-is-an-edge`). One thing. Two words in that
sentence ("calls" and "composition") are one job on the hex map: an edge is where the product's surface meets its
wiring, and the owner placed both in the SDK on purpose. It owns no behaviour: it does not lint, validate, describe,
keep, list files or schedule. Today it owns far more (section 3), which is the crate being wrong, not this sentence.

## 2. On main (f4ae3bd802)
**Where.** `crates/ryo/crates/sdk`, package `idiomatic-sh-ryo-sdk`, library `ryo`; 6,294 Rust lines without tests or
fixtures under `src/` (counted with `git show` over every `src/**/*.rs` that is not `*_test.rs` and not under
`fixtures`); `api.txt` 3,672 lines. Five resources:
```
pub use resources::{languages, linter, operator, query, relation};  @f4ae3bd802:crates/ryo/crates/sdk/src/lib.rs:19
```
**The linter contract, verbatim.**
```
pub fn lint(  @f4ae3bd802:crates/ryo/crates/sdk/src/resources/linter/lint/entry.rs:24
    pub fn new(languages: Languages, pool: WorkerPool, memory: MemoryBudget, cache: Cache) -> Self {  @f4ae3bd802:crates/ryo/crates/sdk/src/resources/linter/lint/entry.rs:46
    pub fn prepare(  @f4ae3bd802:crates/ryo/crates/sdk/src/resources/linter/lint/entry.rs:86
pub fn lint_with(linter: &Linter, mut request: LintRequest) -> Result<LintReport, LintError> {  @f4ae3bd802:crates/ryo/crates/sdk/src/resources/linter/lint/entry.rs:114
impl From<Workers> for WorkerPool {  @f4ae3bd802:crates/ryo/crates/sdk/src/resources/linter/lint/entry.rs:145
impl From<Arc<rayon::ThreadPool>> for WorkerPool {  @f4ae3bd802:crates/ryo/crates/sdk/src/resources/linter/lint/composition.rs:263
pub struct Linter(LinterLintEngine);  @f4ae3bd802:crates/ryo/crates/sdk/src/resources/linter/lint/types.rs:155
pub struct Workers(pub NonZeroUsize);  @f4ae3bd802:crates/ryo/crates/sdk/src/resources/linter/lint/types.rs:77
pub enum Cache {  @f4ae3bd802:crates/ryo/crates/sdk/src/resources/linter/lint/types.rs:94
pub struct WorkerPool(Arc<dyn SchedulerPort>, WorkerCount);  @f4ae3bd802:crates/ryo/crates/sdk/src/domain/capacity/types.rs:11
pub struct Languages(pub Vec<Language>);  @f4ae3bd802:crates/ryo/crates/sdk/src/domain/language/types.rs:135
pub fn run(  @f4ae3bd802:crates/ryo/crates/sdk/src/resources/query/run/entry.rs:18
```
**It was an edge in the owner's sense.** `lint_with` converted the request into core's request and called core's use
case, which the SDK composed per call in `composition.rs`:
```
pub(crate) fn build(  @f4ae3bd802:crates/ryo/crates/sdk/src/resources/linter/lint/composition.rs:103
            ryo_core::inbound::use_cases::lint::use_case::LintUseCaseImpl::new(  @f4ae3bd802:crates/ryo/crates/sdk/src/resources/linter/lint/composition.rs:154
```
with 14 ports (file analysis, cross-file, corpus, facts spill, language, matching, query language, memory, operators,
scheduler, workers, row sink, digest, store). The corpus was built inside that per-call composition, over the
request's root (`corpus::factory::ignore(root, listing, beside, skipped)`), and the cache directory beneath the root
was skipped there (`store_skip`). That per-call corpus is the reason for decision 3.
**Implements / calls.** Implements nothing. Depends on execution, execution_runtime, relations, core, operators,
matching, language, values, query_language (its `Cargo.toml`), plus rayon and thiserror.
**Who calls it** (by `git grep` of `idiomatic-sh-ryo-sdk` in every `Cargo.toml`): the command line (`query run` only),
the nine languages (the author surface in `ryo::languages` and the macros), the `benchmark` and `benchmark_contracts`
crates, the Ruff, Clippy and Oxlint benchmark packages, Tastemaker's core and command line. `Linter::new` outside the
SDK: Tastemaker's ryo adapter (`adapter.rs:111`), the benchmark's local engine adapter, four Ruff engine tests; the
free `lint`/`lint_with`: the command line's replay tests (15 call sites) and the same.
**Proof.** 30 `#[test]` functions: 29 unit tests in 17 `*_test.rs` files under `src/resources` and the trybuild runner
`tests/compile_fail.rs` (10 `fail_*` and 2 `pass_*` cases). Case tables: 109 `CaseName(..)` rows, of which the lint
entry has 29 (replay of cached absence and findings, observations, listed and given corpora, host relations,
refusals, budget) and the composition 8 (where the cache directory is skipped). 20 fixture files
(`lint/fixtures.rs` and 19 under `lint/fixtures/`, 1,874 lines). No acceptance feature.
**Tastemaker entries** (main `crates/ryo/tastemaker.toml`): 12 (the SDK's allowed internal packages), 25 (a language
depends on the SDK alone), 26-27 (benchmark data uses the SDK only), 39 `edges-apart`, 44 (no default trait methods in
`resources/languages`), 70 and 90-93 (the SDK's compile cases), 102 (`api.txt` is the `cargo public-api` baseline),
133 `crate-sdk`, 134 the `idiomatic.arch/sdk` spec with its re-export allowlist.

## 3. On the perf branch (2a207464c)
**Where.** Same crate; 6,809 Rust lines by the same count; `api.txt` 3,642 lines. The linter surface is the owner's
(24 Nov):
```
    pub fn new(languages: Languages, pool: WorkerPool, cache: Cache) -> Self {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/entry.rs:18
    pub fn lint(&self, requests: Vec<LintRequest>) -> Vec<Result<LintReport, LintError>> {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/entry.rs:31
impl Drop for Linter {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/entry.rs:82
impl From<super::types::Workers> for WorkerPool {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/entry.rs:92
impl From<Arc<rayon::ThreadPool>> for WorkerPool {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/composition.rs:68
pub struct Linter(LinterLintEngine);  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/types.rs:129
pub struct Workers(pub NonZeroUsize);  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/types.rs:72
pub enum Cache {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/types.rs:77
pub struct WorkerPool(Arc<dyn SchedulerPort>, WorkerCount);  @2a207464c:crates/ryo/crates/sdk/src/domain/capacity/types.rs:11
pub enum MemoryBudget {  @2a207464c:crates/ryo/crates/sdk/src/domain/capacity/types.rs:34
pub use lint::session::print_measurement;  @2a207464c:crates/ryo/crates/sdk/src/resources/linter.rs:12
```
**It is not an edge any more: it is the driver's front half.** `Linter` holds a session number, not a use case:
```
pub(crate) struct LinterLintEngine {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/types.rs:133
pub(crate) fn build_engine(languages: Vec<Language>, pool: WorkerPool, cache: super::types::Cache) -> LinterLintEngine {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/composition.rs:17
```
and `lint` walks each request, then runs a function pointer on the scheduler's owner thread:
```
        let (execute, _) = super::executor::in_use();  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/entry.rs:35
            match Arc::clone(&engine.scheduler).on_owner(&mut job) {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/entry.rs:62
pub(crate) type Execute = fn(usize, Vec<LintRequest>, &Languages, &Cache, NonZeroUsize) -> Vec<Result<LintReport, LintError>>;  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/executor.rs:14
static LINKED: OnceLock<(Execute, fn(usize))> = OnceLock::new();  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/executor.rs:17
pub(super) fn in_use() -> (Execute, fn(usize)) {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/executor.rs:23
```
The session lives in a thread-local map keyed by that number, which is why every call goes through `on_owner`:
```
    static SESSIONS: std::cell::RefCell<HashMap<usize, Live>> = Default::default();  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:4
    fn admit(&mut self, req: sdk::LintRequest, linked: &sdk::Languages, unbounded: bool) -> Result<RoundRequest, sdk::LintError> {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:477
```
About 2,000 of the 2,523 lines under `resources/linter` are session, admission, call cache, path globs, walk and
measurement (mapping findings, `trunk-kept-crates.findings.md`; confirmed by the file list: `session/entry.rs` 911,
`call_cache.rs` 338, `walk.rs` 124, `sdk_request.rs` 94, `measure.rs` 75, `rows.rs` 68, `executor.rs` 59). The SDK
imports the executor's internals directly:
```
ryo_executor = { package = "idiomatic-sh-ryo-executor", path = "../executor", version = "0.2.0" }  @2a207464c:crates/ryo/crates/sdk/Cargo.toml:26
```
plus core, execution, matching, language, values, query_language, relations; globset, blake3, rustc-hash, rayon,
thiserror, tracing.
`query run` builds a fresh `Linter` per call and reaches a crate-private entry:
```
    let linter = sdk::Linter::new(languages, pool.clone(), sdk::Cache::None);  @2a207464c:crates/ryo/crates/sdk/src/resources/query/run/entry.rs:69
    let report = sdk::lint::rows::answer(&linter, lint, usize::try_from(request.limit.0).unwrap_or(usize::MAX), memory)  @2a207464c:crates/ryo/crates/sdk/src/resources/query/run/entry.rs:70
pub(crate) fn answer(linter: &Linter, request: LintRequest, limit: usize, memory: MemoryBudget) -> Result<LintReport, LintError> {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/rows.rs:29
```
`query validate`, `query describe`, `operator list|describe`, `relation list|describe` are unchanged from main: each
composes a core or query_language use case and maps the SDK's public types to it (`validate/schemas.rs` 318 lines).
**Who calls it.** As on main, plus the lint images (`executor/benches/lint_image`, feature `bench-hooks`) and
Tastemaker's live benchmark project (`crates/tastemaker/benchmarks/project/live`, `bench-hooks`). `Linter::new` outside
the SDK: Tastemaker's adapter, the benchmark's local adapter (2), the lint image, 6 replay-test sites of the command
line. The Tastemaker adapter is the product caller:
```
                Ok(ryo::linter::Linter::new(  @2a207464c:crates/tastemaker/crates/core/src/outbound/analysis/adapters/ryo/adapter.rs:73
                    ryo::linter::WorkerPool::from(std::sync::Arc::clone(pool)),  @2a207464c:crates/tastemaker/crates/core/src/outbound/analysis/adapters/ryo/adapter.rs:75
        match ryo::query::validate(  @2a207464c:crates/tastemaker/crates/core/src/outbound/analysis/adapters/ryo/adapter.rs:89
            Err(ryo::query::ValidateError::Catalog { reason }) => {  @2a207464c:crates/tastemaker/crates/core/src/outbound/analysis/adapters/ryo/adapter.rs:99
```
**Proof.** 69 `#[test]` functions: 24 unit tests in `src` (one of them inside `session/entry.rs`), 42 in eight
integration files that drive `Linter::lint` with the toml language (`tests/lint.rs` 5, `lint_cache.rs` 7,
`lint_host.rs` 3, `lint_parse.rs` 2, `lint_provider.rs` 7, `lint_refusals.rs` 8, `lint_unadvertised.rs` 2,
`lint_unread.rs` 8), `tests/validate_contract.rs` 2, and the trybuild runner (10 `fail_*`, 4 `pass_*`). 63 case rows.
**Tastemaker entries** (tip `crates/ryo/tastemaker.toml`): 12, 22, 23-24, 36 `edges-apart`, 42, 67 and 84-87, 95
(`api.txt`), 123 `crate-sdk`, 124 the `sdk` spec; and outside Ryo, Tastemaker's own `library-surface` allowlist of the
`ryo::` names it may call (`crates/tastemaker/tastemaker.toml:10`).

## 4. What changed between them
126 commits touch `crates/ryo/crates/sdk` between the revisions (`git log --oneline f4ae3bd802..2a207464c`);
`git diff --stat` is 77 files, +4,313 -4,744. The ones that change the contract:
- `cd615d61c9` "the linter's public surface is new and lint; no retire, no free lint, executor hooks behind
  bench-hooks": `lint`, `lint_with`, `prepare` go; `lint(Vec<..>) -> Vec<Result<..>>` arrives (perf row 17). Owner, 24
  Nov: "no one is worjing on the sdk we can do whatever we want" (STATE.md 7215).
- The memory budget leaves `Linter::new` (owner, 24 Nov: "why does it need a memory budget"; STATE.md 7225).
- `ce8077f9af` "the session glue lives in the sdk crate over the SDK-free executor": the driver's session moves INTO
  the SDK. This is the change the draft undoes: an edge holding the driver.
- `5897aea060` the walk moves into the SDK; `73689ba059` lint publishes what the cache keeps before it returns (a host
  may exit without destructors; perf row 52).
- `f5cf2a74d3` `LintRequest.given_project_inventory` and `LintReport.dependencies` leave; with them 8 public types
  (`DependencyBytes`, `DependencyFingerprint`, `GivenProjectInventory`, `ProjectDependency`,
  `ProjectDependencyState`, `ProjectEntryObservation`, `ProjectEntryState`, `ProjectInventoryCompleteness`).
- `6cbc3df41a`, `dd44e732ae` the old engine's request and report conversions and the execution_runtime dependency go.
- `aee1b561d7` the session emits the `ryo::node_counters` tracing event the benchmark reads (section 8).
**Lost that should not have been.** (1) The edge's shape: on main the SDK composed a use case behind a contract; at
the tip it runs the driver itself through a fn-pointer link point. (2) main's 29 lint-entry case rows (cached absence
and contract-change replay, observations, budget refusal) and their 20 fixture files: the tip's 42 integration tests
re-point groups 1 to 6 of them (`105de2243b`), not all; which rows have no successor is not checked row by row.
(3) The `Uncompilable` refusal of `query validate` (validate sheet). Nothing of the public surface was lost that the
owner did not drop.

## 5. The drafted public interface
The listing is `api.txt`: **133 public items**, of which 22 are the SDK's own (10 types, 5 functions on them, 7 free
functions) and 111 are re-exports; 7 are UNPROVEN (they name `ryo_table`). Appendix A 3.19
(`PLAN.md:1458-1473`) is the claim; checked against the code it is right on every signature (the seven entries match
the tip verbatim) and right that the SDK "becomes thin". It did not say what the SDK still defines, what it
re-exports from whom, or how `Linter` holds the composed use case; this part does.

### 5a. The two calls (owner, 24 Nov), unchanged
`Linter::new(Languages, WorkerPool, Cache)` and `Linter::lint(&self, Vec<LintRequest>)` keep the tip's signatures
verbatim. What changes is inside: `Linter` holds one `ryo_lint::LintUseCaseImpl<..>` built in `new` with every
type parameter fixed at compile time (programs' `LoadUseCaseImpl` over ryoql's `ParseUseCaseImpl`, plan's
`LowerUseCaseImpl<CompareUseCaseImpl>` and operators' `PlanUseCaseImpl`; `Screen`; `ScanUseCaseImpl<Screen,
RunUseCaseImpl>`; `CrossFileUseCaseImpl<Languages, _>`; the corpus factory (decision 3); `factory::globset()`; the
cache adapter; the pool's scheduler; a store factory `move |workers| Stores::new(workers)` (perf row 54);
`CheckUseCaseImpl`). `lint` is one call: `self.0.lint(requests)`. So `executor.rs` (`LINKED`, `in_use`,
`link_executor`, `NEXT_SESSION`), the thread-local session map and the `on_owner` hop leave the SDK: the session's
state is the lint use case's own, privately (lint sheet decision 4). The tip's `impl Drop for Linter` goes: the
owner's words on 26 Nov were "no" to `retire` and an ordinary drop; dropping the composed use case is that drop.

### 5b. The edge's own choices: `Cache`, `WorkerPool`, `Workers`
These are what a caller decides and nothing below the edge may decide (the edge picks the cache mode: perf rows 2,
52; the binary picks the allocator: row 37). `Cache { None, At(PathBuf) }` stays `#[non_exhaustive]`, so a later
mode (the parked `Cache::Session` of row 53, or a findings-only mode) is additive. `new` maps `None` to
`ryo_cache::discard()` and `At(path)` to `ryo_cache::file(CacheLocation { directory, build })`, the build identity
taken here, once (cache sheet decision 2 says what goes in it). `WorkerPool` stays opaque. Changed inside: it holds
one concrete scheduler, `ryo_scheduler::rayon_over(Arc<rayon::ThreadPool>)`, whichever `From` made it
(`From<Workers>` builds the rayon pool first, cut to the machine with `ryo_scheduler::workers_to_start`), so the lint
use case's `Sd` parameter is one type and no `Arc<dyn SchedulerPort>` is needed. The pool's `WorkerCount` sizes the
stores (`Stores::new(count)`, perf row 54). `From<Arc<rayon::ThreadPool>>` is decision 2.

### 5c. What it re-exports, and from whom
The rule (decision 1): the SDK defines nothing a crate below defines; every type a user writes is re-exported from
its one definition, at the resource path users write today (`ryo::linter::LintRequest`, `ryo::query::ValidateError`).
That is what the lint listing already assumes (its decision 1 and "pub struct ryo::linter::Linter -- stays in the SDK
edge, over LintUseCaseImpl (sdk sheet)"). Re-exported from inbound/lint: the 44 request, report and error types, plus
`RowLimit` and `print_measurement`. From the concepts their fields name: `RuleId` (plan), `Glob` (corpus),
`ParameterName` (programs), `ObservationName` and `CheckError` (ryoql), `ProviderFault` (execute), `HostedRelation`
and the three host names (relations), `Languages` and `ProjectSearchScope` (language), `ColumnName` and `CellKind`
(table, UNPROVEN). Folded: `ProviderName` into `LanguageName` (one name of a language), `ObservationColumnName` into
`ColumnName`, `ObservationKind` into `CellKind` (the tip already converts one into the other), `HostDeclaration` and
`HostColumn` into `HostedRelation`, `ProviderCode` into `ProviderFault`. `query`: validate's three types, ryoql's
describe types and query-bundle constructors (decision 5), `StepCount` (plan), `RelationName`, `OperatorName`,
`RowCount` (table, UNPROVEN). `operator` and `relation`: 28 types re-exported from inbound/describe, which is not
written; they keep today's names and are listed by name only.

### 5d. What the SDK still defines
Ten types and seven free functions: `Linter`, `Cache`, `WorkerPool`, `Workers` (5a, 5b); `query::{run, validate,
describe}`, `operator::{list, describe}`, `relation::{list, describe}` with the tip's signatures verbatim; and the
`query run` table: `RunRequest`, `RunReport`, `Heading`, `Row`, `RunError`, `DEFAULT_LIMIT`. Those six stay here
because shaping a rows answer into headings and a place column (`query/run/entry.rs:46-99`) is the edge's
presentation of `LintUseCase::rows`, which answers a `RowsReport`; no crate below has a "heading". Changed in them:
`RunReport.total`/`shown` use the table's `RowCount`; `RunError` keeps the 7 variants the tip builds and drops
`Facts`, `Memory`, `Scheduler`, `Output`, `OutputClosed` (never constructed at the tip; the last two are matched only
by the command line's `remedy.rs`); and `Corpus` becomes `Io { what: Port, reason }`, because today every port's
failure is reported as the corpus's:
```
        LintError::Io { reason, .. } => RunError::Corpus { reason },  @2a207464c:crates/ryo/crates/sdk/src/resources/query/run/schemas.rs:56
```
`query::run` composes a `LintUseCaseImpl` over `Cache::None`, calls `rows(request, limit, &ryo_lint::memory::sysinfo(
memory))` once, and shapes the answer: the tip's `rows::answer` moves to `LintUseCase::rows` (lint sheet decision 2).
`query::validate` composes `ValidateUseCaseImpl::new(CheckUseCaseImpl::new(winnow, language catalog),
LowerUseCaseImpl::new(CompareUseCaseImpl::new()), vocabularies)`; the kind vocabularies come from `Languages`.

### 5e. The author surface leaves
`ryo::languages` keeps three re-exports for users who link languages (`LanguageConnection`, `LanguageName`,
`Recognition`; decision 4). The 569 author re-exports, the four macros and `languages::contract` go to
outbound/language, as its listing already records (`pub macro ryo_sdk::{link!, fills!, reads!} -- moved here`). That
is 757 of the tip's 3,642 api.txt lines and most of the SDK's public items.

### 5f. Features
`bench-hooks` forwards to `ryo_lint/bench-hooks` and adds one item, `ryo::linter::print_measurement`; it changes no
behaviour of `lint` (criterion 13: features additive). `fixtures` goes from the SDK: its users were the SDK's own
tests (`ryo::cases`), the conformance kit (to outbound/language) and core/values fixtures that leave with those
crates.

### 5g. Counts and what stops being public
Method: per resource, named types (struct, enum, trait, const, type) by last path segment, plus free functions, plus
inherent methods (trait-impl methods such as `clone`, `fmt`, `from` left out), plus macros, plus `pub use` lines, from
each side's `api.txt` (`cargo public-api` output; `#[doc(hidden)]` items such as `Languages::reads` are absent from it).

| Side | linter | query | operator | relation | languages | total |
|---|---|---|---|---|---|---|
| main | 78 (74 types, `lint`, `lint_with`, `new`, `prepare`) | 43 | 15 | 19 | 520 (495 `pub use`, 7 types, 15 methods, 3 macros) | **675** |
| tip | 70 (66 types, two paths of `print_measurement`, `new`, `lint`) | 43 | 15 | 19 | 592 (569, 7, 12, 4) | **739** |
| draft | 84 | 34 | 15 | 17 | 3 | **133** (22 own) |

The `# NO LONGER PUBLIC` section names each item. The ones that matter: the author surface (592 items, to
outbound/language); the `Drop` impl; the duplicate paths (`lint::session::print_measurement`, `query::{Cell, Slot}`,
the observation types under `query`); `ryo::cases`.

## 6. Proof it brings
| Proof | Source | Fate |
|---|---|---|
| the 42 lint integration tests (8 files) | tip `crates/ryo/crates/sdk/tests/` | **carried as they are**, paths of re-exported types updated. They are the end-to-end seam of the public surface with a real language (toml); inbound/lint cannot run them because it cannot compose (edges compose). `a_lint_leaves_what_the_cache_keeps_on_disk_before_it_returns` (`lint_cache.rs:109`) is perf row 52's gate |
| `a_rule_without_an_id_is_refused_where_the_id_is_made`, `a_glob_that_is_not_one_is_refused_where_it_is_made` | `lint_refusals.rs:93`, `:99` | **carried, and at risk**: plan drafts `RuleId(pub Box<str>)` and corpus `Glob(pub Box<str>)`, public fields with no check, so both would fail to compile as written (decision 5) |
| `validate_contract.rs` (2) | tip | **restated**: `ValidateError` becomes validate's `Refused { error }` (decision 1) |
| the unit tests of the four catalog verbs and `query run`/`validate` schemas (21) | tip `src/resources/**/*_test.rs` | `query run` schemas (1) and entry (2) **carried**; the other 18 test schema mapping that the draft deletes (re-export instead of mirror): **move** to inbound/validate and inbound/describe as their own seam tests, or go with the mapping |
| `walk_test.rs` (1), `entry_test.rs` (1: the linked executor counts rounds and closed sessions), the test in `session/entry.rs` | tip | the walk test to outbound/corpus; the linked-executor test **goes** with the link point (its property, one round per call, becomes a lint seam test); the session test to inbound/lint |
| main's 8 `store_skip` cases | main `crates/ryo/crates/sdk/src/resources/linter/lint/composition_test.rs` | **restored** here: the corpus factory closure is where the cache directory beneath the root is skipped (5a, decision 3); the tip moved the function to `walk.rs` and has no table for it |
| the trybuild runner and its 14 cases | tip `tests/compile_fail.rs`, `tests/ui/` | to outbound/language with the macros (its sheet section 6) |
| missing | — | a test that `Linter` and `WorkerPool` are `Send + Sync` (Tastemaker keeps the linter in a `OnceLock` shared across threads); a test that `lint(vec![])` answers `vec![]` without composing work; a `compile_fail` doctest that `Linter` exposes no third method (the tip has two for removed names, `types.rs:123-128`) |

## 7. Perf gains it carries
From `perf-gains.tsv`, the rows whose crate column names `edges/sdk`:

| Row | Gain (measured) | What in the draft keeps it |
|---|---|---|
| 2 | bench composition in the product; kept store holds the session (Django kept cache 690 KB -> 290 MB; product check 52.4 s and 291 MB before plain composition; defect ryo-fmx3.1.11.14 open) | `Cache` is the only knob and the edge's: `None` -> `discard()`, `At` -> `file(..)`. What the kept cache holds is decided behind `CachePort` (cache sheet); no plan or frontier column crosses the SDK. `#[non_exhaustive]` leaves room for row 53's findings-only mode without a break |
| 17 | the Tastemaker check submits its calls in rounds (Ryo part 29.013 -> 25.541 CPU-s) | `Linter::lint(Vec<LintRequest>) -> Vec<Result<..>>` verbatim; it is one call into `LintUseCase::lint(Vec<..>)`, which spreads the list over workers |
| 37 | allocator swap and per-worker pools (jemalloc -14 to -17%) | the SDK names no allocator (`#[global_allocator]` is in the command line, the benchmark, the lint images and Asymptote only, by `git grep` at the tip) and keeps none; the pools are the store's, sized per worker through `Stores::new(pool's WorkerCount)` |
| 52 | plain composition; teardown skipped by forgetting the services (CPU 52.4 -> 29.7 s; store 291 MB -> 2,477,287 bytes) | the documented promise of `lint` stays: when it returns, everything the cache keeps is on disk, so a host may `std::mem::forget` the linter (Tastemaker does: `std::mem::forget(composed);  @2a207464c:crates/tastemaker/cli/src/resources/workspace/check/command.rs:82`) or `process::exit` (the lint image: `exit_process(outcome)  @2a207464c:crates/ryo/crates/executor/benches/lint_image/main.rs:204`). No `Drop` impl of the SDK does work |
| 54 (crate column store; scheduler) | per-worker pool budget (dagster peak 1,257.5 -> 480.1 MiB, not isolated) | the SDK is where the count meets the store: `Stores::new(WorkerCount)` from the pool, once in `new` |

**Under this draft, which of the 48 history changes would have landed inside the SDK alone?** None of the measured
ones: the SDK holds no hot code. Row 2's edge part (choosing the mode) and row 52's (relying on "on disk before return")
are each a change of one caller (Tastemaker) plus the cache crate; row 17 is Tastemaker plus lint. **Which would still
cross into another crate through the SDK?** Only a new edge choice: a new cache mode (row 53) touches `Cache` and the
cache crate; a pool shared differently (row 13, parked) touches nothing here beyond what `WorkerPool` already carries.
At the tip the opposite held: rows 10 (call cache), 13 (workers), 14 (warm store) and 17 all edited
`sdk/src/resources/linter/lint/session/*`, because the driver sat in the edge.

## 8. Benchmarks that cover it
| Benchmark | What it calls in the SDK today (tip) | After the draft |
|---|---|---|
| lint images `vec`, `vec_ts`, `vec_rust` (`executor/benches/lint_image`, own workspace) | `ryo::linter::{Languages, Linter::new, lint, Workers -> WorkerPool, Cache::{None, At}, LintRequest, Corpus::Listed, RelativePath, ProjectSearchScope, SourceLanguage::Named, Rule, RuleApplicability, Glob::try_from, RuleId::try_from, Slot, Cell::{File, Span}, LintReport, LintError, print_measurement}`, `ryo::query::{QueryModuleCatalog, QueryModuleSource, RyoQlQuery::with_modules}`, `ryo::languages::Language`; feature `bench-hooks` (`ryo = { package = "idiomatic-sh-ryo-sdk", path = "../../../sdk", features = ["bench-hooks"] }  @2a207464c:crates/ryo/crates/executor/benches/lint_image/Cargo.toml:28`) | the same paths and calls; `ryo::languages::Language` becomes `LanguageConnection`; `Cell::Span`'s fields read the same (`start.0`, `end.0`). The images move to `tools/` with the benchmark (plan section 4) and keep calling only the SDK, so nothing internal they reach can move under them |
| the Tastemaker check (`tm` image, Tastemaker's live benchmark project, `bench-hooks`) | `Linter::new` over `WorkerPool::from(Arc<ThreadPool>)`, `lint` in rounds, `Cache`, `query::{validate, describe}`, `operator::{list, describe}`, `relation::{list, describe}`, `Languages`, `print_measurement`; `ValidateError::{Catalog, UnknownRelation}` matched | the same calls; the two matched variants become `ValidateError::Refused { error }` (decision 1) and Tastemaker's `library-surface` allowlist gets a new version naming the draft's paths |
| Ruff, Clippy, Oxlint linter benchmarks (through `benchmark`'s local engine adapter and `benchmark_contracts`) | `Linter::new` (2 sites), `lint`, `RuleId`, `RelativePath`, `SourceText`, `Skipped`, `SkipReason`, `ObservationProducer`, `HostDeclaration`/`HostColumn`/`HostKind` (Ruff), `query::validate`, `RyoQlQuery::try_from`, `QueryModuleCatalog::try_new`, `ryo::languages::{LanguageName, SourcePath, PrimaryFileRole}` | the same through the SDK, `HostDeclaration` written as `HostedRelation`; `SourcePath` and `PrimaryFileRole` are author-surface names the draft no longer re-exports (decision 4) |
| the benchmark's evidence reader | the tracing target string `"ryo::node_counters"` (`const NODE_COUNTERS_TARGET: &str = "ryo::node_counters";  @2a207464c:crates/ryo/crates/benchmark/src/outbound/engine/adapters/local/writer.rs:24`), emitted by the SDK's session (`tracing::event!(name: "run", target: "ryo::node_counters", tracing::Level::INFO, runs = runs, nodes = nodes);  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:405`) | emitted by inbound/lint with the same target and fields: a string contract between two crates that no type holds; it must be named on the lint sheet and in the benchmarks sheet |
| `sdk/benches/performance.rs` | an empty divan main at both revisions | nothing to keep |

## 9. Checks
IMPORTS: lint, validate, describe, programs, ryoql, plan, operators, relations, table, screen, execute, project, language, store, cache, corpus, scheduler
CROSSED: per request

- Imports: an edge imports inbound and "the factories of the crates they compose" (Appendix A section 2). Every Ryo
  crate above is either inbound or composed here; `table` and `relations` are named only through re-exports. Outside
  Ryo: rayon (for `From<Arc<rayon::ThreadPool>>`, decision 2) and thiserror (`RunError`); globset, blake3, rustc-hash
  and tracing leave with the code that used them (to corpus, cache, lint and lint). Not `ryo_symbols`, not the
  languages.
- No layout word in a signature: none of the 133 items names a column store, row slot, `Vec<Vec<u64>>`, `Batch`,
  `World` or `Live`.
- Crossed: once per `lint` call into `LintUseCase::lint` (and once per `query`/`operator`/`relation` call). No
  per-file, per-step or per-row crossing passes through the SDK; the composed use case's type is fixed at compile time,
  so even that one crossing is not `dyn`.
- Direction rule to update: `edges-apart` (`crates/ryo/tastemaker.toml:36`) says the command line depends on no SDK
  crate, yet `ryo_sdk = { package = "idiomatic-sh-ryo-sdk", path = "../crates/sdk" }  @2a207464c:crates/ryo/cli/Cargo.toml:22`.
  How the rule passes today is not checked; after the draft the command line composes `inbound/lint` itself and drops
  the SDK (command-line sheet).

## 10. Decisions for the owner
1. **Re-export, or keep the SDK's own public copies?** Example: the tip's `query::ValidateError` is a 7-variant mirror
   of query_language's `TypecheckError`, kept in step by `validate/schemas.rs` (318 lines); Tastemaker matches
   `ValidateError::Catalog` (`adapter.rs:99`). Why it matters: copies are a second definition of each concept
   ("one definition per concept") and the mapping code is where drift lives; re-exports make every inbound type a
   public promise of the SDK, so a change in inbound/lint is an SDK semver change. What we get by re-exporting: the
   schema mapping goes (validate 318, relation 157, operator 140, lint 59, describe 52: about 700), 111 names have one owner each, and the lint and validate sheets already assume it. One
   inconsistency must be settled either way: lint's `LintError::Rule` carries ryoql's `CheckError` while
   `query::validate` answers `ValidateError::Refused { error: RefusedQuery }`, two shapes for one refused query.
   **Pick: re-export, and lint's `Rule` carries `RefusedQuery` as validate does** (a change for the lint sheet).
2. **Keep `From<Arc<rayon::ThreadPool>> for WorkerPool`?** Example: Tastemaker builds the linter over its own pool
   (`adapter.rs:75`), so one pool serves the whole check. Why: it puts a rayon type in the SDK's public API (the
   scheduler sheet flags it), so a rayon major version is an SDK major version. What we get by keeping it: one pool
   per process for every host that already has one; dropping it means a second pool beside Tastemaker's. **Pick:
   keep it, and make it the one path (`From<Workers>` builds a rayon pool and goes through it), so `WorkerPool` holds
   one concrete scheduler and no `dyn`.**
3. **The corpus is per request; lint's `LintParts` takes one.** Example: `ryo_corpus::factory::ignore(RootPath,
   CorpusListing, Vec<CorpusFile>, SkippedDirectories)` needs the request's root, listing and given files, and main
   built it inside the per-call composition (`composition.rs:103`); the drafted `LintParts { corpus: Co, .. }` is fixed
   at `Linter::new`, before any request exists. Why: as drafted the SDK cannot compose lint at all. What we get: the
   SDK hands lint a corpus factory, `Fn(&RootPath, CorpusListing, Vec<CorpusFile>) -> Co`, as `LintParts` already does
   for stores (`St: Fn(WorkerCount) -> S`), and the cache directory skip (`store_skip`) lives in that closure, at the
   edge where the cache directory is known. **Pick: a factory in `LintParts` (a change for the lint sheet).**
4. **What `ryo::languages` keeps.** Example: Tastemaker's command line calls `linked().reads(&scenario) !=
   ryo::languages::Recognition::Declines` (`create/composition.rs:45`) and `linked().skipped()`; the benchmark calls
   `language.primary_role(ryo::languages::SourcePath(..))`. `reads` and `skipped` are `#[doc(hidden)]` inherent
   methods of the SDK's `Languages`, which moves to outbound/language, whose listing does not carry them. Why: an
   inherent method can only live with its type. What we get: users link languages through three names
   (`LanguageConnection`, `LanguageName`, `Recognition`) and the language crate owns everything about a language.
   **Pick: `reads` and `skipped` become public methods of `ryo_language::Languages` (a change for the language
   sheet); `SourcePath` and `PrimaryFileRole` are not re-exported, and the benchmark asks `LanguageConnection` through
   a method the language sheet names, or depends on the language contract as a tool may.**
5. **One home for query text and for the checked names.** Example: `RyoQlQuery::try_from`, `RyoQlQuery::with_modules`,
   `QueryModuleCatalog::try_new` and `QueryModuleSource::try_new` are called by all three linter benchmarks, the lint
   images, the command line and Tastemaker; the ryoql listing has `QueryBundle` opaque with no constructor and no
   catalog types. Likewise `RuleId::try_from` refuses blank text and `Glob::try_from` refuses a bad glob today, but
   plan and corpus draft `RuleId(pub Box<str>)` and `Glob(pub Box<str>)`. Why: a user must be able to make a query,
   an id and a glob, and an invalid one must be refused where it is made (two carried tests prove it). What we get:
   the constructors live with the type they make, the SDK re-exports them. **Pick: ryoql defines `RyoQlQuery` (the
   name users write; `QueryBundle` renamed) with its constructors and the catalog types; plan's `RuleId` and corpus'
   `Glob` get private fields with `TryFrom<&str>` and their schema errors (changes for three sheets).**

## 11. Not checked
- **Cross-sheet names that disagree**, found while choosing what to re-export (none is mine to settle): lint names
  `ryo_relations::LanguageName` while outbound/language defines `ryo_language::LanguageName`; lint names
  `ryo_corpus::GlobsUseCase` while corpus defines `PathGlobsUseCase`; lint defines `ExecutablePermission` and corpus
  defines another; lint's `RowTotal(pub usize)` and table's `RowCount(pub usize)` are one concept; relations defines
  none of `HostedRelation`, `RelationName`, the three host names or `KindVocabularies`, which lint, ryoql, validate and
  this listing name; ryoql defines neither `ObservationName` nor `Production`/`ProductionName`/`ProductionForm`.
  This listing writes each as the defining sheet's draft would have to; check-types will flag them.
- inbound/describe has no sheet: the 28 `operator` and `relation` types are listed by today's names only, and whether
  describe defines them under those names (`DescribeRequest` exists twice, once per resource) is open.
- Whether `ryo::query::ObservationScope` has any reader outside the SDK (only the SDK's own use at the tip was seen).
- Whether `QueryBundleError` already refuses a blank text, so that `RyoQlQuerySchemaError` can go.
- Which of main's 29 lint-entry case rows and 20 fixtures have no successor among the tip's 42 integration tests: the
  rows were listed, not matched one by one.
- The exact concrete type of `Linter`'s field: it depends on lint's `LintParts` (decision 3) and on whether the cache
  is one private enum over `discard` and `file` (assumed) or a `Box<dyn CachePort>` (crossed once per call, so either
  meets the rule).
- How `edges-apart` passes while the command line's `Cargo.toml` names the SDK.
- The SDK's README and doctests were not read; the `idiomatic.arch/sdk` spec's re-export allowlist (tip line 124) was
  read only in its first 200 characters and must get a new version listing the draft's 111 re-exports.
- Build 133 was not read for this crate (the SDK there did not name the executor, per the plan; not re-verified).
- Line counts are mechanical (`wc -l` of `git show` output) and the public-item counts follow the method of 5g; a
  different method gives different totals.
- Quotes: every `@rev:path:line` in this sheet was re-checked with `git show "REV:PATH" | sed -n 'LINEp'` (count in
  the final report); no check script (`checks/check-quotes.py`, `check-types.py`, `check-graph.py`) was run.
