# inbound/lint
STATUS: drafted (28 Nov 2026; not reviewed, not agreed). Lines naming a `ryo_table` type are UNPROVEN until
`domain/table/spike.md` records a pass.

Revisions: MAIN = f4ae3bd802 (release 0.2.0, live). TIP = 2a207464c (perf branch, build 139). B133 = f0219b3b3 (not
needed here: main already has the designed lint use case). Read only with `git -C /workspaces/idiomatic-public-benchmark
show "REV:PATH"`. Every `@rev:path:line` quote below was re-checked by script (section 11).

## 1. Owns
One lint call: a list of rule requests in, one report (or one refusal) per request out, in request order.

One thing. Everything it does is in service of that call: turn the requests into one load, ask the cache first, spread
the files over workers, collect each file's answers and the cross-file answers, decode them into findings and split
them back per request. What it calls (programs, screen, execute, project, corpus, cache, scheduler) each owns its own
thing; lint owns none of their work. Composition (the worker pool, the walker, the store, the cache adapter) stays at the
edges, as the owner decided ("the edge owned by the sdk composes"; plan section 8.2, Agent G).

## 2. On main (f4ae3bd802)

**Where.** Two places, one designed and one wiring:
- `crates/ryo/crates/core/src/inbound/use_cases/lint/` (1,479 Rust lines without tests: `use_case.rs` 384,
  `service.rs` 400, `errors.rs` 348, `types.rs` 319, `interface.rs` 17, `constants.rs` 11), with core's
  `inbound/use_cases/open/` (113 lines) called before it, and two outbound ports of core it uses: `outbound/memory`
  (125 lines without tests) and `outbound/row_sink` (190).
- The SDK's `crates/ryo/crates/sdk/src/resources/linter/lint/` (1,696 lines without tests: `schemas.rs` 457,
  `types.rs` 382, `composition.rs` 340, `errors.rs` 230, `entry.rs` 150, `connections.rs` 121, `lint.rs` 16), which
  converts the SDK's types into core's and composes core's use case.

**The contract, verbatim.**
```
pub trait LintUseCase: Send + Sync {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/lint/interface.rs:7
fn lint(&self, request: LintRequest) -> Result<LintReport, LintError>;  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/lint/interface.rs:16
pub struct LintUseCaseImpl {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/lint/use_case.rs:66
pub fn new(  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/lint/use_case.rs:111
pub fn compiling_into(mut self, plans: CompiledPlans) -> Self {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/lint/use_case.rs:162
pub fn counting(mut self, observer: fn() -> Arc<dyn ObserverPort>) -> Self {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/lint/use_case.rs:170
pub struct LintRequest {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/lint/types.rs:36
pub answer: AnswerShape,  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/lint/types.rs:48
pub struct LintReport {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/lint/types.rs:143
pub enum LintError {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/lint/errors.rs:18
pub const DEFAULT_LIMIT: u64 = 20;  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/lint/constants.rs:4
```
`new` takes 14 positional arguments, all but `workers` an `Arc<dyn ..>`: `file_analysis, cross_file, corpus, facts,
language, matching, query_language, memory, operators, scheduler, workers, rows, digest, store` (lines 111-126). The
answer is written to the rows port (`RowSinkPort`), not returned; `LintRequest.answer: AnswerShape` chooses findings or
rows. One request per call.

Called before it, on every SDK lint, and its two ports:
```
pub trait OpenUseCase: Send + Sync {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/open/interface.rs:7
fn open(&self, request: OpenRequest) -> Result<OpenReport, OpenError>;  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/open/interface.rs:13
pub fn new(language: Arc<dyn LanguagePort>, memory: Arc<dyn MemoryPort>) -> Self {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/open/use_case.rs:30
pub trait MemoryPort: Send + Sync {  @f4ae3bd802:crates/ryo/crates/core/src/outbound/memory/interface.rs:7
fn budget(&self) -> Result<MemoryBytes, MemoryError>;  @f4ae3bd802:crates/ryo/crates/core/src/outbound/memory/interface.rs:15
fn resident_bytes(&self) -> Result<MemoryBytes, MemoryError>;  @f4ae3bd802:crates/ryo/crates/core/src/outbound/memory/interface.rs:22
pub fn sysinfo(  @f4ae3bd802:crates/ryo/crates/core/src/outbound/memory/factory.rs:8
pub trait RowSinkPort: Send + Sync {  @f4ae3bd802:crates/ryo/crates/core/src/outbound/row_sink/interface.rs:10
```
`open` checks that the language catalog answers and that the machine has the budget, then wraps the session (56 lines,
read in full).

**The SDK's entries over it** (each one request per call; the linter holds a memory budget):
```
pub fn lint(  @f4ae3bd802:crates/ryo/crates/sdk/src/resources/linter/lint/entry.rs:24
pub fn new(languages: Languages, pool: WorkerPool, memory: MemoryBudget, cache: Cache) -> Self {  @f4ae3bd802:crates/ryo/crates/sdk/src/resources/linter/lint/entry.rs:46
pub fn prepare(  @f4ae3bd802:crates/ryo/crates/sdk/src/resources/linter/lint/entry.rs:86
pub fn lint_with(linter: &Linter, mut request: LintRequest) -> Result<LintReport, LintError> {  @f4ae3bd802:crates/ryo/crates/sdk/src/resources/linter/lint/entry.rs:114
let opened = super::composition::build_open(budget)  @f4ae3bd802:crates/ryo/crates/sdk/src/resources/linter/lint/entry.rs:122
ryo_core::inbound::use_cases::lint::use_case::LintUseCaseImpl::new(  @f4ae3bd802:crates/ryo/crates/sdk/src/resources/linter/lint/composition.rs:154
```

**Implementors and callers.** `LintUseCaseImpl` is the one implementor. Callers of `ryo_core`'s lint module outside
core: the SDK 12 files, the command line 8 files in `src` (its `query run` composes `LintUseCaseImpl` itself:
`service: LintUseCaseImpl::new(  @f4ae3bd802:crates/ryo/cli/src/resources/query/run/composition.rs:68`) and 10 in
`tests`. `OpenUseCase`: the SDK only (`composition.rs:7`, `entry.rs:5`).

**Public items** (counted by `^\s*pub (const fn|fn|struct|enum|trait|type|const|static|use|mod) ` per file, tests and
fixtures excluded): core lint 21 (types 14, use_case 4, interface 1, errors 1, constants 1), open 6, memory 3,
row_sink 11: **41**; the SDK's lint module **37** (types 26, entry 4, `pub mod` 3, errors 2, schemas 2).

**Proof.** Core lint: 19 `#[test]` driving 97 case rows (`use_case_test.rs` 10, `guards` 10, `inapplicable_identity` 6,
`lifecycle` 23, `project_budget` 6, `refusals` 4, `rules` 19, `runs` 16, plus 3 in the contract fixture), 2 scenario
outlines with 4 examples (`acceptance.feature`), and a contract test any implementor runs:
`pub fn check(subject: &impl LintUseCase) {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/lint/fixtures/contract.rs:122`.
Open: 3 `#[test]`, 2 examples. Memory: 2 (sysinfo adapter) + 3 (fake). Row sink: 1 + 3. SDK lint: 6 `#[test]` over 47
case rows (`entry_test.rs` 25, `schemas_test.rs` 14, `composition_test.rs` 8).

**Tastemaker entries** (`crates/ryo/tastemaker.toml` at main): line 123, `idiomatic.arch/pipeline` over
`crates/core/src/inbound/use_cases/lint` ("Retained work preserves source freshness, bounded resource lifetimes and exact
serial/parallel outcomes"); line 94, `lint-allocation-test` (`crates/core/tests/allocation.rs`); line 95,
`lint-call-count-test` (`cli/tests/call_count.rs`: "a port is called for a file, never for a row"); the SDK spec's
re-export list naming `ryo::linter::{LintRequest, Rule, RuleId, Glob, Corpus, MemoryBudget, WorkerPool, lint, ..}`
(lines 137-157).

## 3. On the perf branch (2a207464c)

**Where.** The working lint is the SDK's: `crates/ryo/crates/sdk/src/resources/linter/lint/` (2,523 lines without tests;
`session/entry.rs` 911, `types.rs` 347, `session/call_cache.rs` 338, `errors.rs` 159, `walk.rs` 124, `entry.rs` 97,
`session/sdk_request.rs` 94, `composition.rs` 80, `session/measure.rs` 75, `rows.rs` 68, `session.rs` 62,
`executor.rs` 59, `schemas.rs` 59, `session/modules.rs` 23, `lint.rs` 18, `connections.rs` 9). Core keeps the trait
with no implementor and a second family of types (`core/src/inbound/use_cases/lint/`: 541 lines; `types.rs` 259,
`errors.rs` 215, `coordinates.rs` 46, `interface.rs` 17, `constants.rs` 4), and the memory and row-sink ports unchanged
(the memory fake deleted).

**The interface actually called** is the SDK's, fixed by the owner on 24 and 26 Nov ("FINAL SDK LINTER SURFACE: exactly
Linter::new(languages, pool, cache) and Linter::lint(&self, Vec<LintRequest>)", engine/STATE.md:10923):
```
pub fn new(languages: Languages, pool: WorkerPool, cache: Cache) -> Self {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/entry.rs:18
pub fn lint(&self, requests: Vec<LintRequest>) -> Vec<Result<LintReport, LintError>> {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/entry.rs:31
pub struct Linter(LinterLintEngine);  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/types.rs:129
pub struct LintRequest {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/types.rs:169
pub struct LintReport {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/types.rs:281
pub(crate) rows_total: usize,  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/types.rs:292
pub enum LintError {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/errors.rs:11
pub enum ProviderCode {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/errors.rs:141
```
`LintError` has 16 variants (errors.rs:14-130). `Linter::lint` walks each walked corpus itself
(`match super::walk::listed(request, engine) {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/entry.rs:42`),
then runs the whole list on the pool's owner thread
(`match Arc::clone(&engine.scheduler).on_owner(&mut job) {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/entry.rs:62`)
through a function pointer a test may replace:
```
pub(crate) type Execute = fn(usize, Vec<LintRequest>, &Languages, &Cache, NonZeroUsize) -> Vec<Result<LintReport, LintError>>;  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/executor.rs:14
LINKED.get().copied().unwrap_or((super::session::execute_round, super::session::release_session))  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/executor.rs:24
```

**The session it reaches** (all crate-private; this is what the crate is built from):
```
static SESSIONS: std::cell::RefCell<HashMap<usize, Live>> = Default::default();  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:4
pub(crate) fn execute_round(  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:37
pub(crate) fn execute_rows(  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:16
let result = self.admit(request, linked, true);  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:382
fn admit(&mut self, req: sdk::LintRequest, linked: &sdk::Languages, unbounded: bool) -> Result<RoundRequest, sdk::LintError> {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:477
self.batch.prepare_rules(items);  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:605
self.batch.epoch_with_store(&self.languages, &mut self.state, Some(store));  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:392
fn deliver(&mut self, request: Pending, linked: &sdk::Languages) -> Reply {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:726
let receipt = Batch::answer(&self.batch.work[*i], &self.state.answers)  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:736
tracing::event!(name: "run", target: "ryo::node_counters", tracing::Level::INFO, runs = runs, nodes = nodes);  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:405
match validate(linked, ValidateRequest { query, hosted, observations: Vec::new() }) {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:200
completed_calls: [VecDeque<(CallKey, sdk::LintReport)>; 2],  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:103
let memoize = unbounded && self.store.is_none() && authority == Authority::Given  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:529
&& let Some((_, report)) = self.completed_calls[memo_pool].iter().find(|(saved, _)| saved == &memo)  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:549
let proof = self.store.as_mut().unwrap().request_inputs(  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:585
pub(super) struct CallCache {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/call_cache.rs:117
pub(super) fn query(&mut self, query: &str) -> (Rc<str>, stored::Key) {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/call_cache.rs:216
pub(crate) fn answer(linter: &Linter, request: LintRequest, limit: usize, memory: MemoryBudget) -> Result<LintReport, LintError> {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/rows.rs:29
let port = ryo_core::outbound::memory::factory::sysinfo(budgeted(memory));  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/rows.rs:33
pub(super) fn listed(request: LintRequest, engine: &LinterLintEngine) -> Result<(LintRequest, Vec<(RelativePath, Box<str>)>), LintError> {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/walk.rs:18
let walk = ryo_core::outbound::corpus::factory::ignore(  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/walk.rs:38
pub(super) fn naming_unread(mut report: LintReport, failed: Vec<(RelativePath, Box<str>)>) -> LintReport {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/walk.rs:116
pub fn print_measurement() {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session.rs:42
pub(crate) fn print_measurement() {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/measure.rs:47
```
In words: a thread-local map holds one `Live` session per `Linter` across calls (its `Batch`, kept `Store`, file
versions, glob filters, call memo). Per request, `admit` lowers the request's rules once (`prepare_rules`), types each
rule's parameters (spread over workers when 128 or more), keys it for the kept store, registers each rule with the
executor's `Batch` (which admits it a second time: bead ryo-fmx3.1.13.19); then one `epoch` runs every request's files;
`deliver` renders each rule's receipt into findings; the round ends with the `ryo::node_counters` event. A refused rule
is worded by calling the SDK's own `query::validate` (entry.rs:200). `query run` reaches the same session through
`rows::answer` (`execute_rows`), the only place the memory port is used.

**Callers.** `Linter::new`/`Linter::lint` outside the SDK's `src`: the command line's replay tests (5 files plus 1
fixture), the SDK's tests (8), the benchmark crate (1, `outbound/engine/adapters/local/adapter.rs`), the lint image (1),
Tastemaker's analysis adapter (1) and its check worker (1). `rows::answer`: the SDK's `query::run` only
(`query/run/entry.rs:70`). Core's dead family is still read by the command line (`remedy.rs:71`, `command.rs:24`, the
replay fixtures), and core's `FindingCoordinates` by the executor and the SDK's language contract:
```
pub struct FindingCoordinates(LineStarts);  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/lint/coordinates.rs:11
pub use ryo_core::inbound::use_cases::lint::coordinates::FindingCoordinates;  @2a207464c:crates/ryo/crates/executor/src/languages.rs:74
impl Remedy for ryo_core::inbound::use_cases::lint::errors::LintError {  @2a207464c:crates/ryo/cli/src/outcome/remedy.rs:71
#[arg(long, default_value_t = ryo_core::inbound::use_cases::lint::constants::DEFAULT_LIMIT)]  @2a207464c:crates/ryo/cli/src/resources/query/run/command.rs:24
```

**Public items** (same count): core lint 20 (types 15, coordinates 2, interface, errors, constants 1 each), memory 3,
row_sink 11: **34**; the SDK's lint module **38** (types 26, `pub mod` 5, entry 2 (`Linter::new`, `Linter::lint`), errors 2, schemas 2,
`print_measurement` 1). The SDK's `linter` module re-exports 31 items of the lint module and 36 lines of its own
`domain` types (`sdk/src/resources/linter.rs`).

**Proof.** The SDK: 42 `#[test]` in `sdk/tests/lint*.rs` (lint 5, lint_cache 7, lint_host 3, lint_parse 2, lint_provider
7, lint_refusals 8, lint_unadvertised 2, lint_unread 8), all through `Linter`; 3 in `src` (`entry_test.rs` 1, `walk_test.rs`
1, `session/entry.rs` 1). Core: 1 (`coordinates.rs`). Memory 2, row sink 4. No scenario.

**Tastemaker entries** (`tastemaker.toml` at the tip): line 113, the `pipeline` spec still names
`crates/core/src/inbound/use_cases/lint`, which holds no use case any more (stale); line 88, `lint-allocation-test`
names `crates/core/tests/allocation.rs`, absent at the tip (stale); the call-count entry is gone; the SDK spec's list
(lines 127-147) still names `ryo::linter::lint`, which is now the module, not the function.

## 4. What changed between them

- **Removed.** Core's implementation and its proof: `ff4533b99a` (26 Nov, "core: the old engine arms are deleted")
  took `use_case.rs`, `service.rs`, the 19 tests, 97 case rows, 2 scenario outlines and the contract fixture. The SDK's
  entries `lint`, `lint_with`, `prepare` and the memory argument of `Linter::new` went in `cd615d61c9` (26 Nov, "the
  linter's public surface is new and lint"), on the owner's decision of 24 Nov. The SDK's 6 unit tests (47 rows) went in
  `ce8077f9af`. `cli/tests/call_count.rs` went in `791c136b69`. `OpenUseCase` and the memory fake went in `35b6eaf0cf`
  (27 Nov, "stage 2: delete what audit B lists (modules nothing calls ...)").
- **Added.** The session over the executor, moved into the SDK in `5094c6add3` and `ce8077f9af` (26 Nov); the linked
  executor point in `f186cf7a0e`; `Vec` in, `Vec` out came with the Tastemaker rounds (`db28b15d9`, 24 Nov; perf row 17);
  the per-request hashing and whole-call memo (`b640148bed`, 25 Nov; perf row 10); the walk listed by the SDK
  (`5897aea060`); `query run`'s rows path made crate-private to the SDK (`2bde413a94`: "no hidden Linter method, no
  hidden LintReport field"). 53 commits touch the SDK's linter folder after the branch left main.
- **Changed.** One request per call became a list per call; the answer is returned, not written to a rows port; the
  memory budget left the linter and survives only on the rows path; the rule's refusal is worded through `validate`
  instead of core's `Query`/`UnknownKind` variants (21 core variants against the SDK's 16).
- **Lost that should not have been.**
  1. A lint use case behind a contract. Main had `LintUseCase` with one implementor and a contract test; the tip has the
     trait with none and puts the whole call in a 911-line SDK file over the executor's `Batch`, reached through a
     thread-local map.
  2. Its proof: 19 tests over 97 rows, 4 scenario examples, the contract fixture, the call-count test.
  3. `OpenUseCase`, deleted on "nothing calls it" evidence the day the owner stopped exactly that ("contracts are not
     dead code", memory of 27 Nov). Its fate is decision 5.
  4. The memory port's fake, deleted with it, so the port has no test double at the tip.

## 5. The drafted public interface        (explains every line of api.txt that is changed or new)

The listing has 121 lines: 62 public items and 59 enum variants (76 `[perf]`, 34 `[changed]`, 9 `[main]`, 2 `[new]`).
Against 41 + 37 on main and 34 + 38 at the tip.

### 5a. The claim it starts from, checked

Appendix A 3.16 drafts `LintUseCase::lint(Vec<LintRequest>) -> Vec<Result<..>>`, `LintUseCaseImpl::new(languages,
scheduler: Arc<dyn SchedulerPort>, workers, corpus: Arc<dyn CorpusPort>, cache: Arc<dyn CachePort>, memory: Arc<dyn
MemoryPort>)`, its own ports `MemoryPort` and `RowSinkPort`, and the SDK's one family of types. Checked:
- **Right:** the trait's name and bounds; the list in, list out (the tip SDK's, entry.rs:31); one family of types, the
  SDK's (core's at the tip is read only by the command line); `MemoryPort` as lint's port.
- **Wrong, 1: `RowSinkPort` is not lint's.** At the tip nothing in the engine writes rows to it; its one implementor
  and only caller is the command line's output (`cli/src/resources/query/run/rendering.rs:33`, `cli/src/output/schemas.rs:13`).
  Lint returns reports. Decision 5.
- **Wrong, 2: the constructor misses what the call needs.** It names no `load`, no screen, no file run, no cross-file
  phase and no store. The draft hands in every use case lint calls (5c).
- **Missing:** `query run`'s rows answer (the tip's `rows::answer`), which the command line also needs and which an
  edge cannot reach inside another edge (decision 2); wording a refused rule without importing inbound/validate.

### 5b. The calls: `lint` and `rows`

```
pub trait LintUseCase: Send + Sync {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/lint/interface.rs:7
    fn lint(&self, requests: Vec<LintRequest>) -> Vec<Result<LintReport, LintError>>;   @new
    fn rows<M: MemoryPort>(&self, request: LintRequest, limit: RowLimit, memory: &M) -> Result<RowsReport, LintError>;   @new
}
```
`lint` is the tip SDK's `Linter::lint` signature on core's trait. One call does, in order:
1. Per request: check what needs no file (root is text, the named source language is linked once, host relations
   unique and well formed, globs compile), and list a walked corpus through `CorpusPort` (today `walk.rs:18`, in the SDK).
2. Per request: ask the cache for a whole-request answer (`CachePort`, Appendix A 3.13 `AnswerKey::Request`); a full
   hit is answered without loading anything (perf row 14).
3. Once per call: the rules of every remaining request, deduplicated by text, parameters and observations, become ONE
   `ryo_programs::LoadRequest`, and `LoadUseCase::load` runs once (rows 6, 8, 13). A refused rule refuses its request
   (`RuleRefusal` to `LintError`); its wording comes from ryoql's `CheckUseCase::check`, called only for a refused
   rule (today `entry.rs:200` calls the SDK's `validate`).
4. Once per language per call: `ScreenUseCase::prepare` (row 19).
5. Per distinct file version over all requests: read once through `CorpusPort::read` (rows 50, 51), ask the cache by
   file (`AnswerKey::File`), and compute its enabled rules as the union over the requests that list it (their `files`,
   `include`, `exclude` through `GlobsUseCase`, their source language). Lint keeps the map from (request, rule) to the
   load's `RuleOrdinal` and from a file version to the requests that list it (the tip's `ReportSlots`, `PairAliases`,
   which the execute sheet hands to lint).
6. `SchedulerPort::run`, one job per worker: `stores(count)` makes the worker's store (row 54), `ScanUseCase::open`
   once, `scan` per file, decode the file's `FileAnswers` into findings per (request, rule) while the worker's store and
   `ScanScratch::symbols` are live (execute decision 5), hand each `Carried` to `CrossFileUseCase::receive`, `close`.
7. Once: `CrossFileUseCase::run` over the workers' hand-overs; decode its answers with its frozen dictionary.
8. Before returning: `CachePort::keep`, so everything kept is on disk when `lint` returns (row 52; tip
   `a_lint_leaves_what_the_cache_keeps_on_disk_before_it_returns`).
9. Emit the `ryo::node_counters` event from the summed `ScanCounters::{step_runs, nodes_made}` (same target, name and
   fields as entry.rs:405); split the reports per request, in request order; name unread files.

`rows` is the same run for one request with every row counted and the first `limit` decoded, held against `memory`
before any file is read and after (today `rows.rs:29-67`; `TooLarge`, `MemoryBudgetExceeded`). It is the one place the
memory port is crossed, and the port is handed to the call, not to the constructor, because only `query run` has a
budget (decision 2).

### 5c. How it is built

`LintUseCaseImpl::new(LintParts { languages, load, screen, scan, cross_file, corpus, globs, cache, scheduler, workers,
stores, check })`. Each part is a generic parameter fixed at compile time, never `dyn`: `scan`, `corpus` and `cache` are
crossed once per file, the rest once per call or per worker. The parts struct replaces main's 14 positional `Arc<dyn>`
arguments (review criterion 12: no long positional lists). `stores: Fn(WorkerCount) -> S` is the store's constructor,
called once per worker of a call with the call's worker count: that is the tip's
`let _budget = finish::PoolBudget::for_workers(shards.len());  @2a207464c:crates/ryo/crates/executor/src/check_session/worker.rs:194`
(row 54) moved to the edge's composition; the store sheet may replace the closure with a named factory. The
`Debug` impl is main's (`finish_non_exhaustive`).

The edges build it: the SDK's `Linter::new(languages, pool, cache)` composes one over the pool's scheduler, the walker,
the cache adapter `cache` chooses, the store and the domain use cases, and `Linter::lint` forwards to `lint` (the owner's
surface unchanged); the SDK's `query::run` and the command line's `query run` compose one and call `rows` with
`memory::sysinfo(budget)`.

### 5d. The types

- **Request** (`LintRequest`, `Rule`, `Corpus`, `CaseSource`, `SourceText`, `RelativePath`, `ExecutablePermission`,
  `RuleApplicability`, `SourceLanguage`, `HostRelation`, `HostRow`, `ObservationBinding`, `ObservationBindingName`,
  `ObservationColumn`, `ObservationProducer`, `ParameterValue` and its five data types, `FlagState`): the tip SDK's,
  field for field. Changed only where the same concept is already defined by another sheet: `RuleId` is plan's, `Glob`
  corpus', `ParameterName` programs', `QueryBundle` (the rule's text; the SDK's `RyoQlQuery` wraps it) and
  `ObservationName` ryoql's, `LanguageName`, `HostedRelation` and the host names the relations', `ColumnName`,
  `CellKind` and `ByteSpan` the table's, `ProjectSearchScope` the language's. `HostRow`'s number is `HostRowNumber`,
  because `RowNumber` is the table's word. Decision 1.
- **Report** (`LintReport`, `Finding`, `FindingColumn`, `FindingKey`, `Slot`, `Cell`, `HostText`, `HostNumber`,
  `Place`, `Line`, `Column`, `Unknown`, `Message`, `Skipped`, `SkipReason`, `Unread`, `Unreadable`, `ReadFailure`): the
  tip SDK's. `LintReport` loses its hidden `rows_total`; a rows answer is `RowsReport { report, total: RowTotal }`.
- **Failures.** `LintError`: the tip SDK's 16 variants. `Rule` carries ryoql's `CheckError`, of which the SDK's
  `ValidateError` is the public mirror, because inbound crates may not import each other. `Provider` carries the
  catalog's `LanguageName` and execute's `ProviderFault` (the SDK's `ProviderCode` has the same four variants). `Port`
  loses `Facts` and `Output`, which nothing constructs at the tip. `MemoryError`: main verbatim.
- **Memory.** `MemoryPort` and `MemoryError` are main's verbatim. `MemoryBudget` and `MemoryBytes` are this crate's:
  today the SDK's `capacity::MemoryBudget` and values' `runs::{Budget, MemoryBytes}` are two types for one concept.
  `memory::sysinfo(budget)` is main's factory, called only by edges.
- **Measurement.** `print_measurement()` under the `bench-hooks` feature, as today (section 8).

### 5e. What stays private (and is the crate's body)

The request-to-ordinal map and the per-file-version request lists; the decoding of answer tables into findings
(subject and key columns from `Programs::emit`, Symbol cells through `SymbolsUseCase::text`); the parameter conversion
to programs' `PlanConstant` (today `entry.rs:206`); the host-relation checks; the unread naming; the call-to-call state
of decision 4; the clocks `print_measurement` prints.

### 5f. What stops being public, and the counts

| Side | Public items in the concept | In the draft |
|---|---|---|
| main core: lint 21, open 6, memory 3, row_sink 11 | 41 | `LintUseCase` and `MemoryPort` with its error and factory carried; the rest listed in api.txt `# NO LONGER PUBLIC` |
| main SDK lint module | 37 | its request and report types carried; `lint`, `lint_with`, `prepare` gone already at the tip |
| tip core: lint 20, memory 3, row_sink 11 | 34 | the dead family gone; `FindingCoordinates` to execute; row sink to the command line |
| tip SDK lint module | 38 | 26 types stay here or move to their concepts; 5 `pub mod`, `Linter`, the SDK's composition types stay in the SDK |
| drafted | 62 items + 59 variants | — |

## 6. Proof it brings

| Proof | Source | Fate |
|---|---|---|
| `fixtures/contract.rs::check` (3 rows: no rule no unread, blank id, duplicate id) | main core | **restated**: the contract test every `LintUseCase` runs, over fakes of the handed-in parts |
| 19 use-case tests, 97 rows | main core `use_case_test*` | **restated** by group: `rules` 19, `refusals` 4, `runs` 16, `use_case_test` 10 test this crate (rules, refusals, cold/warm/edit runs through a fake `CachePort`); `lifecycle` 23 split: the store-damage rows (`a_store_cut_short_...`, `..._byte_flipped_...`) to the cache sheet, the plan-reuse rows (`compiles_each_rule_once_per_session`, `interleaved_runs_naming_different_rules_compile_each_rule_once`) here, against decision 4; `project_budget` 6 to the project sheet's budget guard; `guards` 10 and `inapplicable_identity` 6 not read row by row (section 11) |
| 2 scenario outlines, 4 examples | main core `acceptance.feature` | **restated** as lint scenarios (two violations and an unread file; text only; refused rule; corpus down) |
| 3 tests, 2 examples | main core `open` | **restated** under decision 5 (catalog answers; budget larger than the machine refused) |
| 42 tests | tip `sdk/tests/lint*.rs` | **carried** as they are: they go through the SDK's `Linter`, whose surface does not change; they are the end-to-end proof of `lint` |
| `the_linked_executor_answers_lists_and_a_dropped_linter_closes_its_session` | tip `sdk/src/resources/linter/lint/entry_test.rs:59` | **restated**: N requests give N results in order (row 17); the linked-executor half goes with the link point |
| `walk_test.rs` 1, `session/entry.rs` 1 | tip SDK | the walk test to corpus; the version-environment test restated here |
| SDK unit tests, 6 over 47 rows | main `entry_test`, `schemas_test`, `composition_test` | `schemas_test` (rule id and glob parsing) to plan and corpus; the rest restated here where they test a call |
| memory: 2 adapter tests, 3 fake tests | main core; the fake deleted at the tip | adapter tests carried; **fake and its tests restored from main** |
| row sink 4, coordinates 1 | both | move with their code (command line; execute) |
| **missing** | — | seam tests that one call loads once for N requests sharing rules (row 6: count `load` calls); that screens are prepared once per language per call (row 19); that a full cache hit calls neither `load` nor `scan` (row 14); that `rows` reads memory before the first `scan` |

Planted bugs the restated tests must catch: answers out of request order (row 17 test); a second `load` per request
(row 6 test); `keep` after return (`lint_cache.rs:109`); a refused rule answered as empty (`refusals`); a walked corpus
not walked (`lint_refusals.rs: a_walked_corpus_walks_the_repository`).

## 7. Perf gains it carries

Rows of `perf-gains.tsv` whose crate column names lint (6, 10, 17), then the rows lint must keep working though another
crate owns them.

| Row | Gain (measured) | State | What in the draft keeps it |
|---|---|---|---|
| 6 | three drivers joined (LOSS: fastapi 0.941 -> 1.702 CPU-s, start-up 73.6 -> 295.5 ms) | live, regression open | Lint makes ONE `LoadRequest` from every request of the call and calls `load` once; execute takes ordinals into that one set and admits nothing. The double admission (SDK `admit`, entry.rs:477, then `Batch::admit` through `register_query`) has no place to live: lint has no admission of its own and no `Batch` |
| 10 | per-request blake3 hashing and a whole-call cache (+600 M instructions; 205 whole calls reused cold) | live, unjustified | Keys come from one place, computed once: the file's key is the corpus' `FileVersion` content key, the rule's key is made once per distinct rule text in the call, the request key combines them; no per-request rehash of the rule text (`call_cache.rs:216` today). The whole-call memo (`completed_calls`, entry.rs:103, 529, 549) is not carried (decision 3) |
| 17 | the Tastemaker check submits its calls in rounds (Ryo part 29.013 -> 25.541 CPU-s) | live | `lint(Vec<LintRequest>) -> Vec<Result<..>>`, one result per request in order; one load, one screen preparation and one spread over workers per call |
| 14 | warm store: a full hit answered before preparation (rerun Django 62.0 -> 32.1 ms) | live | Step 2 of 5b asks the cache per request before `load`; a call whose requests all hit calls neither `load` nor `scan` |
| 19 | pair screen across the whole session (22.34 -> 17.04 CPU-s) | live | `ScreenUseCase::prepare` once per language per call over the call's one `Programs`, shared by every request; across calls, decision 4 |
| 13 | one frozen prepared set for every worker (ten cores fastapi 3.62 -> 1.38) | parked | Lint hands every worker the same `&Programs` and `&Screens` through `ScanUseCase::open`; it never binds per worker |
| 50, 51 | file versions shared by inode and content; absence known from the read | live | Lint reads each distinct file version once per call through `CorpusPort::read` and passes the `FileVersion` (or its `CorpusError`) to `scan` |
| 52 | teardown skipped; store 291 MB -> 2.48 MB | live | `keep` before `lint` returns; nothing needs a destructor walk |
| 54 | `PoolBudget::for_workers` | live | `stores(WorkerCount)` per worker, with the call's worker count |
| 2, 53 | the kept store's size (defect open) | live, parked | Behind `CachePort`; lint hands the cache findings per file, never plans or frontier columns |

**Aim 3, perf changes from the history that would have landed inside lint alone under this draft:** row 17's engine side
(take a batch, answer in order), row 10's whole-call memo (and its removal), row 14's ordering (ask the cache before
loading), and the admission half of row 6 (stop re-shaping each request; build one load). **Still crossing:** row 10's
keys (cache and corpus), row 6's one load (programs), row 53 (cache, and the plan's proof of file-local rules), row 2
(cache and the edges), row 17's Tastemaker half (its caller).

## 8. Benchmarks that cover it

| Benchmark | Where | What it calls in lint's concept today | After the draft |
|---|---|---|---|
| Ruff, Oxlint and Clippy linter benchmarks and their case suites | the benchmark crate's local engine adapter: `let built = linter::Linter::new(  @2a207464c:crates/ryo/crates/benchmark/src/outbound/engine/adapters/local/adapter.rs:63` and `let report = engine.lint(vec![LintRequest {  @2a207464c:crates/ryo/crates/benchmark/src/outbound/engine/adapters/local/adapter.rs:206` | the SDK's `Linter::new` and `Linter::lint`; the tracing event read at `if row.target != "ryo::node_counters" {  @2a207464c:crates/ryo/crates/benchmark/src/application/profile_evidence.rs:325` and `const NODE_COUNTERS_TARGET: &str = "ryo::node_counters";  @2a207464c:crates/ryo/crates/benchmark/src/outbound/engine/adapters/local/writer.rs:24` | unchanged calls (the SDK surface is the owner's and stays); lint emits the same event with the same `runs`, `nodes` fields |
| the lint images `vec`, `vec_ts`, `vec_rust` (the bar table) | `let engine = sdk::Linter::new(  @2a207464c:crates/ryo/crates/executor/benches/lint_image/main.rs:144`, `.lint(vec![request])  @2a207464c:crates/ryo/crates/executor/benches/lint_image/main.rs:154`, `sdk::print_measurement();  @2a207464c:crates/ryo/crates/executor/benches/lint_image/main.rs:157` | `Linter`, and `print_measurement` under `bench-hooks` | the same three calls; `print_measurement` prints lint's own lines. The `CHECK_SESSION` line's executor-internal fields (`bound_proofs`, `literal_positions`, `distinct_pairs`, `requirement_programs`, `requirement_bindings`, `visit_projects`) are not reachable from lint; no gate script reads them (`short-check.sh`, `window-nine.sh`, `price.py` grep clean), only older experiment scripts under `build/redo-lowering/` |
| the Tastemaker check (`tm` image) | `Ok(ryo::linter::Linter::new(  @2a207464c:crates/tastemaker/crates/core/src/outbound/analysis/adapters/ryo/adapter.rs:73`, `let mut linted = linter.lint(sent).into_iter();  @2a207464c:crates/tastemaker/crates/core/src/outbound/analysis/adapters/ryo/adapter.rs:150`, `ryo_sdk::linter::print_measurement();  @2a207464c:crates/tastemaker/benchmarks/project/live/tm_main.rs:72` | `Linter` in rounds (row 17) | unchanged |
| the Tastemaker benchmark project's capture | `use ryo_executor::check_session::replay;  @2a207464c:crates/tastemaker/benchmarks/project/live/capture.rs:4` | the executor directly, not lint | breaks with the executor; it must call the SDK's `Linter` (programs sheet says the same) |
| `query run` cases | `let report = sdk::lint::rows::answer(&linter, lint, usize::try_from(request.limit.0).unwrap_or(usize::MAX), memory)  @2a207464c:crates/ryo/crates/sdk/src/resources/query/run/entry.rs:70` | `rows::answer` | `LintUseCase::rows`; what it answers is unchanged |
| the twelve workloads and the bar table | evidence `build/ports/short-check.sh`, `build/tm-int/scripts/window-nine.sh` | the built binaries | unchanged |
| a bench of lint alone | — | none on either side | none proposed: lint's own cost is per call (admission, decoding); the images measure it |

## 9. Checks
IMPORTS: table, symbols, relations, operators, ryoql, plan, programs, screen, execute, project, language, corpus, cache, scheduler
CROSSED: per request

- **Directions.** `inbound` may import `application`, `domain` and `outbound`: screen, execute, project are application;
  table, symbols, relations, operators, ryoql, plan, programs are domain; language, corpus, cache, scheduler are
  outbound. No inbound crate: the refused-rule wording uses ryoql's `CheckUseCase`, not `inbound/validate` (today the
  session calls the SDK's `validate`, entry.rs:200, which in the tree would be an import between inbound crates). No
  `store`: lint is generic over `S: TablePort` and the store is made by the closure the edge hands in. Outside crates:
  `sysinfo` (the memory adapter), `tracing` (the event); `rayon`, `blake3` and `ignore` leave lint (scheduler, cache,
  corpus).
- **No layout word.** No `Table`, `cols`, `Batch`, `World`, `Live`, slot or `Rc` in a signature; answer tables stay
  inside the crate (`S::Table` only in private code).
- **How often.** `lint` and `rows` once per call; inside, lint crosses `load` once per call, `prepare` once per language
  per call, `open`/`close` once per worker, `scan`, `CorpusPort::read`, the file cache lookup and `receive` once per
  file, `run` once per call, `keep` once per call. Nothing per row crosses a contract: decoding reads a whole answer
  table of one rule in one file through the store's reader (per set). The parts are generics, never `dyn`; the only
  `dyn` left is `SchedulerPort::run`'s job closures, once per worker.
- **Isolation.** Replacing lint's implementation touches only this crate; a perf change to admission, decoding, the
  cache's ordering or the request-to-file map lands here alone (section 7). Adding a language touches nothing here.
- **nu-hex inside.** `src/inbound/use_cases/lint` (trait, impl, parts), `src/outbound/memory` (port, sysinfo adapter,
  fake under `fixtures`), `src/domain/{requests, reports, admission, decoding}`.
- **Tastemaker.** New entries `crate-lint`, `nu-hex-lint`; the stale `pipeline` spec (tip line 113) re-pointed to
  `crates/inbound/idiomatic-sh-ryo-lint/src/inbound/use_cases/lint`; the stale allocation entry (tip line 88) re-pointed
  to a lint allocation test or removed with a reason; the command line's dependency on the SDK at the tip
  (`ryo_sdk = { package = "idiomatic-sh-ryo-sdk", path = "../crates/sdk" }  @2a207464c:crates/ryo/cli/Cargo.toml:22`)
  contradicts `edges-apart` and goes once both edges call lint.

## 10. Decisions for the owner              (each: the example from the code; why; what we get; my recommendation)

1. **The request vocabulary: one definition per concept.** Example: the SDK's `linter` module re-exports 36 lines of its
   own `domain` types beside the lint types (`sdk/src/resources/linter.rs`); several repeat a concept another crate
   defines: the SDK's `RuleId` beside plan's `RuleId`, `ParameterName` beside programs', `ObservationKind` (six
   variants) beside `CellKind`, `ProviderCode` beside the executor's `ProviderFault`, `ValidateError` beside ryoql's
   `CheckError`, `RowNumber` meaning a host row while the table uses it for any row. Why: one definition per concept, and
   inbound crates cannot share types with each other, so what lint and validate both use must sit in domain. What we
   get: lint owns only lint's words (request, rule, report, finding, place, unread, its error); the SDK re-exports them
   and the domain ones. Cost: the SDK's surface changes names it re-exports (the owner allowed breaking changes); plan's
   `RuleId` must gain the SDK's non-blank check (`RuleId::try_from`, a private field). **Recommend: yes, as drafted.**
2. **`query run`'s rows answer.** Example: `rows::answer(linter, request, limit, memory)` (rows.rs:29) is crate-private
   to the SDK by an earlier decision ("no hidden Linter method, no hidden LintReport field", `2bde413a94`), but the
   command line's `query run` needs the same answer and an edge cannot import the other. Why: without a lint call for it
   the command line keeps a second path or keeps depending on the SDK. What we get: `LintUseCase::rows(request, limit,
   &memory)` returning `RowsReport`; the SDK's `Linter` does not expose it, so the owner's `new` + `lint` surface stands.
   Alternatives: main's `answer: AnswerShape` field on `LintRequest` (visible to every SDK user), or `query run` as its own
   inbound crate (a fourth inbound crate the tree does not have). **Recommend: the `rows` method, memory handed to the
   call.**
3. **The whole-call memo.** Example: `completed_calls` keeps up to 128 small reports per pool and answers an identical
   later call from memory (entry.rs:103, 529-556); row 10 measured +600 M instructions of hashing for 205 reused calls out
   of 46,831. Why: it is a second cache beside the kept store, keyed by its own `CallKey`. What we get by dropping it: one
   cache (`CachePort`), keys computed once per rule text. Risk: the Tastemaker check or the case suites may have leaned on
   it; the check's bar (5 s) is the measure. **Recommend: drop it, measure the Tastemaker check before and after.**
4. **What one call leaves for the next.** Example: the tip keeps a `Live` session per linter in a thread-local map
   (entry.rs:4) with the `Batch`, the prepared programs, the file versions, the glob filters and the memo, and runs the
   whole call on the pool's owner thread to reach it. Why: the SDK promises that "every lint over it shares its session
   (the plans, the files read, the answers kept in cache)" (the doc of `Linter::new`, the SDK's
   lint/entry.rs:15-16), and the Tastemaker rounds reuse plans. What we
   get: the kept answers live in the cache, file versions in the corpus adapter; lint itself keeps, privately, the last
   `Programs` and `Screens` for an equal deduplicated rule set (programs decision 1: `Programs` is `Clone`) and its compiled
   glob matchers; no thread-local and no owner-thread requirement. **Recommend: as drafted; nothing of it public.**
5. **Fates of main's contracts lint no longer calls.** Examples: `OpenUseCase` (main open/interface.rs:7; deleted at
   `35b6eaf0cf` on "nothing calls it"), `RowSinkPort` (main row_sink/interface.rs:10; at the tip implemented and called
   only by the command line), core's second type family (read only by the command line's `remedy.rs:71` and test
   fixtures), `FindingCoordinates` (read by the executor), `DEFAULT_LIMIT` (read by the command line's `--limit`). Why: a
   designed contract is not dropped for lack of a caller; each needs a home. What we get: `OpenUseCase` folded into lint
   (its catalog check where languages are linked, its budget check in `rows`), its 3 tests and 2 examples restated there;
   `RowSinkPort` and `DEFAULT_LIMIT` to the command line, the one edge that writes rows; `FindingCoordinates` to execute;
   core's second family deleted with `core`. **Recommend: as listed; `OpenUseCase` needs your yes because it is a designed
   use case being folded, not moved.**

## 11. Not checked

- **Quotes.** All 82 `@rev:path:line` quotes in this sheet were re-checked by a script running `git show "REV:PATH" | sed
  -n 'LINEp'` and comparing the trimmed line; 82 match, 0 differ. Inline `path:line` references without `@` were read by
  hand when the code was read (entry.rs:206, 529-556, rows.rs:29-67, errors.rs:14-130, tastemaker.toml lines) and are
  not script-checked. `api.txt` carries tags, not quotes.
- **Sheets this one leans on that do not exist yet**: outbound/language (`LanguageConnection`, `ProjectSearchScope`),
  outbound/corpus (`CorpusPort::read` returning `FileVersion`, `GlobsUseCase`, `Glob`), outbound/cache (`CachePort`,
  `AnswerKey`), outbound/scheduler (`SchedulerPort`, `WorkerCount`), outbound/store (the per-worker constructor).
  The relations listing does not yet hold `HostedRelation` or the three host names; ryoql's does not yet hold
  `ObservationName`. When those land, this listing is re-checked against them.
- **Byte identity of refusals.** `LintError::Rule` carrying ryoql's `CheckError` in place of the SDK's `ValidateError`,
  and `Provider` carrying execute's `ProviderFault` in place of `ProviderCode`: whether their `Display` texts are
  identical was not compared. The tip reports a memory failure on the rows path as `Io { what: Port::Corpus, reason:
  "memory: .." }` (rows.rs:36, 44); whether to keep that text or use `Port::Memory` is not decided.
- **Whether `check` alone words a refusal as the tip's `validate` does**: the tip's `validate` may also run the kind
  check (plan's `judge`); not traced.
- **The middle of `admit`** (entry.rs:477-724: version keys, environments, hosted keys, the retained-store proof) was
  read in outline; how much of it is corpus' and cache's rather than lint's is assigned here by reading, not by tracing
  every call.
- **Main's `guards` (10 rows) and `inapplicable_identity` (6 rows)** were counted by name only; their fates are not set.
  Main's `service.rs` (400 lines) was not read.
- **The cost of decoding answers inside the worker job** (execute decision 5) against the tip's delivery, which renders
  after the epoch in parallel chunks of 128 or more (entry.rs:733-755): not measured.
- **Hosted schemas per request.** One load per call assumes the requests of one call declare compatible host relations
  (programs sheet, section 11); if not, lint loads once per distinct set. Not checked against the Tastemaker check.
- **`on_owner`.** The draft drops the owner-thread run; whether the rayon adapter needs the call to start on its own pool
  for nested jobs (the reason `on_owner` was added) is not checked.
- **No `tm check`** was run on a skeleton of the draft: `Vec`, `String` and `usize` in public fields and the tuple-free
  but long generic list of `LintParts` may fail the nominal-type rules.
- **Line counts** are by `wc -l` per file, tests excluded by file name; public-item counts are by the regex in section 2
  and count methods inside impls.
