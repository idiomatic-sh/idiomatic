# domain/plan
STATUS: drafted (28 Nov; not reviewed, not agreed). Bead ryo-fmx3.1.13.23.4.8.

Revisions: MAIN = f4ae3bd802, TIP = 2a207464c. Every quote is one line, two spaces, `@rev:path:line`, re-checked with
`git show "REV:PATH" | sed -n 'Np'` (count in section 11). "X" below is `crates/ryo/crates/executor/src` at TIP; "QL" is
`crates/ryo/crates/query_language/src`; "V" is `crates/ryo/crates/values/src/domain`.

## 1. Owns
The plan: a request's checked rule programs lowered to one plan of shared steps, with what that plan decides before
the first file is read (the passes' marks, each rule's file/cross-file cut, the kind words it asks of the languages).
One thing: lowering, the passes, the cut and the kind words are all facts of one value, the plan, computed once when
rules load. What it does NOT own: parsing and the typed checks (ryoql), what an operator means, which inputs and
columns it reads and its output shape (operators), binding and freezing a request's programs for the workers
(programs).

## 2. On main (f4ae3bd802)
**Where.** Three crates. The planner was query_language's `compile` use case: the older lowering plus 13 optimizer
passes, producing `values::plans::Plan`, a plan that carried its own file/cross-file cut. The kind check was
execution_domain's. `execution`'s `compile_rules` called the planner per rule through a port (that use case is
preparing rules: the programs sheet's).
```
pub trait CompileUseCase: Send + Sync {  @f4ae3bd802:crates/ryo/crates/query_language/src/inbound/use_cases/compile/interface.rs:8
fn compile(&self, request: QueryRequest) -> Result<CompileReport, CompileError>;  @f4ae3bd802:crates/ryo/crates/query_language/src/inbound/use_cases/compile/interface.rs:14
pub struct CompileReport {  @f4ae3bd802:crates/ryo/crates/query_language/src/inbound/use_cases/compile/types.rs:10
pub enum CompileError {  @f4ae3bd802:crates/ryo/crates/query_language/src/inbound/use_cases/compile/errors.rs:7
pub fn new(syntax: Arc<dyn SyntaxPort>, catalog: Arc<dyn CatalogPort>) -> Self {  @f4ae3bd802:crates/ryo/crates/query_language/src/inbound/use_cases/compile/use_case.rs:35
let plan = lower_with_observations(  @f4ae3bd802:crates/ryo/crates/query_language/src/inbound/use_cases/compile/use_case.rs:96
pub struct Plan {  @f4ae3bd802:crates/ryo/crates/values/src/domain/plans/types.rs:23
pub frontier: Vec<Input>,  @f4ae3bd802:crates/ryo/crates/values/src/domain/plans/types.rs:38
pub struct PlanStep {  @f4ae3bd802:crates/ryo/crates/values/src/domain/plans/types.rs:56
pub scope: StepScope,  @f4ae3bd802:crates/ryo/crates/values/src/domain/plans/types.rs:60
pub enum StepScope {  @f4ae3bd802:crates/ryo/crates/values/src/domain/plan_parts/types.rs:313
pub(crate) fn scope_of(  @f4ae3bd802:crates/ryo/crates/query_language/src/domain/planning/service.rs:76
pub(crate) fn frontier_of(steps: &[plans::PlanStep]) -> Vec<parts::Input> {  @f4ae3bd802:crates/ryo/crates/query_language/src/domain/planning/service.rs:184
pub(crate) fn cut_handover(steps: Vec<PlanStep>, emitted: StepId) -> Planned {  @f4ae3bd802:crates/ryo/crates/query_language/src/domain/cutting/service.rs:28
pub fn first_unknown<P: std::borrow::Borrow<Plan>>(  @f4ae3bd802:crates/ryo/crates/execution_domain/src/domain/kind_check/service.rs:200
pub struct UnknownKind {  @f4ae3bd802:crates/ryo/crates/execution_domain/src/domain/kind_check/types.rs:33
pub trait CompileRulesUseCase {  @f4ae3bd802:crates/ryo/crates/execution/src/inbound/use_cases/compile_rules/interface.rs:10
pub trait QueryLanguagePort: Send + Sync {  @f4ae3bd802:crates/ryo/crates/execution/src/outbound/query_language/interface.rs:11
```
So main had the design this sheet restores: each step's `scope` (per file or cross-file) and the plan's `frontier`
(what per-file steps hand over) decided by the planner, and `cut_handover` cutting a handed-over relation to the
columns later steps read. Main's kind check took the pattern matcher handed in (`first_unknown(plans, bindings,
vocabularies, matcher: PatternMatcher<'_>)`).

**Lines without tests** (counted by script over `git show`, test, fixture and `_test` files excluded): compile use case
360; the 13 passes 4,814 (`capture_pushing` 160, `cutting` 329, `distinctness` 204, `fusing` 722, `inlining` 387,
`lineage` 317, `narrowing` 237, `pruning` 80, `pushing` 436, `regrouping` 422, `selection_pushing` 1,020, `walking` 414,
`widths` 86); `values::plans` 620; execution_domain `kind_check` 333. **6,127.** (The older lowering it ran first,
`lowering` 599 and `planning` 307, is still at the tip and is ryoql's: ryoql decision 1.)

**Public items in the concept: 27 (+1 method).** compile 5 (`CompileUseCase`, `CompileReport`, `CompileError`,
`CompileUseCaseImpl`, `new`) + `compile`; `values::plans` 11 (`Plan`, `RuleReads`, `PlanStep`, `Operation`, `Emit`,
`inputs_of`, `named`, `reads_of`, `each_input`, `kind_of`, `comparisons_of`); kind_check 10 (`kinds_asked`,
`first_unknown`, `unknown_message`, `refusal`, `AskedKind`, `RulePlace`, `UnknownKind`, `KindOrigin`, `KindMessage`,
`KindRefusal`); `StepScope` 1.

**Callers (production files).** `CompileUseCase`: execution's query-language adapter and factory (3 files), the SDK's
lint, query run and query validate compositions (3), the command line's query run and validate compositions (2).
kind_check: execution's `rule_compilation` (2 files), `compile_rules` (1), `metadata` (3). A step's scope or the plan's
frontier is read in 74 files across 8 crates (`git grep -l -E "\.frontier\b|StepScope::CrossFile"`: execution 28,
execution_runtime 13, core 9, execution_domain 8, operators 6, query_language 6, values 3, sdk 1).

**Proof.** compile: 44 `#[test]`, 198 case rows, 2 scenarios (`compile/acceptance.feature`). The 13 passes: 23
`#[test]`, 78 rows (`selection_pushing` 19, `fusing` 11, `walking` 8, `cutting` 6, `distinctness` 6, `pushing` 6,
`regrouping` 6, `widths` 5, `narrowing` 3, the other four 2 each). `values::plans` 1 test. kind_check 1 test over 20
rows (`kind_check/service_test.rs`). For example the cut's own property, as a row:
```
name: CaseName("a_relation_asked_for_one_column_is_cut_to_it"),  @f4ae3bd802:crates/ryo/crates/query_language/src/domain/cutting/service_test.rs:42
```
**Tastemaker.** `crate-query-language` and `nu-hex-query-language` (`crates/ryo/tastemaker.toml:129,190`), `crate-values`
and `nu-hex-values` (`:132,193`); `entry-traits-witnessed` names `compile_rules/interface.rs` (`:76`). None names the
planner's passes.

## 3. On the perf branch (2a207464c)
**Where.** Spread over four places, none of them a crate of its own:
- QL `domain/plan_lowering` (2,951 lines without tests; moved from build 133's `engine` crate at 04f8f4ec1b, 27 Nov):
  the lowering (`expansion`), the passes (`semijoins`, `counts`, `axes`, `pushdown`), step numbering with hash-consing
  (`numbering`), column use (`readers`), and a fourth list of the operators (`operators/service.rs`, a `match` with 24
  operator-name arms building typed steps):
  ```
  pub fn lowered(pack: &PackPrograms, stage: PlanningStage) -> FinishPlan {  @2a207464c:crates/ryo/crates/query_language/src/domain/plan_lowering/service.rs:25
  pub fn mark_through(plan: &mut FinishPlan, stage: PlanningStage) {  @2a207464c:crates/ryo/crates/query_language/src/domain/plan_lowering/service.rs:32
  pub fn passed(plan: &mut FinishPlan, stage: PlanningStage) -> usize {  @2a207464c:crates/ryo/crates/query_language/src/domain/plan_lowering/service.rs:61
  pub struct SharedSteps {  @2a207464c:crates/ryo/crates/query_language/src/domain/plan_lowering/shared.rs:11
  pub fn append(&mut self, mut step: FinishStep, inputs: &[usize]) -> usize {  @2a207464c:crates/ryo/crates/query_language/src/domain/plan_lowering/shared.rs:19
  pub fn finish(self) -> Vec<FinishStep> {  @2a207464c:crates/ryo/crates/query_language/src/domain/plan_lowering/shared.rs:120
  pub struct PackPrograms {  @2a207464c:crates/ryo/crates/query_language/src/domain/plan_lowering/types.rs:23
  pub struct RuleContext {  @2a207464c:crates/ryo/crates/query_language/src/domain/plan_lowering/types.rs:43
  pub struct ContextTable {  @2a207464c:crates/ryo/crates/query_language/src/domain/plan_lowering/types.rs:64
  pub struct ContextObservation {  @2a207464c:crates/ryo/crates/query_language/src/domain/plan_lowering/types.rs:86
  pub struct ObservationProducer {  @2a207464c:crates/ryo/crates/query_language/src/domain/plan_lowering/types.rs:108
  pub struct PackRule {  @2a207464c:crates/ryo/crates/query_language/src/domain/plan_lowering/types.rs:127
  pub enum RuleProgram {  @2a207464c:crates/ryo/crates/query_language/src/domain/plan_lowering/types.rs:144
  pub struct UnreadQuery(pub Box<str>);  @2a207464c:crates/ryo/crates/query_language/src/domain/plan_lowering/types.rs:161
  pub enum PlanningStage {  @2a207464c:crates/ryo/crates/query_language/src/domain/plan_lowering/types.rs:173
  pub struct WrittenText(pub Box<str>);  @2a207464c:crates/ryo/crates/query_language/src/domain/plan_lowering/types.rs:194
  pub enum PlanLoweringError {  @2a207464c:crates/ryo/crates/query_language/src/domain/plan_lowering/errors.rs:17
  pub(crate) fn operator_step(  @2a207464c:crates/ryo/crates/query_language/src/domain/plan_lowering/operators/service.rs:46
  pub(crate) fn interned(  @2a207464c:crates/ryo/crates/query_language/src/domain/plan_lowering/numbering/service.rs:17
  pub(super) fn mark(plan: &mut FinishPlan, uses: &Uses) {  @2a207464c:crates/ryo/crates/query_language/src/domain/plan_lowering/pushdown.rs:17
  pub(crate) fn used(plan: &PlanReaders<'_>, memo: &mut [Worked], step: StepNumber, column: &str) -> bool {  @2a207464c:crates/ryo/crates/query_language/src/domain/plan_lowering/readers/service.rs:166
  ```
  It is reached because main's closed `domain` was opened: `pub mod kind_check;  @2a207464c:crates/ryo/crates/query_language/src/domain.rs:15`
  and `pub mod plan_lowering;  @2a207464c:crates/ryo/crates/query_language/src/domain.rs:16`.
- QL `domain/kind_check` (197 lines), which reads the plan and builds a matcher inside:
  ```
  pub fn asked(plan: &FinishPlan) -> Vec<(Test, String)> {  @2a207464c:crates/ryo/crates/query_language/src/domain/kind_check/service.rs:18
  pub fn names_kind(test: Test, word: &str, vocabularies: &KindVocabularies) -> bool {  @2a207464c:crates/ryo/crates/query_language/src/domain/kind_check/service.rs:115
  let matching = MatchingUseCaseImpl::new();  @2a207464c:crates/ryo/crates/query_language/src/domain/kind_check/service.rs:116
  pub fn unknown_word(plan: &FinishPlan, vocabularies: &KindVocabularies) -> Option<String> {  @2a207464c:crates/ryo/crates/query_language/src/domain/kind_check/service.rs:137
  pub fn unknown_message(word: &str, vocabularies: &KindVocabularies) -> String {  @2a207464c:crates/ryo/crates/query_language/src/domain/kind_check/service.rs:143
  pub fn located(program: &WrittenProgram, word: &str) -> Option<(QuerySpan, Placed)> {  @2a207464c:crates/ryo/crates/query_language/src/domain/kind_check/location.rs:34
  ```
- V `finish_plans` (1,901 lines, 166 public items: 96 const, 49 struct, 17 enum, 4 fn). Of these only 8 are the plan's
  own; the rest are the step enum, its argument types, the expressions and the operator and argument names
  (operators'):
  ```
  pub struct FinishPlan {  @2a207464c:crates/ryo/crates/values/src/domain/finish_plans/types.rs:33
  pub struct PlannedRule {  @2a207464c:crates/ryo/crates/values/src/domain/finish_plans/types.rs:54
  pub enum RuleEmit {  @2a207464c:crates/ryo/crates/values/src/domain/finish_plans/types.rs:73
  pub struct PlanEmit {  @2a207464c:crates/ryo/crates/values/src/domain/finish_plans/types.rs:93
  pub struct EmitSubject(pub Box<str>);  @2a207464c:crates/ryo/crates/values/src/domain/finish_plans/types.rs:112
  pub struct UnplannedRule {  @2a207464c:crates/ryo/crates/values/src/domain/finish_plans/types.rs:126
  pub ungiven: Option<Box<str>>,  @2a207464c:crates/ryo/crates/values/src/domain/finish_plans/types.rs:132
  pub struct UnplannedReason(pub Box<str>);  @2a207464c:crates/ryo/crates/values/src/domain/finish_plans/types.rs:145
  pub enum FinishStep {  @2a207464c:crates/ryo/crates/values/src/domain/finish_plans/types.rs:160
  pub struct SelectStep {  @2a207464c:crates/ryo/crates/values/src/domain/finish_plans/types.rs:231
  pub enum ProbeMark {  @2a207464c:crates/ryo/crates/values/src/domain/finish_plans/types.rs:329
  pub fn readers(steps: &[FinishStep]) -> Vec<Vec<StepNumber>> {  @2a207464c:crates/ryo/crates/values/src/domain/finish_plans/service.rs:95
  use crate::domain::runs::types::RuleId;  @2a207464c:crates/ryo/crates/values/src/domain/finish_plans/types.rs:20
  ```
- The executor: its own driver of the lowering (X/`lower.rs`, 249 lines), the cut computed late over the executor's
  `Exec` and `Run` (X/`finish/partition.rs`, 525 lines, 0 tests, with its node demand in X/`finish/session/demand.rs`,
  598 lines), and the kind refusal (X/`check_session/kinds.rs`, 32 lines):
  ```
  pub fn query_parts(rule: &str, text: &str, mut context: RuleContext, keep: bool, separator: Option<&str>, modules: ParsedModules, observations: &[Observation], literals: &mut SharedPlanData) -> Result<(super::finish::Plan, ReadPack), Refusal> {  @2a207464c:crates/ryo/crates/executor/src/lower.rs:90
  pub fn query_batch(  @2a207464c:crates/ryo/crates/executor/src/lower.rs:120
  pub fn finish_plan(pack: &ReadPack, keep_for_first: bool, separator: Option<&str>, emit: Option<&str>, literals: Option<&mut SharedPlanData>) -> Result<super::finish::Plan, Refusal> {  @2a207464c:crates/ryo/crates/executor/src/lower.rs:207
  pub struct SharedPlanData {  @2a207464c:crates/ryo/crates/executor/src/lower.rs:76
  pub struct Partition {  @2a207464c:crates/ryo/crates/executor/src/finish/partition.rs:36
  pub columns: BTreeMap<usize, Vec<bool>>,  @2a207464c:crates/ryo/crates/executor/src/finish/partition.rs:41
  pub filters: BTreeMap<usize, Vec<(usize, Vec<u8>)>>,  @2a207464c:crates/ryo/crates/executor/src/finish/partition.rs:42
  pub file_steps: Vec<bool>,  @2a207464c:crates/ryo/crates/executor/src/finish/partition.rs:44
  pub fn partition(  @2a207464c:crates/ryo/crates/executor/src/finish/partition.rs:328
  exec.schema(run, i)?;  @2a207464c:crates/ryo/crates/executor/src/finish/partition.rs:191
  pub fn unknown(plan: &FinishPlan, languages: impl FnOnce() -> Vec<(String, Vec<String>)>) -> Option<Refusal> {  @2a207464c:crates/ryo/crates/executor/src/check_session/kinds.rs:11
  ```
  The typecheck use case lowers through the same functions for `query validate` (QL `typecheck/keeper.rs`, 63 lines):
  ```
  pub(super) fn lowered_plan(root: WrittenProgram, modules: ParsedModules) -> Result<(FinishPlan, usize), String> {  @2a207464c:crates/ryo/crates/query_language/src/inbound/use_cases/typecheck/keeper.rs:30
  fn typecheck_kinds(&self, request: QueryRequest, kinds: &KindVocabularies) -> Result<ValidateReport, TypecheckError>;  @2a207464c:crates/ryo/crates/query_language/src/inbound/use_cases/typecheck/interface.rs:23
  ```

**Lines without tests:** plan_lowering 2,951 + kind_check 197 + finish_plans 1,901 + lower.rs 249 + partition.rs 525 +
kinds.rs 32 + keeper.rs 63 = **5,918**, plus the part of demand.rs (598) that describes a cut input's node demand.
Of finish_plans, the plan's own types are about 125 lines (`types.rs:22-146`); the rest is operators'.

**Public items in the concept: 49 (+2 methods).** plan_lowering 15 (+`append`, `finish`); kind_check 6; the plan's own
8 of finish_plans (`FinishPlan`, `PlannedRule`, `RuleEmit`, `PlanEmit`, `EmitSubject`, `UnplannedRule`,
`UnplannedReason`, `readers`); partition.rs 9; lower.rs 10 (`PackTexts`, `parsed`, `ReadPack`, `Producer`,
`Observation`, `SharedPlanData`, `intern`, `query_parts`, `query_batch`, `finish_plan`); kinds.rs 1.

**Interface actually called, and by whom.** No trait: every call is a free function or a public field.
- `lowered`/`mark_through`: X/`lower.rs` (twice, `Expanded` then `Axes`: lines 141/143 and 208/219) and QL
  `typecheck/keeper.rs` (lines 44/48). `passed` is imported by lower.rs (line 4) and never called.
- `SharedSteps`: X/`lower.rs:194` (per-query subset) and X/`check_session/shared.rs:11` (one plan per reader, merged
  per worker while binding: `let slot = self.steps.append(step, slots);  @2a207464c:crates/ryo/crates/executor/src/check_session/shared.rs:52`).
- `RuleContext`/`ContextTable`: X/`check_session.rs:91`, X/`check_session/replay/batch.rs:230`, X/`request_input.rs`
  (the benchmark request's caller values).
- The cut: computed once per program at admission, in
  `pub fn prepare(id: usize, program: &Program, hosted: &[Hosted], catalog: &session::named::Catalog, patterns: &finish::Patterns) -> Result<Self, crate::check_session::refusal::Refusal> {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/batch.rs:149`,
  `let local = finish::partition::file_steps(&mut exec, &mut run, |name| {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/batch.rs:173`,
  `Some(Arc::new(finish::partition::partition(&mut exec, &mut run, local)?))  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/batch.rs:203`;
  `Partition` is then read in 8 executor files (check_session.rs, frontier.rs, pairs.rs, closed_workers.rs,
  readers.rs, worker.rs, batch.rs, finish/session/contribution.rs).
- The kind check: `self.unknown_kind.get_or_init(|| kinds::unknown(&self.typed, languages)).clone()  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:131`
  (once per program) and QL `typecheck/use_case.rs:148` (validate).
- A rule's lowering is driven per rule today: `let (plan, pack) = lower::query_parts(alias, &query.text, context, true, capture_separator, modules, &query.observations, literals)?;  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:96`,
  and per module catalog for batched queries (`query_batch`, perf row 8). The executor's lowering returns the
  executor's own plan, not the typed one: `let executor_plan = super::finish::from_typed(plan.clone());  @2a207464c:crates/ryo/crates/executor/src/lower.rs:244`.

**Proof.** plan_lowering: 6 `#[test]` over 7 rows (`axes` 1, `counts` 1, `joins` 4, `semijoins` 1; and
`operators/vocabulary_test.rs` 2 tests comparing the operator names the checks know with those the plan lowers).
kind_check, finish_plans, lower.rs, partition.rs, demand.rs, kinds.rs: **0 tests**. Typecheck's 3 success rows compare
`steps`, which this sheet takes (ryoql sheet section 6).

**Tastemaker.** `idiomatic.ryo/fact-reads-through-lookup` includes `crates/query_language/src/domain/plan_lowering/**`
(`crates/ryo/tastemaker.toml:70`); `crate-query-language`, `nu-hex-query-language` (`:119,180`); `crate-values`,
`nu-hex-values` (`:122,183`). Nothing names the executor's lowering or cut.

## 4. What changed between them
- **Removed (main's planner):** the `compile` use case, the 13 optimizer passes, `compile_rules`, `rule_compilation`
  and the query-language port, at fd6122c42a (27 Nov, "query language: one front end: the checks, then the executor's
  plan lowering; the optimizer, the compile use case, compile_rules, rule_compilation and the query-language port are
  deleted"); main's kind check at c46607e439 (27 Nov). `values::plans::Plan`, `Emit` and `RuleReads` are absent at
  the tip; `PlanStep`/`Operation` remain, read only by ryoql's older lowering.
- **Added (the prototype's lowering):** finish plan types and passes 8d3ced6f96 (22 Nov), moved to values at
  c97750cc82 (22 Nov); the shared reader plan 5aee5787c1 (24 Nov, perf row 20); pushdown 63e72f56a8 (24 Nov, row 16);
  literal, projection and predicate interning ec4081a6bf (24 Nov), ba2550e397, f0b9db3b86, 71c6843dc0 (25 Nov); the
  pack lowered once per module catalog 28017029a9 (25 Nov, row 8); the kind check once over the plan that runs
  f0db2d155a (27 Nov); plan_lowering into query_language 04f8f4ec1b (27 Nov, row 1); the cut indexes closed readers
  once ceb2047110 (27 Nov, row 1). 23 commits touch the engine-era plan_lowering, 27 touch partition.rs.
- **Changed place of the cut:** on main the planner decided each step's scope and the frontier; on the tip the cut is
  computed in the executor at admission, over the executor's numbering, from step schemas the executor evaluates
  (`exec.schema`, above). The plan type itself no longer carries it.
- **Lost that should not have been:** (1) the cut as part of the plan (main's design) and its tests: main's `cutting`
  6 rows and `planning` 2 rows; the tip's cut has 0 tests; (2) the kind check's 20 rows (main) for a check that has 0
  tests at the tip; (3) the compile seam suite (44 tests, 198 rows) and the passes' 23 tests / 78 rows, whose subject
  (main's optimizer) is gone: only the cut's property survives in today's code; (4) main's closed `domain` in
  query_language (opened at the tip to export the lowering).

## 5. The drafted public interface        (explains every line of api.txt that is changed or new)
`api.txt` beside this sheet: **26 public items, 8 methods, 20 variants, 2 impl lines**. Corrections to Appendix A
§3.6 (`PLAN.md:1223-1237`):
- `FinishPlan.needs: Vec<Demand>` is dropped: what a plan reads of each relation is operators'
  `PhysicalPlan::demands()` (operators api.txt), computed in operators' `plan`; a second copy here would be two
  definitions of one concept. The plan's column use (`readers/service.rs:166`) stays private to the passes.
- `FinishPlan.cut` becomes one `Cut` per planned rule (`PlannedRule.cut`), because the tip computes it per program and
  its subtree-class rule depends on the whole program (`file_steps_by` takes the plan's single subtree-class step).
- `LowerUseCase::lower(programs, stage)` loses the stage and gains the hosted relations (the cut needs which caller
  tables hold File cells).
- `unknown_kind` is kept; `judge` (validate's half) and `rule_plan` are added because they have callers today
  (`typecheck_kinds`, `query_batch`); `logical` is added because operators' draft expects the plan to write
  `LogicalProgram`.

### 5a. The use case
```
pub trait LowerUseCase: Send + Sync {  @new
fn lower(&self, pack: &PackPrograms, hosted: &[HostedRelation]) -> Plan;  @new
fn rule_plan(&self, plan: &Plan, rule: &RuleId) -> Result<Plan, PlanError>;  @new
fn logical<'p>(&self, plan: &'p Plan) -> LogicalProgram<'p>;  @new
fn unknown_kind(&self, plan: &Plan, kinds: &KindVocabularies) -> Option<UnknownKind>;  @new
fn judge(&self, query: &CheckedQuery, kinds: &KindVocabularies) -> Result<StepCount, PlanError>;  @new
```
- `lower` is the three call pairs `lowered(.., Expanded)` then `mark_through(.., Axes)` (lower.rs 141/143, 208/219;
  keeper.rs 44/48) as one call, with the literal interning the executor does after it (`SharedPlanData`, lower.rs
  144-161, 226-242) moved inside, and the cut of each rule computed (5c). The stage goes private: every caller runs to
  `Axes`. The retain the executor does between the two stages
  (`if let Some(rule) = emit { plan.rules.retain(|r| r.rule.0.as_ref() == rule); }  @2a207464c:crates/ryo/crates/executor/src/lower.rs:218`)
  becomes `RuleRole::Observed` on the producers, so the passes see exactly the emitting rules, as today.
- `rule_plan` is `query_batch`'s loop (lower.rs 175-204): the steps one rule's emit reaches, renumbered through
  `SharedSteps`, refused as today when unplanned or without an emit. Keeper's step count is `rule_plan(..).steps.len()`.
- `logical` writes operators' `LogicalProgram` from the plan: each node's `OperatorCall`, its inputs (steps, relations
  numbered once, the tree, hosted and observed inputs, frontier places from the cut) and its `StepScope` from the cut.
  It replaces the logical half of
  `pub fn from_typed(plan: Arc<FinishPlan>) -> Plan {  @2a207464c:crates/ryo/crates/executor/src/finish/plan_load.rs:51`;
  the physical half is operators' `PlanUseCase::plan`, and binding it for workers is programs'.
- `unknown_kind` is `kinds::unknown` (above) with `asked` and the per-language loop private, and the comparison of a
  word with a kind (equal, prefix, suffix, glob, regex) asked of operators' `CompareUseCase::holds` instead of
  `MatchingUseCaseImpl::new()` built inside `names_kind`.
- `judge` is the plan half of `typecheck_kinds` (ryoql decision 2): lower the checked query with placeholder
  parameters (keeper.rs 19-27), refuse what the lowering refuses, run the kind check when kinds are given, and answer
  the steps of the rule's plan; a query with observations answers the checks' own count, as today (use case
  lines 154-160). It takes ryoql's `CheckedQuery`, so nothing is parsed twice. The `ValidateReport` (relations read,
  operators used, steps) is assembled by inbound/validate from `CheckedQuery` and this count, and the unknown word is
  placed there with ryoql's `locate` (decision 3).

### 5b. How it is built
`LowerUseCaseImpl<C: CompareUseCase>::new(compare: C)`  `@new`. The only thing handed in is the comparisons the kind
check needs; the catalog, the operators' shapes and the relations are domain crates below and are called directly. The
type parameter keeps the call static (no `dyn`); every method is crossed per request or per rule of a request.

### 5c. The cut
```
pub struct Cut { pub steps: Vec<StepScope>, pub crossing: Vec<Crossing>, pub emit: StepScope }  @new
pub struct Crossing { pub step: StepNumber, pub columns: ColumnSet, pub keys: Vec<CrossingKey>, pub nodes: Option<NodeDemand> }  @new
pub struct CrossingKey { pub column: ColumnNumber, pub text: Box<[u8]> }  @new
```
Each field against the tip's `Partition`: `steps` is `file_steps` (one bool per step, as operators' `StepScope`, main's
word); `emit` is `local_emit`; `crossing` is one entry per element of `inputs`, with `columns` its `columns` entry (the
columns cross-file readers read: a `Vec<bool>` today, operators' `ColumnSet` here), `keys` its `filters` entry (the
literal equalities one of which a crossing row must satisfy) and `nodes` its `domains`/`reach` entry (the nodes of the
file a cross-file reader can need), from
`let mut domains = super::session::demand::domains(exec, run, &local, &input_set);  @2a207464c:crates/ryo/crates/executor/src/finish/partition.rs:488`.
`NodeDemand` is demand.rs's `Nodes` (`Column`, `Children`, `Below`, `Above`, `Union`) numbered by plan steps.
What the cut needs that the plan does not have today: each step's output kinds (the tip asks the executor,
`exec.schema(run, i)`). Under the draft it asks operators for the output shape of a call given its inputs' shapes, a
pure read with no execution; operators' listing exposes the shape only after `plan` (`PhysicalPlan::shape`), so a
shape-of-a-call read is owed by the operators sheet (decision 1). Which relations are project-wide comes from
`ryo_relations::scope_of`; which caller tables hold File cells, from the hosted relations handed to `lower`.
What stays with programs (not the plan): the admission's `closed`, `layout`, `file_cells`, `sliced_host` and
`file_inventory` flags, which read the linked languages (`project::layout` asks a language whether it can answer).

### 5d. The types
- `Plan` is `FinishPlan` renamed (decision 2): `steps: Vec<OperatorCall>` (operators' one list, replacing `FinishStep`),
  `rules`, `unplanned`. The marks the passes write (`ProbeMark`, `CountMark`, `ExistsMark`, the pushed filter) live
  in operators' argument types (`SelectArguments`, `AxisArguments`), because the kernel reads them; the plan sets them.
- `PlannedRule` gains `cut`. `UnplannedRule` keeps the typed `PlanLoweringError` in place of the sentence and of
  `ungiven`, both of which are made from it today:
  `PlanLoweringError::NotGiven(name) => Some(name.0.clone()),  @2a207464c:crates/ryo/crates/query_language/src/domain/plan_lowering/expansion/service.rs:100`.
- `PackPrograms`, `RuleContext`, `ContextTable`, `ContextObservation`, `ObservationProducer` are the tip's, with
  `ColumnName` the table's, `ObservationName` and `ParsedModules` ryoql's, `PlanConstant` operators', and
  `RuleContext.parameters` keyed by `RuleId`. `PackRule` gains `role`. `RuleProgram` stays ryoql's (its api.txt).
- `RuleId` moves from `values::runs` (that module imports `ryo_language`, outbound, so it cannot be in domain).
- `StepCount` moves from `values::descriptions`. `UnknownKind { word, languages }` is the executor's refusal fields
  (`Self::UnknownKind { word, languages } => write!(f, "kind `{word}` names no kind of {languages}"),  @2a207464c:crates/ryo/crates/executor/src/check_session/refusal.rs:42`);
  its `Display` keeps that text, which is also `unknown_message`'s.
- `PlanError` replaces the executor's string refusals for the lowering ("unplanned query {reason}", "missing query
  emit", `MissingParameter`) and keeper's `String`, with the same texts (review criterion 7: byte identity).
- `SharedSteps` is carried for the per-reader merge (perf row 20), typed: `append(OperatorCall, &[StepNumber]) ->
  StepNumber`.

### 5e. What stops being public
Listed by item in api.txt `# NO LONGER PUBLIC`: of the tip's 49, 16 carry over as types (9 of plan_lowering, 6 of
finish_plans, `Partition` as `Cut`); `lowered`/`mark_through`/`passed`/`PlanningStage` fold into `lower`; kind_check's
four functions go private (`names_kind` to operators, `located` to ryoql); `UnplannedReason` goes; `readers` goes
private; partition.rs's 8 functions go private; lower.rs's 10 move to ryoql (`PackTexts`, `parsed`), go private
(`SharedPlanData`, `intern`) or are replaced (`query_parts`, `query_batch`, `finish_plan`, `ReadPack`); `kinds::unknown`
becomes `unknown_kind`. The 158 other finish_plans items are operators'. Main's 27 are not restored as they are.

### 5f. Public interfaces side by side
| Side | Public items | Traits |
|---|---|---|
| main `f4ae3bd802` | 27 (+1 method): compile 5, values::plans 11, kind_check 10, `StepScope` | 1 (`CompileUseCase`); `CompileRulesUseCase` and `QueryLanguagePort` are programs' |
| tip `2a207464c` | 49 (+2): plan_lowering 15, kind_check 6, finish_plans 8, partition.rs 9, lower.rs 10, kinds.rs 1 | 0 |
| drafted | 26 (+8 methods, 20 variants) | 1 (`LowerUseCase`) |

## 6. Proof it brings
| Proof | Source | Fate |
|---|---|---|
| `axes`, `counts`, `semijoins` (1 row each), `joins` (4 rows) | tip QL `plan_lowering/*/service_test.rs` | **carried**: they call the private passes, which stay in this crate |
| `vocabulary_test.rs` (2 tests: the checks and the plan know the same operator names) | tip QL `plan_lowering/operators/vocabulary_test.rs` | **restated**: with one `OperatorName` list in operators, they become "every operator ryoql's checks know lowers, and back", run here |
| typecheck's 3 success rows (they compare `steps`) | tip QL `typecheck/use_case_test.rs` | **restated** against `judge` |
| main's kind check, 20 rows | main `execution_domain/src/domain/kind_check/service_test.rs` | **restated** against `unknown_kind` (the plan type and the matcher change; the words and verdicts carry) |
| main's `cutting` 6 rows (`a_relation_asked_for_one_column_is_cut_to_it` and siblings) and `planning` 2 rows | main QL `domain/{cutting, planning}` | **restated** against `Cut.crossing[].columns` and `Cut.steps`: the same properties on today's plan |
| main's compile seam (44 tests, 198 rows) and the other 12 passes (17 tests, 72 rows) | main QL | **not carried**: they assert main's optimizer, which is gone; the refusal rows among them are ryoql's to restate |
| the cut: per-step scope, crossing columns, crossing keys, node demand, the subtree-class case, a caller table read per file | none (partition.rs and demand.rs have 0 tests) | **MISSING**, and the largest gap: gains 7 and 1 rest on code no test holds. Planted bugs to catch: a step marked file-local that reads a cross-file input; a crossing column dropped that a reader reads; a key filter kept for an input one reader reads without it |
| `rule_plan` (subset reaches exactly the emit's steps, renumbered densely, equal steps shared) | none | **MISSING** |
| `SharedSteps` (equal steps merged across programs; inputs remapped) | none | **MISSING** |
| literal interning inside `lower` (one `Arc` per equal rows, condition, projection) | none | **MISSING** |
| `RuleRole::Observed` gives the same marks as today's retain | none | **MISSING**: one case per pass with a producer |

## 7. Perf gains it carries
From `contracts/perf-gains.tsv` (54 rows); the rows whose crate column names `domain/plan`. Figures are the TSV's.

| Row | Gain (measured) | What in the draft keeps it |
|---|---|---|
| 1 | lowering moved in; partition indexes closed readers once (start-up 341.3 -> 317.9 ms) | `lower` takes the whole pack in one call; the cut is computed once per rule inside `lower`, keeping ceb2047110's one pass over closed readers (`partition.rs:355`), never per source or per file |
| 7 | file-local rules leave the cross-file phase (dagster 26.127 -> 22.124 CPU-s, 1,245.4 -> 477.0 MiB) | `PlannedRule.cut`: `None` or `Cut.emit == PerFile` says a rule never enters the cross-file phase; `Crossing.columns` and `keys` say what leaves a file, decided at load. Release after the last reader is project's |
| 8 | pack lowered once per request (888 -> 586 M instructions) | `lower(&PackPrograms, ..)` once per request; `rule_plan` takes a subset of the one plan's steps, never a lowering of its own |
| 16 | scalar test pushed through an inner join into the scan (23.887 -> 23.267, flat) | `lower` always runs the `Axes` stage, whose `pushdown::mark` writes the pushed filter into the select's arguments (operators' `SelectArguments`) |
| 20 | one shared plan per reader (64.064 -> 52.952 CPU-s) | steps hash-consed in `lower` (`numbering::interned`) so equal steps of different rules are one `StepNumber`; `SharedSteps` kept public for the per-reader merge while binding |
| 27 | rules as programs anchored at a node (lost: 17 to 43 times worse) | nothing: plans stay whole-file; the draft has no per-node anchoring |
| 35 | step summaries decide pairs (django 11.60 -> 9.10) | step identity is the one `StepNumber` space of the pack plan, so operators' per-node `StepSummary` is shared by every rule using the step; the plan offers no second description of rules |
| 40 | Rust lowering replaces the Python chain (8.0 -> 0.093 CPU-s) | `lower` runs in process on parsed programs; no encoded or precompiled pack in any signature ("we aren't shipping rule packs") |
| 46 | crates/engine (lost) | its plan half is this crate (plan_lowering came from `engine`); its load/scan/project calls are programs', execute's and project's |

Not in the TSV but live and kept: literal rows, select predicates and projections interned across prepared programs
(ec4081a6bf, ba2550e397, f0b9db3b86, 71c6843dc0); no figure found. Kept by interning inside `lower`, so every rule's
steps share one `Arc` per equal literal. Room to make: RUF100's 64 ms re-lowering of one rule over 410 producers
(plan section 10.1/review-performance) needs a producer lowered once per pack: `RuleRole::Observed` in the one pack
lowering is that room; not measured.

## 8. Benchmarks that cover it
- No benchmark names a plan item. `git grep` of `plan_lowering|finish_plans|kind_check|ryo_executor::lower` over the
  benchmark crates, the lint images, Tastemaker's benchmarks, the SDK and the command line finds only the SDK's
  `PlanConstant` import (operators'). The draft cannot break a benchmark by name.
- The lint images (`vec`, `vec_ts`, `vec_rust`, `tm`) measure it end to end: lowering is 95 to 98 ms of Python's 296 ms
  start-up (`discovery-inputs/review-performance.md:11`), and the dagster/fastapi CPU and memory cells carry gain 7.
- `build/one-engine/scripts/price-root-split.py:47` (evidence repo) buckets `ryo_query_language|plan_lowering|...|
  Admission|ReaderPlan|::lower|...|planning::service` as "lower". After the regroup it must match `ryo_plan` (and
  `ryo_ryoql` separately) or the start-up split goes blind.
- `query_language/benches/performance.rs` registers no bench. A lowering bench over the Ruff pack (413 rules) would
  give `lower` its own number; not proposed here (the images are the measure).
- No Asymptote lock under `crates/ryo/.asymptote` names these paths.

## 9. Checks
IMPORTS: table, relations, operators, ryoql
CROSSED: per request
- Directions: all four are below `plan` in the agreed order. Today's wrong-way edges and their fix: ryoql -> plan
  (kind_check, keeper.rs) goes because both move here; plan -> executor (lower.rs builds `finish::Plan` through
  `from_typed`) goes because `logical` hands operators' `LogicalProgram` up instead; plan -> `ryo_language` through
  `values::runs::RuleId` goes because `RuleId` moves here; plan -> `ryo_matching` (`names_kind`) goes to operators'
  `CompareUseCase`, handed in.
- No layout word: the cut speaks of steps, columns and keys, never of storage; `Crossing.columns` is a set of column
  numbers.
- How often: `lower`, `logical` and `judge` once per request; `rule_plan` and `unknown_kind` once per rule of a request;
  `SharedSteps::append` once per step per reader while binding (per worker, today). Nothing is per file, row or cell.
- `dyn`: none; `LowerUseCaseImpl` is generic over the comparisons.
- Tastemaker: `fact-reads-through-lookup` must follow `plan_lowering` to its new path; `crate-plan` and `nu-hex-plan`
  entries are new; the `layer-order` sets name `idiomatic-sh-ryo-plan`.

## 10. Decisions for the owner              (each: the example from the code; why; what we get; my recommendation)
1. **Where the file/cross-file cut is decided.** Example: the tip computes it per program at admission over the
   executor's own numbering, evaluating each step's schema (`partition.rs:191`, `batch.rs:173, 203`); main's planner
   decided it and stored it in the plan (`plans/types.rs:38, 60`, `planning/service.rs:76, 184`). Why: gains 7 and 1
   (the largest memory gain on record) are the cut, it is a static fact of the plan, and today it has no test and
   lives in the 35,000-line proof of concept. What we get: the cut computed once when rules load, owned and tested
   with the plan, and `logical` able to give operators each node's `StepScope`. The cost: the plan must learn a call's
   output kinds without running it. **Recommend: the cut is the plan's (`PlannedRule.cut`), and operators exposes the
   output shape of a call from its inputs' shapes (a pure read) for it; the operators sheet to confirm.**
2. **Who owns the step type, and the plan's name.** Example: `FinishStep` (`finish_plans/types.rs:160`) is one of five
   operator lists, and plan_lowering's `operator_step` (`operators/service.rs:46`) is a sixth, a match over operator
   names building typed steps; operators' draft makes `OperatorCall` the one list and takes the argument types. Why:
   "one definition per operator". What we get: `Plan { steps: Vec<OperatorCall>, .. }`; the marks the passes write
   live in the argument types the kernels read. `FinishPlan` is the prototype's word ("finish") and becomes `Plan`
   (the executor's own `Plan`, `finish.rs:490`, goes with programs/execute). **Recommend: operators owns
   `OperatorCall` and its arguments; plan owns `Plan`, `PlannedRule`, `Cut`; reading a written call's arguments into
   an `OperatorCall` (today `operator_step` and `given/service.rs`) moves to operators beside each operator.**
3. **Validate's plan half, and who builds the report.** Example: `typecheck_kinds` lowers through the running plan
   (keeper.rs:30) and runs `unknown_word` (use case line 148), returning a `ValidateReport` whose relations and
   operators come from the program, not the plan. Why: ryoql's decision 2 sends both methods here; but relations read
   and operators used are ryoql's facts (`CheckedQuery.reads`, `.operators`), and the unknown word's place is ryoql's
   `locate`. What we get: plan answers only what the plan knows (`judge -> StepCount`, `UnknownKind`); inbound/validate
   composes ryoql's check, plan's judge and ryoql's locate. The kind vocabularies (`values::descriptions`) need a domain
   home: proposed `relations` (a column's vocabulary is already the catalog's, `vocabulary_of`). **Recommend: `judge`
   as drafted; `ValidateReport` to inbound/validate; `KindVocabularies`, `KindWord`, `LanguageName`, `RelationName`,
   `HostedRelation` to relations.**
4. **One pack plan downstream, or a plan per rule merged again per reader.** Example: today every rule becomes its own
   program (`query_parts` per rule, `query_batch`'s renumbered subset), and each worker merges the bound programs back
   into one plan per reader with `SharedSteps` (`shared.rs:52`). Why: the merge repeats per worker what `lower` already
   did once (equal steps are hash-consed at lowering); perf row 13 (parked) needs one frozen plan for every worker.
   What we get by keeping both calls now: the build is a move with today's callers. **Recommend: keep `rule_plan` and
   `SharedSteps` in this build; the programs sheet decides whether its frozen set holds the one pack plan, and if so
   both calls go private.**
5. **Which proof to restate.** Example: main's kind check (20 rows) and `cutting` (6 rows) test properties today's code
   still has and no tip test holds; main's compile seam (198 rows) tests an optimizer that is gone. Why: the cut and
   the kind check have 0 tests at the tip. What we get: the cut's property proved again before the build moves it.
   **Recommend: restate the 20 kind rows and the 6+2 cut rows against `unknown_kind` and `Cut`, write the missing cut
   contract test of section 6, and record the compile seam as not carried.**

## 11. Not checked
- Quotes: all `@rev:path:line` quotes in this sheet were re-checked by script (count in the final line of this
  section). Inline `path:line` references in prose (`tastemaker.toml` lines, `partition.rs:355`, `finish.rs:490`,
  `use_case.rs:148`, lower.rs and keeper.rs line ranges, `review-performance.md:11`, `price-root-split.py:47`) were read
  by hand once and not script-checked.
- The cut's move: whether every base relation's column kinds are known from `ryo_relations` without the linked
  languages (the tip's admission declares bases through `session::named::Catalog::from_readers`, i.e. from the
  languages); whether `with_file_slice` and the subtree-class case (`demand::local_class`) need anything only the
  executor has; how `ancestor_domains` and `reach` map onto `NodeDemand` (I read demand.rs's heads, not its bodies).
- That operators can expose a call's output shape before `plan`: operators' listing has `PhysicalPlan::shape` only.
- That `logical` belongs here rather than in programs: operators' listing says "the program the `plan` crate writes";
  PLAN 8.2 gives `from_typed` to programs. The programs sheet does not exist yet.
- `StepNumber`, `PlanConstant`, `ColumnSet`, `StepScope`, `LogicalProgram`, `CompareUseCase` are named from operators'
  listing as drafted on 28 Nov (still being written when read); `StepNumber` is not on it (it sits inside the argument
  types). `RelationName`, `HostedRelation`, `KindVocabularies`, `KindWord`, `LanguageName` are not on relations' listing.
- `judge` needs each declared parameter's kind to make placeholders (keeper.rs:33 calls ryoql's crate-private
  `parameter_kind_named`); whether `WrittenProgram` exposes the kind typed is not checked.
- Whether `RuleRole::Observed` gives exactly today's marks (the retain happens after `Expanded`; I argued, not tested).
- Main's `compile_rules`, `rule_compilation` and `QueryLanguagePort` were read only enough to give them to programs.
- Main's caller counts for the frontier/scope are file counts of a name pattern, not call graphs.
- No `tm check` on a skeleton of the draft; the nominal-type rules may object to `Option<Cut>`, `Box<[u8]>`,
  `Vec<Vec<Box<str>>>` in `ContextTable` (carried from the tip) and `BTreeMap<Box<str>, ..>` in `RuleContext`.
- Build 133 was not cited: plan_lowering existed there (in `engine`) and exists at the tip.
- Perf figures are the TSV's; nothing was measured.
- Quote check: 86 of 86 `@rev:path:line` quotes in this sheet match `git show` exactly (script, 28 Nov); the `@new`
  lines are drafts. api.txt carries tags, not quotes.
