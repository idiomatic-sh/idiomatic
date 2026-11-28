# domain/programs
STATUS: drafted (28 Nov; not reviewed, not agreed). Bead ryo-fmx3.1.13.23.4.9.

Revisions: MAIN = f4ae3bd802, TIP = 2a207464c, B133 = f0219b3b3. "X" below is `crates/ryo/crates/executor/src` at TIP.
Every quote is one line, two spaces, `@rev:path:line`; section 11 gives how many were re-checked and how.
Types borrowed from other sheets: `table` (`Shape`, `ColumnNumber`), `symbols` (`FrozenSymbols`), `relations`
(`Relation`, `RelationName`, `HostedRelation`), `operators` (`PlanUseCase`, `PhysicalPlan`, `RuleOrdinal`, `PlanNodeId`,
`Demand`, `PlanConstant`, `OperatorFault`), `ryoql` (`ParseUseCase`, `FragmentText`, `RuleText`, `ObservationName`).
Three come from `plan`, whose sheet is NOT written yet: `LowerUseCase`, `KindVocabulary`, `RuleCut` (working names
from PLAN.md Appendix A 3.6); the plan sheet decides them.

## 1. Owns
A request's rules prepared to run: made once before the first file, frozen, and read by every worker.
(One thing. Parsing is ryoql's, lowering and the file/cross-file cut are plan's, binding steps to kernels is operators',
the dictionary is symbols'. Programs calls each of them once per load and owns the result. Which rules run on a file is
screen's; running them is execute's.)

## 2. On main (f4ae3bd802)
**Where.** Two calls in `execution`, and the prepared program they make in `execution_runtime`:
```
pub trait CompileRulesUseCase {  @f4ae3bd802:crates/ryo/crates/execution/src/inbound/use_cases/compile_rules/interface.rs:10
fn compile_rules(  @f4ae3bd802:crates/ryo/crates/execution/src/inbound/use_cases/compile_rules/interface.rs:17
pub struct CompileRequest<'a> {  @f4ae3bd802:crates/ryo/crates/execution/src/inbound/use_cases/compile_rules/types.rs:5
pub trait PrepareUseCase {  @f4ae3bd802:crates/ryo/crates/execution/src/inbound/use_cases/prepare/interface.rs:10
fn prepare(&self, request: PrepareRequest) -> Result<PreparedExecutionProgram, OperatingError>;  @f4ae3bd802:crates/ryo/crates/execution/src/inbound/use_cases/prepare/interface.rs:16
pub enum PlanSource {  @f4ae3bd802:crates/ryo/crates/execution/src/inbound/use_cases/prepare/types.rs:8
pub struct PrepareRequest {  @f4ae3bd802:crates/ryo/crates/execution/src/inbound/use_cases/prepare/types.rs:22
pub struct PreparedExecutionProgram {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/prepared_execution/types.rs:23
pub trait ProgramPreparedExecutionPort {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/prepared_execution/interface.rs:22
pub trait PreparedReporting {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/prepared_execution/interface.rs:99
```
`compile_rules` compiles each rule through the port it is handed, one plan per rule, and keeps the plans across requests
in a bounded cache keyed by rule id ("a program that lints many projects with the same rules compiles each rule once"):
```
pub trait QueryLanguagePort: Send + Sync {  @f4ae3bd802:crates/ryo/crates/execution/src/outbound/query_language/interface.rs:11
fn compile(  @f4ae3bd802:crates/ryo/crates/execution/src/outbound/query_language/interface.rs:30
pub struct CompiledPlans(pub(super) Arc<Mutex<PlanCache>>);  @f4ae3bd802:crates/ryo/crates/execution/src/application/rule_compilation/service.rs:23
pub struct CompiledRules {  @f4ae3bd802:crates/ryo/crates/execution/src/application/rule_compilation/service.rs:28
```
`prepare` then plans the whole request ONCE through `OperatorsPort::plan` and freezes it behind `Arc`s: one
`PassProgram` per request, shared by every file, with only per-file scratch pooled under a lock:
```
pub fn prepare(  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/service.rs:37
pub(crate) struct PassProgram {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/file_pass/types.rs:43
pub(crate) scratch: std::sync::Mutex<Vec<PassScratch>>,  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/file_pass/types.rs:53
fn plan(&self, program: &LogicalProgram<'_>) -> Result<PhysicalPlan, PortableOperatorFault>;  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/outbound/operators/interface.rs:25
pub(crate) struct PreparedProgram {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/preparation/types.rs:67
```
The opaque program is held by a `compile_fail` doctest (`/// ```compile_fail,E0603` at `preparation/types.rs:64`).
- **Public items: 49** (compile_rules 12: trait, method, `CompileRequest`, `CompileRulesUseCaseImpl` with `new` and
  `compiling_into`, `RuleCompilationError`, `CompiledPlans`, `CompiledRules` with `plans`, `bindings`, `identify`;
  prepare 7: trait, method, `PrepareRequest`, `PlanSource`, the re-exported `ObservationIdentity`, `PrepareUseCaseImpl`
  with `new`; the prepared program 30: `PreparedExecutionProgram`, `ProgramPreparedExecutionPort` with 14 methods,
  `PreparedReporting` with 2, ten read types, `RuleTotal`), plus the 4 of `QueryLanguagePort` it is handed.
- **Lines without tests:** 2,236 (compile_rules, prepare and rule_compilation 996; runtime `program/preparation` 702;
  `prepared_execution` 474; execution_domain `prepared_request` 64), counted with `git show | wc -l` over the non-test
  files of those directories.
- **Implementors:** `CompileRulesUseCaseImpl`, `PrepareUseCaseImpl` (execution); core wraps both
  (`core/src/application/engine/service.rs:47`, `:87`). `QueryLanguagePort` has one production adapter
  (`execution/src/outbound/query_language/adapters/use_case/adapter.rs:41`) and test fakes.
- **Callers:** the lint use case, once per request:
```
let compiled = self.compilation.compile_rules(CompileRequest {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/lint/use_case.rs:183
.prepare(PrepareRequest {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/lint/use_case.rs:253
let services = execution.begin(&program, &listing.files, observer.clone());  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/lint/use_case.rs:263
```
  Production files naming the items, by crate (`git grep -l`, tests and fixtures excluded): core 14, execution 8,
  execution_runtime 5, sdk 2 (`lint/composition.rs:333`, `lint/entry.rs:8`).
- **Proof:** 13 `#[test]` (compile_rules 2 over 4 case rows; prepare 2; rule_compilation 2 over 10 case rows plus
  `observing` 1; core engine 1 over 1 row; runtime preparation 2; `tests/boundary_contract/compiled_rules` 3) and two
  acceptance features of two scenarios each (`compile_rules`: "Distinct rules compile", "A repeated rule id is
  refused"; `prepare`: "An empty request is prepared", "A binding without its rule is refused").
- **Tastemaker:** `idiomatic.arch/entry-traits-witnessed` names both interfaces and the prepared-program ports
  (`crates/ryo/tastemaker.toml:76` on main); `idiomatic.arch/library-surface` keeps the engine crates private (`:16-17`).

## 3. On the perf branch (2a207464c)
**There is no crate and no contract.** The concept is spread over five places in the proof of concept and the SDK,
each a pub item of `ryo_executor` (whose `lib.rs` makes every module `pub`).
1. **The lowering, called from the executor** (`X/lower.rs`, 249 lines): parse with the factory (ryoql's sheet), then
   lower the batch once with plan's `lowered`, then cut it again into one renumbered `FinishPlan` per query:
```
pub fn query_batch(  @2a207464c:crates/ryo/crates/executor/src/lower.rs:120
let mut plan = lowered(&programs, PlanningStage::Expanded);  @2a207464c:crates/ryo/crates/executor/src/lower.rs:141
let mut shared = SharedSteps::default();  @2a207464c:crates/ryo/crates/executor/src/lower.rs:194
let typed = Arc::new(FinishPlan { steps: shared.finish(), rules: vec![planned], unplanned: Vec::new() });  @2a207464c:crates/ryo/crates/executor/src/lower.rs:202
Ok((super::finish::from_typed(typed.clone()), typed))  @2a207464c:crates/ryo/crates/executor/src/lower.rs:203
pub struct SharedPlanData {  @2a207464c:crates/ryo/crates/executor/src/lower.rs:76
pub fn finish_plan(pack: &ReadPack, keep_for_first: bool, separator: Option<&str>, emit: Option<&str>, literals: Option<&mut SharedPlanData>) -> Result<super::finish::Plan, Refusal> {  @2a207464c:crates/ryo/crates/executor/src/lower.rs:207
```
   A rule that observes others is lowered alone, re-parsing every producer's text into its own pack:
```
pub fn query_parts(rule: &str, text: &str, mut context: RuleContext, keep: bool, separator: Option<&str>, modules: ParsedModules, observations: &[Observation], literals: &mut SharedPlanData) -> Result<(super::finish::Plan, ReadPack), Refusal> {  @2a207464c:crates/ryo/crates/executor/src/lower.rs:90
texts.rules.push((alias.clone(), Some(source.text.to_string())));  @2a207464c:crates/ryo/crates/executor/src/lower.rs:101
pub struct Producer {  @2a207464c:crates/ryo/crates/executor/src/lower.rs:62
pub struct Observation {  @2a207464c:crates/ryo/crates/executor/src/lower.rs:69
```
2. **Binding from the typed plan** (`X/finish/plan_load.rs`): `from_typed` builds the executor's own plan, whose
   per-node data is filled lazily under `OnceLock`s:
```
pub fn from_typed(plan: Arc<FinishPlan>) -> Plan {  @2a207464c:crates/ryo/crates/executor/src/finish/plan_load.rs:51
pub args: std::sync::OnceLock<Args>,  @2a207464c:crates/ryo/crates/executor/src/finish.rs:486
pub frames: std::sync::OnceLock<Vec<frame::Frame>>,  @2a207464c:crates/ryo/crates/executor/src/finish.rs:491
pub emits: Vec<(String, usize, String, Vec<String>)>,  @2a207464c:crates/ryo/crates/executor/src/finish.rs:494
```
3. **The prepared program and the session's schedule** (`X/check_session.rs`): one `Program` per (query, parameters),
   cached for the session, with two caches filled on first use behind a lock:
```
pub struct Program {  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:56
pub requirements: std::sync::Mutex<Vec<(Option<usize>, std::sync::Arc<Requirements>)>>,  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:65
pub unknown_kind: std::sync::OnceLock<Option<refusal::Refusal>>,  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:67
pub fn prepare(  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:88
pub fn assemble(  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:102
pub fn refuse_unknown_kind(&self, languages: impl FnOnce() -> Vec<(String, Vec<String>)>) -> Option<refusal::Refusal> {  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:130
pub programs: Vec<crate::check_session::refusal::Planned>,  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:256
pub prepared: HashMap<(Query, Parameters), usize>,  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:257
pub fn prepare_many(&mut self, items: Vec<(Query, Parameters)>) {  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:313
if !query.observations.is_empty() || self.prepared.contains_key(&(query.clone(), parameters.clone())) {  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:317
if group.len() < 2 { continue; }  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:327
pub type Planned = Result<crate::check_session::Program, Refusal>;  @2a207464c:crates/ryo/crates/executor/src/check_session/refusal.rs:73
```
   and, per worker and per reader visit, every needed program is bound again against the reader and the worker's own
   dictionary, then all of them are re-merged into one plan for the visit:
```
pub fn bind<'p, L: FileSource>(  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:601
pub struct Bound<'p> {  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:579
let schema = parser.and_then(|p| p.declarations().iter().find(|s| s.name == name))  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:620
if parser.is_some_and(|p| !p.expands(rel)) {  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:647
(node,alts.iter().map(|(col,w)|(*col,u64::from(run.sym(w)))).collect())  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:668
let mut merged = shared::ReaderPlan::default();  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:951
let mut b = bind(&mut registry, program, parser, &mut run, tables,  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:979
let (plan, supplied) = merged.finish();  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:989
pub struct ReaderPlan {  @2a207464c:crates/ryo/crates/executor/src/check_session/shared.rs:10
```
   `visit` is called once per file worker, each with the shared `programs` slice:
```
let (results, children) = jobs::run(&shards, |shard| {  @2a207464c:crates/ryo/crates/executor/src/check_session/worker.rs:193
```
4. **Admission** (`X/check_session/replay/batch.rs`): per (program, host schema), the first time it is admitted, a
   throwaway run types the program and computes the file/cross-file cut:
```
pub struct Admission {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/batch.rs:88
pub fn prepare(id: usize, program: &Program, hosted: &[Hosted], catalog: &session::named::Catalog, patterns: &finish::Patterns) -> Result<Self, crate::check_session::refusal::Refusal> {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/batch.rs:149
let local = finish::partition::file_steps(&mut exec, &mut run, |name| {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/batch.rs:173
Some(Arc::new(finish::partition::partition(&mut exec, &mut run, local)?))  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/batch.rs:203
pub fn prepare_rules(&mut self, items: Vec<(Query, Parameters)>) {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/batch.rs:220
pub fn register_query<L: session::source::Source>(  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/batch.rs:239
```
5. **The SDK's admission** (`sdk/src/resources/linter/lint/session/entry.rs`), which reshapes each request and then
   asks the executor to prepare, per rule, again (bead ryo-fmx3.1.13.19):
```
fn admit(&mut self, req: sdk::LintRequest, linked: &sdk::Languages, unbounded: bool) -> Result<RoundRequest, sdk::LintError> {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:477
self.batch.prepare_rules(items);  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:605
let program = self.batch.prepared_rule(query.clone(), &parameters);  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:677
```
- **Public items: 42** by this list (my boundary; see 5g): lower.rs 8, plan_load.rs 3 (`IndexedSteps`, `StepInputs`,
  `from_typed`; the rest of the file is operators'), `Program` and its 4 methods, `Requirements`, `Parameters`,
  `Schedule::{prepare_many, prepare, finish_preparation}`, `Bound`, `bind`, `Bindings`, `FactMask`,
  `requested_facts`, query.rs 4, shared.rs 6, `kinds::unknown`, `Planned`, `Admission` and its `prepare`,
  `Batch::{prepare_rules, prepared_rule}`. Plus their public fields (13 in lower.rs, 8 in query.rs, 8 in `Program`).
- **Lines without tests:** about 1,020 in the ranges named above (lower.rs 249; `check_session.rs` 56-143, 247-358,
  549-705; `batch.rs` 81-238; query.rs 98; shared.rs 84; kinds.rs 32; `plan_load.rs` 51-94). By range, not by file.
- **Callers:** 23 production files of the executor and 2 of the SDK (`lint/session.rs`, `lint/session/entry.rs`) name
  these items (`git grep -l`); the Tastemaker benchmark capture reaches them through
  `ryo_executor::check_session::replay` (section 8).
- **Proof:** 5 `#[test]` touch it: `check_session/kinds_test.rs` (`a_parameter_nothing_is_bound_to_is_refused_by_name`,
  `a_word_for_a_kind_no_language_has_is_refused_with_the_languages_looked_in`, `a_word_that_names_a_kind_is_not_refused`,
  `a_pattern_is_checked_against_the_kinds_and_not_as_a_kind`, `a_query_that_writes_no_kind_asks_no_language`), all
  through `query_batch`. No test of `Program`, `bind`, `Admission::prepare`, `ReaderPlan` or `from_typed`. No feature.
- **Tastemaker:** none names the concept; the executor has `crate-executor` (`crates/ryo/tastemaker.toml:105`) and a
  sibling allowlist (`:31`).

## 4. What changed between them
- **Removed:** main's `CompileRulesUseCase`, `PrepareUseCase`, `QueryLanguagePort` and the whole prepared program, at
  `861c7de10e` (26 Nov: "the old executor goes: the execution use case, prepare, file_dag, ...") and `fd6122c42a`
  (27 Nov: "... the compile use case, compile_rules, rule_compilation and the query-language port are deleted").
  The 13 tests and 4 scenarios went with them. Build 133's `LoadUseCase` went with the engine crate at `04f8f4ec1b`
  (27 Nov: "the engine crate, plan_codec and the vec lower/stage tool are deleted").
- **Added on the tip:** the executor's own preparation, in steps: `24665d03e4` (23 Nov, "Bind executor steps directly
  from the shared typed plan": perf row 25), `28017029a9` (25 Nov, "Lower the queries of a request that share a module
  catalog together, once, and give each its step subset": row 8), `786a8a6ca2` (26 Nov, "the admission comes from the
  executor's own lowering: typed refusals, one compile per query"), `04f8f4ec1b` (the lowering moved into
  query_language: row 1).
- **Changed in kind:** main froze ONE program per request and shared it with every file (`Arc<PreparedProgram>`,
  `Arc<PassProgram>`, planned once through `OperatorsPort::plan`). The tip shares the `Program` list across worker
  threads but re-does the expensive half per worker and per reader: `bind` against the worker's `Run` (interning into a
  per-worker, `Rc`-based dictionary), `Exec::new`, and the `ReaderPlan` re-merge (section 3, item 3). That is the
  measured cost of ten workers: preparation 1.637 s against 0.351 s (perf row 13; `review-performance.md` B5).
- **Changed in kind, start-up:** Python start-up 296 ms, all inside the SDK's admit (`trial-1/summary.txt`, quiet
  run): lowering the 412 rules in one batch 95-98 ms; one rule (RUF100) lowered alone 64 ms because it observes the 410
  others and `query_parts` re-parses their texts; per-rule typing in `Admission::prepare`/`file_steps` 103 ms;
  partition 21 ms (fixed to 0.6 ms by `ceb2047110`, row 1); empty epoch 19 ms.
- **Lost that should not have been:** (1) main's frozen per-request program shared by every file, which the tip has
  only for the lowered `FinishPlan`, not for the bound plan; (2) main's 13 tests and 4 scenarios and build 133's load
  feature (12 scenarios, among them "a sub-plan two rules share is lowered once" and "load accepts the rules it can
  lower and names each one it refuses"); (3) one contract for the concept: the tip's is five places and 42 pub items.

## 5. The drafted public interface        (explains every line of api.txt that is changed or new)
`api.txt` beside this sheet: **29 public items** (1 trait, 1 method, 7 inherent functions, 17 structs, 3 enums) plus 13
variants and 5 impl lines. Appendix A 3.7's claim, checked:
- `LoadUseCase::load(rules: SelectedRules<'_>, given: LoadGiven<'_>, kinds: &KindVocabulary) -> Result<..>` assumed
  rules arrive as compiled plans (`SelectedRule { plan: &Plan, .. }` at B133). Wrong for the tip: nothing compiles
  before load any more; load must take texts and call parse and lower itself. Corrected: `LoadRequest` of texts.
- `kinds: &KindVocabulary` is ONE language at B133 (quoted below). The tip binds per reader, i.e. per language.
  Corrected: every linked language in the request (`LinkedLanguage`), decision 2.
```
pub trait LoadUseCase: Send + Sync {  @f0219b3b3:crates/ryo/crates/engine/src/inbound/use_cases/load/interface.rs:23
kinds: &KindVocabulary,  @f0219b3b3:crates/ryo/crates/engine/src/inbound/use_cases/load/interface.rs:33
```
- `Programs::reads(&self) -> &FactDemand` (plan-wide) would duplicate operators' `PhysicalPlan::demands()` and cannot
  say what ONE file needs. Corrected: `reads(rule, language)`, per rule, so execute ORs the reads of the rules screen
  lets through (the tip's per-file `fact_mask` OR).
- `columns(&self) -> &[RuleColumns]` would duplicate operators' `PhysicalPlan::shape(node)`. Corrected: `emit(rule)`
  gives the node; its shape is operators'.
- `file_local(rule) -> bool`: kept as plan's `RuleCut` behind `cut(rule)`, since the cut is plan's (Appendix A 3.6).

### 5a. The call
```
pub trait LoadUseCase: Send + Sync {                                       @new (B133's trait, changed)
    fn load(&self, request: LoadRequest<'_>) -> LoadedRules;
}
pub struct LoadUseCaseImpl<P, L, O>;                                       // P: ryoql ParseUseCase, L: plan LowerUseCase, O: operators PlanUseCase
impl<P, L, O> LoadUseCaseImpl<P, L, O> { pub fn new(parse: P, lower: L, plan: O) -> Self; }
```
B133's trait and its answer, quoted:
```
fn load(  @f0219b3b3:crates/ryo/crates/engine/src/inbound/use_cases/load/interface.rs:30
pub enum LoadSource<'a> {  @f0219b3b3:crates/ryo/crates/engine/src/inbound/use_cases/load/types.rs:24
Pack(PackBytes<'a>, PackDigest),  @f0219b3b3:crates/ryo/crates/engine/src/inbound/use_cases/load/types.rs:29
pub struct LoadedRules {  @f0219b3b3:crates/ryo/crates/engine/src/inbound/use_cases/load/types.rs:47
pub struct RefusedRule {  @f0219b3b3:crates/ryo/crates/engine/src/domain/lowering/types.rs:150
pub enum LoadRefusal {  @f0219b3b3:crates/ryo/crates/engine/src/domain/lowering/types.rs:168
pub enum LoadError {  @f0219b3b3:crates/ryo/crates/engine/src/inbound/use_cases/load/errors.rs:30
pub struct Programs(pub(crate) ProgramsEngine);  @f0219b3b3:crates/ryo/crates/engine/src/domain/programs/types.rs:27
pub(crate) words: BaseWords,  @f0219b3b3:crates/ryo/crates/engine/src/domain/programs/types.rs:41
```
What load does, in order, each once per call: ryoql `parse` per pack (one parse per
text and per fragment); plan `lower` over the whole load (one lowering, equal steps shared, the cut computed);
symbols: intern every constant and every linked language's fixed names into one layer, then `freeze`; operators
`plan` over the whole lowered plan with that layer (shapes, gates, prepared kernels, bound tests, compiled patterns:
rows 21, 22, 30, 32, 41 in operators' sheet); then, per rule and per language, `RuleReads` from the plan's demands and
the language's `Supply`. Nothing is filled later: no `OnceLock`, no `Mutex`, no `Rc` (row 13).
- **Infallible.** Every failure is one rule's (`refused`) or one rule's on one language (`reads` answers `Err`). B133's
  whole-load errors were the pack's three (the pack is gone: "we aren't shipping rule packs, w'er enot precompliing")
  and `Vocabulary` (a language's fixed names), which the tip checks where the language is linked (`CheckedNames` in
  the language's `base_words`; symbols sheet section 3), so it is not load's.
- **Generic, not `dyn`.** Load is crossed once per call, so `dyn` would cost nothing measurable; generics are used
  anyway so the three handed use cases are the edge's choice fixed at compile time, as the owner's rule asks.

### 5b. What a caller hands in
`LoadRequest { packs, hosted, languages }`.
- `PackSource { fragments, rules }`: one module catalog and its rules. The tip groups by `Query.modules` and lowers a
  group together only if it has two or more rules and no observations (`check_session.rs:317`, `:327`); here every
  rule of a pack is lowered with the pack. Rule ordinals (`ryo_operators::RuleOrdinal`) run across packs in order.
- `RuleSource { text, parameters, observes }`. The tip's `Query` (quoted below) also carries `digest` (the cache's
  key, perf row 10: computed once per rule text, by the cache), `accepted_only` (applicability) and `every_row` (rows
  mode): both decide how execute treats a rule's rows on a file, neither changes the program, so they go to execute's
  request and do not split one program into two.
- `Parameter { name, value }`: an entry of the tip's `Parameters` (quoted below), the value operators' `PlanConstant`.
```
pub struct Query {  @2a207464c:crates/ryo/crates/executor/src/check_session/query.rs:40
pub type Parameters =  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:43
```
- `ObservedInput { name, columns: Shape, producers }` and `Producer { rule: RuleOrdinal, output }`: the producer is a
  rule OF THE SAME LOAD, named by ordinal, with no text. Today the SDK copies the producer's text in
  (`text: Arc::from(source.query.as_ref()),  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/sdk_request.rs:44`,
  having required the producer to be a rule of the same request: `.get(id)  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/sdk_request.rs:26`),
  and `query_parts` parses and lowers it again (64 ms, decision 4).
- `hosted: &[HostedRelation]`: declarations only. A host relation's rows reach a kernel per file through operators'
  `PlanInputs::hosted(HostedNumber)`, so one load serves requests with different rows (the tip keys `Admission` by host
  schema, `batch.rs:265-273`, and aliases every host table into the reader plan per visit, `shared.rs:17-33`).
- `LinkedLanguage { vocabulary, supplies, declares }`: what the tip's `bind` asks the reader on every visit
  (`declarations`, `expands`, `available`, `numbered_projections`, `kinds`), stated once. `Supply` has four answers
  (`Filled`, `Embedded`, `Tree`, `Unavailable`), the last kept because an optional relation a language does not
  advertise is an unknown, never an absence (`check_session.rs:662-663`). `LanguageNumber` is a place in that list.

### 5c. What a load answers
`LoadedRules { programs, refused }` is B133's minus `matched` (statistics: no caller at main or the tip).
`RuleRefusal` is the load-time half of the tip's refusal, with its sentences kept byte for byte:
```
pub enum Refusal {  @2a207464c:crates/ryo/crates/executor/src/check_session/refusal.rs:7
Self::MissingParameter { name } => write!(f, "unplanned query no caller gives {name}"),  @2a207464c:crates/ryo/crates/executor/src/check_session/refusal.rs:41
Self::UnknownKind { word, languages } => write!(f, "kind `{word}` names no kind of {languages}"),  @2a207464c:crates/ryo/crates/executor/src/check_session/refusal.rs:42
Self::UnknownRelation { name } => write!(f, "no reader declares file relation {name}"),  @2a207464c:crates/ryo/crates/executor/src/check_session/refusal.rs:43
None => format!("unplanned query {reason}").into(),  @2a207464c:crates/ryo/crates/executor/src/lower.rs:179
return Err(Refusal::Other(if unwritten.contains(alias.as_str()) { "missing query emit".into() } else { "unplanned query".to_owned() }));  @2a207464c:crates/ryo/crates/executor/src/lower.rs:183
```
`Unread` is new as a variant: today a syntax error becomes `RuleProgram::Unread` and surfaces as `unplanned query
<reason>`; its sentence must stay that (section 11). `Operator(OperatorFault)` is B133's `Unlowerable` as operators'
fault. `Provider` stays with execute (a per-file fault); `Other(String)` goes (every load-time sentence is typed).

### 5d. The prepared set
```
pub struct Programs;                 // Send + Sync; Clone shares (Arc); built only by load      @new (B133's, changed)
impl Programs {
    pub fn accepted(&self) -> &[RuleOrdinal];
    pub fn emit(&self, rule: RuleOrdinal) -> &RuleEmit;                       // node + subject + key columns
    pub fn cut(&self, rule: RuleOrdinal) -> &ryo_plan::RuleCut;               // plan's: file-local, or what crosses
    pub fn reads(&self, rule: RuleOrdinal, language: LanguageNumber) -> Result<&RuleReads, &RuleRefusal>;
    pub fn plan(&self) -> &PhysicalPlan;                                      // one plan, every rule an emit node
    pub fn symbols(&self) -> &FrozenSymbols;                                  // a worker takes layer()
}
```
```
pub trait EnginePort: Send + Sync + std::fmt::Debug {  @f0219b3b3:crates/ryo/crates/execution_runtime/src/outbound/engine/interface.rs:33
fn accepted(&self, programs: &Programs) -> Vec<SelectedRuleOrdinal>;  @f0219b3b3:crates/ryo/crates/execution_runtime/src/outbound/engine/interface.rs:47
fn columns(&self, programs: &Programs) -> Vec<RuleColumns>;  @f0219b3b3:crates/ryo/crates/execution_runtime/src/outbound/engine/interface.rs:50
```
- `accepted` is B133's `EnginePort::accepted` (above), as a borrowed read on the set.
- `emit` replaces `EnginePort::columns` (above) and one entry of the tip's `emits` tuple (`finish.rs:494`, above), whose subject and keys are column NAMES as text, looked up per emit.
- `plan`: ONE `PhysicalPlan` for the load. The tip lowers the batch as one plan, splits it into per-query subsets
  (`lower.rs:194-203`) and re-merges the bound subsets per visit (`check_session.rs:951-989`). The draft never splits:
  equal steps are one node for every rule (rows 8, 20), and execute's pull-based `run` makes only what the file's
  enabled emits read, so a rule screen skips costs nothing.
- `reads`: the per-rule, per-language half of the tip's `Bound` (`facts`, `fact_mask`, `named`, `projects_words`,
  `reads_fields`), made at load. The file-and-request half (host tables, layout, slots, worker-bound filter words)
  stays with execute. `RuleReads { filled: [FilledRead { relation, demand }], declared, tree: [TreeNeed] }`: "what is
  needed is passed down (which columns, which keys, or only is there a row), decided when rules are loaded".
- `symbols`: the frozen dictionary of the load (symbols sheet decision 2, option B). Constants in `plan()` are
  `Symbol`s below its count, valid in every worker's layer.

### 5e. What leaves the concept, and where to
`accepted_only`/`every_row` and `Bindings`/the file half of `Bound` to execute; `Program.requirements` and
`reader_declaration` to screen (screen prepares once per language from `Programs`: row 49); the rule digest and
`CompiledRules::identify` and `PreparedReporting::replay_closure` to cache; `PreparedReporting::metadata` to lint; main's
`CompiledPlans` cross-request cache: not carried (decision 1). All listed by item in api.txt `# NO LONGER PUBLIC`.

### 5g. Counts
| Side | Public items in the concept | After the draft |
|---|---|---|
| main | 49 (+4 in `QueryLanguagePort`) | none carried as it is; 2 calls become 1 |
| tip | 42 (my boundary, listed in 3 and in api.txt) | all stop being public |
| B133 | `LoadUseCase` and 9 types used here (quoted in 5a, 5d) | trait and `LoadedRules`, `RefusedRule`, `Programs` carried, changed |
| draft | **29** (+13 variants, +5 impl lines) | |

## 6. Proof it brings
| Proof | Source | Fate |
|---|---|---|
| compile_rules rows `compiles_each_pattern_once`, `each_module_is_parsed_once_per_session`, `a_word_for_a_kind_no_linked_language_has_is_refused_before_any_file`, `an_index_past_the_rules_is_refused_with_the_count` | main `execution/.../compile_rules/use_case_test.rs:99-135` | **restated** against `load`: a pattern two rules use is compiled once; a fragment set is parsed once per load; `UnknownKind` before any file; a `Producer` ordinal past the rules is refused with the count |
| rule_compilation rows (10) | main `execution/src/application/rule_compilation/service_test.rs:40-174` | the 8 about `CompiledPlans` (bounded FIFO, eviction, lock) go with it (decision 1); `a_blank_rule_never_reaches_the_compiler` and `blank_rule_is_refused_before_any_compile` **restated** (`RuleText::Missing` -> `Unread`, no parse) |
| features: "Distinct rules compile", "A repeated rule id is refused", "An empty request is prepared", "A binding without its rule is refused" | main `compile_rules/acceptance.feature`, `prepare/acceptance.feature` | 3 **restated** as load scenarios; "a repeated rule id" moves to lint (load takes ordinals) |
| B133 load feature, 12 scenarios | `engine/src/inbound/use_cases/load/acceptance.feature` at f0219b3b3 | **restated**: "accepts the rules it can lower and names each one it refuses", "a sub-plan two rules share is lowered once", "two rules whose plans differ only in a bound value do not share a definition", "a step of 65,536 columns is refused, naming what was counted"; not carried: the pack (2), statistics (2), the fixed-name vocabulary (language's) |
| tip `kinds_test.rs`, 5 tests | `X/check_session/kinds_test.rs:28-57` | **carried**, driven through `load` instead of `query_batch` |
| main `tests/boundary_contract/compiled_rules`, 3 tests | main `execution/tests/...` | `independent_compilation_preserves_unused_steps_and_original_frontier_order` contradicts one merged plan (unused steps are not kept); the other two **restated**; not read in full |
| `Programs: Send + Sync`, `Clone` shares | none | **MISSING**: a compile-time assertion |
| a rule observing others is lowered with the load, producers not re-parsed | none | **MISSING**: a test that counts parses (one per text) |
| binding a load twice gives the same `reads` for every rule and language; nothing in `Programs` changes after `load` | none | **MISSING** (it is what makes sharing safe) |
| a `compile_fail` doctest that `Programs` cannot be built outside load | precedent: B133 `programs/types.rs:14`, main `preparation/types.rs:64` | **MISSING** at the tip; restated |

## 7. Perf gains it carries
From `perf-gains.tsv` (54 rows, sources checked by agent H), every row whose crate column names programs:
| Row | Gain (measured) | State | What in the draft keeps it |
|---|---|---|---|
| 1 | whole pack lowered in one call; partition indexes closed readers once (start-up 341.3 -> 317.9 ms) | live | `load` takes every pack of the call and calls plan's `lower` once; the cut is computed once inside it and read through `cut(rule)`, never per source or per file |
| 8 | pack lowered once per request (lowering and parsing 888 -> 586 M instructions) | live | one lowering per load; each rule is an emit node of the one plan, not its own lowered plan; no group-of-two threshold, no observation exclusion (`check_session.rs:317, 327`) |
| 13 | many workers: plan, constants, patterns, symbol tables frozen once (ten cores fastapi 3.619 -> 1.380 CPU-s, 838 -> 387 MiB) | parked:89d13a19e | `Programs` is `Send + Sync` with no `Rc`, lock or lazy cell; it holds operators' `PhysicalPlan` (prepared kernels, bound tests, patterns) and `FrozenSymbols`; per-language `reads` made at load, so a worker binds nothing (today `check_session.rs:979`, per worker per visit) |
| 25 | executor bound from the typed plan, no JSON plan (start-up django 166.1 -> 77.6 ms) | live | load calls operators' `plan` on the lowered plan in process; no text, JSON or encoded plan between lowering and run; no `LoadSource::Pack` |
| 32 | filters compiled at plan load (cold-list 2.144 -> 0.137 s) | live | done by operators' `plan` inside load, once per node of the merged plan |
| 41 | per-node data flat at plan load (django 26.49 -> 18.14) | live | the same: `PhysicalPlan` is built complete in load (the tip fills `args` and `frames` lazily, `finish.rs:486, 491`) |
| 46 | build 133's engine: table form, needs, rule preparation (prep 2.29 -> 1.59) | lost | its `LoadUseCase`, `LoadedRules`, `RefusedRule`, opaque `Programs` are restored (changed) |
| 48 | fused engine: compiled program cached (compile was 62% of a one-file run) | lost | not carried as a cache (decision 1); its point, compile not repeated per file, is kept by one load per call |
| 49 | requirement cache per prepared program and reader (requirement programs 5,110 -> 552) | live | `Programs` is fixed and the linked languages are known at load, so screen computes requirements once per (load, language) into its frozen screens; nothing is cached lazily per reader |
Related rows that depend on this interface though their crate column names another: 6 (one admission: lint calls
`load` once per call, the SDK's second admission goes), 14 (a full cache hit prepares nothing: `load` is separate from
keying and can be skipped), 20 (equal steps shared: one plan), 53 (only file-local programs replayed: `cut(rule)`).
Room it makes, not yet a gain: the 64 ms RUF100 re-lowering (producers by ordinal) and the 103 ms per-rule typing
(operators' `plan` types each merged node once, not each program in a throwaway run, `batch.rs:149-203`).

## 8. Benchmarks that cover it
- **Start-up** (Python 74 ms bar, 296 ms today) is measured by the lint images (`vec`, `vec_ts`, `vec_rust`,
  `executor/benches/lint_image`) with a zero-file request (`trial-1/summary.txt`), through the SDK only: no programs
  name is reached, so the draft cannot break them. Every millisecond of it is load (section 4).
- **Ten-worker wall** and **one edit** (start-up is inside the 301 ms findings-only edit) are measured the same way.
- **The Tastemaker benchmark project** reaches the tip's preparation by name:
```
use ryo_executor::check_session::replay;  @2a207464c:crates/tastemaker/benchmarks/project/live/capture.rs:4
let mut work = batch.register_query(query.into(), req, |root| self.inventory(root, closed), None, layouts.map(|readers| (readers, None)))?;  @2a207464c:crates/ryo/crates/executor/src/check_session/replay.rs:88
```
  After the draft it calls the SDK (or lint), which calls `load`; what it measures is unchanged.
- The bench-hooks clock `clocks.prepare` (`batch.rs:223`, `check_session.rs:914, 991`) is the only by-concept number
  today; after the draft it is one clock around `load`.
- Nothing benchmarks load alone on any side. A bench over the Ruff pack (413 rules, no files) needs no image build and
  would give start-up its own number (`review-performance.md` (d)); proposed, not written.

## 9. Checks
IMPORTS: table, symbols, relations, operators, ryoql, plan
CROSSED: per request
- Imports: all domain and all before programs in the order (`table`, `symbols` < `relations` < `operators` < `ryoql` <
  `plan` < `programs`). No outbound crate: the languages enter as data (`LinkedLanguage`), which each language crate
  fills (outbound imports domain). Outside crates: none needed by the interface.
- Layout words: none in a signature (no `Rc`, `Vec<Vec<u64>>`, row, column store). `Vec` appears in request data
  fields (`supplies`, `declares`, `producers`, `refused`) as in the ryoql and symbols listings; the nominal-type rule
  may object (section 11).
- Crossing: `load` once per lint call. The `Programs` reads are inherent methods on one concrete type, read per file
  and rule by screen and execute; no trait, no `dyn`, inlined.
- Cycle trap (PLAN 10.5): programs is last in the domain order and nothing below names it; the kind check moves to plan
  (ryoql decision 2), so programs reads plan's refusal and not ryoql's.

## 10. Decisions for the owner              (each: the example from the code; why; what we get; my recommendation)
1. **One load per lint call, or per request, or cached across calls.** Example: the SDK prepares per request round
   (`entry.rs:599-605`) and dedups in the session's `prepared: HashMap<(Query, Parameters), usize>`
   (`check_session.rs:257`); main kept compiled plans across requests in a bounded `CompiledPlans` cache
   (`rule_compilation/service.rs:18-23`). Why: the Tastemaker check submits three rounds (row 17) and row 19 shares
   screens across every request of a run. What we get: one frozen set per `Linter::lint` call over the union of its
   rules (deduplicated by lint), one admission (row 6). **Recommend: one load per call; `Programs` is `Clone`, so lint
   may keep it for an equal next call; no plan cache inside programs.**
2. **Bind per language at load, or per worker per visit.** Example: `check_session.rs:951-989` binds every needed
   program again in every worker's visit, against the reader and the worker's own dictionary. Why: that is the measured
   ten-worker cost (1.637 s against 0.351 s) and the reason the prepared set cannot be shared. What we get: `reads(rule,
   language)` made once; a worker binds nothing. **Recommend: at load, with the linked languages in the request.**
3. **One merged plan for the load, or a plan per rule.** Example: `lower.rs:194-203` splits the one lowered plan into
   renumbered per-query plans; `shared.rs:42-54` merges them back per visit. Why: split-then-merge costs start-up and
   per-visit preparation and keeps two plan shapes. What we get: one `PhysicalPlan`, every rule an emit node, equal
   steps one node (rows 8, 20). Risk: a merged node shared by a file-local rule and a cross-file rule must be cut per
   rule (plan sheet). **Recommend: one merged plan.**
4. **Observations name producers by ordinal in the same load.** Example: `lower.rs:101` pushes each producer's text
   into the observing rule's own pack, so RUF100 re-parses and re-lowers 410 rules (64 ms of Python start-up). The SDK
   already refuses a producer outside the request (`sdk_request.rs:25-27`). What we get: one parse and one lowering per
   text. **Recommend: yes.**
5. **Who owns `LinkedLanguage`.** Example: the tip asks the reader per visit (`declarations`, `expands`, `available`,
   `numbered_projections`, `kinds`, `check_session.rs:620-652`); B133 took one `KindVocabulary`
   (`values::descriptions`). Why: programs is domain and may not import the language contract; the data must be domain
   and filled by each language. What we get: one description per language, stated once. **Recommend: programs owns
   `LinkedLanguage` and `Supply`; its `vocabulary` is plan's `KindVocabulary` (one definition, the kind check reads it);
   the language sheet adds the one call that fills it.**

## 11. Not checked
- Quotes: all 94 `@rev:path:line` quotes in this sheet were re-checked by script against `git show "REV:PATH"`
  (exact line text after trimming): 94 match, 0 differ. The first pass found 8 that failed: 7 inline quotes wrapped
  over two lines (now one line each, in blocks) and 1 wrong path (`RefusedRule` is in B133's `domain/lowering/types.rs`,
  not `load/types.rs`, as the discovery notes had it); all fixed before the final pass. Inline `path:line` references
  in prose (no `@`) point at lines quoted in a block or were checked by hand one at a time (`check_session.rs:43, 662-663, 914, 991`,
  `batch.rs:223, 265-273`, `shared.rs:17-33`, `tastemaker.toml:16-17, 31, 76, 105`, `preparation/types.rs:64`,
  `programs/types.rs:14`, `core/src/application/engine/service.rs:47, 87`). `api.txt` carries tags, not quotes.
- The plan sheet does not exist: `ryo_plan::LowerUseCase`, `KindVocabulary`, `RuleCut` are working names. Whether plan
  can lower several packs into one plan with shared steps across packs, and cut a node shared by a file-local and a
  cross-file rule per rule, is not checked.
- `ryo_relations::RelationName` and `HostedRelation` are named by the ryoql listing and here but are not in the
  relations listing; `ryo_ryoql::ObservationName` is ryoql's decision 5, not yet in its listing.
- Whether operators' `plan` can take the whole load's `LogicalProgram` (thousands of nodes for 413 rules) within the
  start-up bar is not measured; nor whether one merged `FileBuffers` sized to the whole plan costs memory per worker.
- Hosted relations: one load assumes one set of host declarations per call. Whether the Tastemaker check's requests in
  one call declare different host schemas for the same rule (the tip keys `Admission` by schema) is not checked; if they
  do, lint loads once per distinct schema set.
- `Supply`'s four answers are drafted from the reader trait's methods; whether they cover every case of `bind`
  (`host_columns::supplies`, `RowScope::Project` refusals, `FilesystemExecutable`, line counts) is not traced.
- The `Unread` sentence: whether every syntax error reaches the user today as exactly `unplanned query <reason>` was not
  traced through `lowered`'s `unplanned` list.
- Lines without tests on the tip are by range (my ranges), not by file; the tip's 42 items depend on where the line is
  drawn in `check_session.rs` and `batch.rs`.
- The 426 `Rc` count in the plan is for the whole executor (`git grep -c` summed: 426), not for the prepared set.
- Main's 3 `boundary_contract/compiled_rules` tests and the B133 load feature's step definitions were read by name only.
- No `tm check` on a skeleton of the draft: tuples and `Vec` in public fields of `LinkedLanguage` may fail the
  nominal-type rule; `reads` returning `Result<&_, &_>` may too.
