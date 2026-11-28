# application/project
STATUS: drafted (28 Nov; not reviewed, not agreed)

Revisions: MAIN = f4ae3bd802 (release 0.2.0, live). TIP = 2a207464c (perf branch, build 139). B133 = f0219b3b3, cited
only for the designed `engine` contract that existed on neither side. All reads through `git -C
/workspaces/idiomatic-public-benchmark show|grep|ls-tree|log`. "X" below is `crates/ryo/crates/executor/src` at the tip;
"RT" is `crates/ryo/crates/execution_runtime/src/application/file_dag` on main. Every line naming a table type is
UNPROVEN until `domain/table/spike.md` records a pass.

## 1. Owns
The cross-file phase of a request: the facts that span files (what each file hands over, and the project's own
relations), held until their last reader, and the cross-file rules run over them once after the last file.

One thing, argued: holding the hand-over and running the rules over it are the two halves of one phase; neither has a
caller without the other (on main both sat in one trait pair, at build 133 in one call, at the tip in one driver
method, `Batch::finish_projects`). The project's relations are facts about all the files together (`RowScope::Project`),
read once per request; a file's visit borrows its slice of them through execute's `ProjectTablesPort`, which this crate
implements (execute's sheet, its decision 3). Two things the tip keeps beside it are NOT this concept and leave
(section 5f): reading the disk for the project's entries (`Environment`, `Authority::observe`: outbound/corpus) and the
packed column layout of carried rows (`population::Words`, `ColumnBlock`: outbound/store).

## 2. On main (f4ae3bd802)

**Where.** Two crates, behind one designed contract.
- `execution_runtime` (RT): `program/project_phase` (342 lines without tests), `program/observation_stage` (530),
  `project_pass` (1,313), `frontiers` (625), `project_source` (131).
- `core`: `application/cross_file` (1,515), `domain/crossing` (287), `domain/project` (140),
  `application/project_context` (165); `execution_domain::domain::{frontier_coverage, frontier_capture}` (144).
- 5,192 lines without tests in those directories (fixtures and `*_test*` excluded).

**The contract, verbatim.** The project phase is two traits, a per-request handle and the stage that runs batches of
occurrences:
```
pub trait ProjectRootExecution {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/project_phase/interface.rs:48
fn evaluate_rules<H, T, E>(  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/project_phase/interface.rs:53
pub trait ProgramProjectPhasePort: ProjectRootExecution {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/project_phase/interface.rs:77
fn finish(self) -> Result<(), OperatingError>;  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/project_phase/interface.rs:84
fn project_phase(&self) -> Result<ProjectPhase<'_>, OperatingError>;  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/request_execution/interface.rs:49
pub trait ProgramObservationStagePort {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/observation_stage/interface.rs:55
fn run_batches<H, T, E: From<OperatingError>>(  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/observation_stage/interface.rs:67
```
`evaluate_rules(phase: PhaseRules, symbols, before, partials, guard, complete)` takes four caller callbacks per
occurrence and answers `ProjectBatch<Result<T, PhaseStop<E>>, E>`. Its types:
```
pub struct OccurrenceNumber(pub usize);  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/project_phase/types.rs:17
pub struct PhaseRules<'a> {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/project_phase/types.rs:31
pub struct PhasePartials {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/project_phase/types.rs:63
pub enum PhaseStop<E> {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/project_phase/types.rs:71
pub struct ProjectPhase<'a> {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/project_phase/types.rs:105
pub struct ProjectOutputOutcome {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/rule_outcome/types.rs:43
pub enum ProjectObservation {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/rule_outcome/types.rs:60
pub enum ProjectBatch<T, E> {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/rule_outcome/types.rs:80
pub enum FrontiersError {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/frontiers/errors.rs:16
```
`execution` re-exports them as its contract:
```
pub use ryo_execution_runtime::application::file_dag::program::project_phase::interface::ProgramProjectPhasePort;  @f4ae3bd802:crates/ryo/crates/execution/src/inbound/use_cases/execution/interface.rs:35
pub use ryo_execution_runtime::application::file_dag::program::project_phase::interface::ProjectRootExecution;  @f4ae3bd802:crates/ryo/crates/execution/src/inbound/use_cases/execution/interface.rs:36
```
**Public items: 20 plus 5 trait methods**, counted as the items `execution/.../interface.rs:20-49` re-exports from
`program::{project_phase, observation_stage}` (14), `rule_outcome::{ProjectBatch, ProjectObservation,
ProjectOutputOutcome}` (3), `FrontiersError` (1), and core's `CrossFile` with `CrossFile::new` (2); the methods are
`evaluate_rules`, `finish`, `run_batches`, and `ProgramRequestExecutionPort::{project_phase, observations}`.

**How a file hands over, and how it is released.** A file's frontier is kept through the facts port (an outbound port
with a memory and a spill adapter) and read back after the files:
```
fn keep(&self, bucket: Bucket, words: WordsToKeep<'_>) -> Result<(), FactsError>;  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/outbound/facts/interface.rs:56
fn finished(self: Box<Self>) -> Result<Box<dyn FactsFinish>, FactsError>;  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/outbound/facts/interface.rs:70
fn kept(&self, bucket: Bucket) -> Result<super::types::KeptBucket, FactsError>;  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/outbound/facts/interface.rs:80
```
The phase decodes each rule's hand-over once, lends it to operators through `PlanInputs::frontier`, and drops it after
the rule's last occurrence:
```
pub(crate) struct HandedOver(pub(crate) Vec<FrontierRows>);  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/frontiers/types.rs:42
impl PlanInputs for PhaseInputs<'_> {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/project_pass/service/inputs/service.rs:31
fn frontier(&self, place: FrontierPlace) -> HeldRows<'_> {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/project_pass/service/inputs/service.rs:41
impl Resources for PhaseResources<'_> {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/project_pass/service/inputs/service.rs:67
pub(crate) fn held_frontier(handed: &HandedOver, place: FrontierPlace) -> HeldRows<'_> {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/frontiers/service.rs:239
pub(crate) fn uses_of(rules: impl IntoIterator<Item = RuleIndex>) -> RuleUses {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/project_pass/service.rs:61
```
So main already released a hand-over after its last reader, and a project relation read by many rules was held once.

**Who implements it.** `ProjectPhase` (`RT/program/project_phase/service.rs:25, 112`), built by
`RequestExecution::project_phase` (`RT/program/request_execution/service.rs:115`); a test double `Witness`
(`execution/tests/boundary_contract/submission/witness/fixtures.rs:114, 134`).

**Who calls it.** core's cross-file application, a per-rule memory budget guard included:
```
pub struct CrossFile {  @f4ae3bd802:crates/ryo/crates/core/src/application/cross_file/service.rs:47
pub fn new(corpus: Arc<dyn CorpusPort>, publication: Arc<dyn PublicationUseCase>) -> Self {  @f4ae3bd802:crates/ryo/crates/core/src/application/cross_file/service.rs:62
within(memory.resident_bytes()?, run.budget, || {  @f4ae3bd802:crates/ryo/crates/core/src/application/cross_file/ordinary/service.rs:34
Budget(#[from] crate::domain::budget::errors::BudgetError),  @f4ae3bd802:crates/ryo/crates/core/src/application/cross_file/errors.rs:5
cross_file: CrossFile,  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/lint/use_case.rs:113
ryo_core::application::cross_file::service::CrossFile::new(Arc::clone(  @f4ae3bd802:crates/ryo/crates/sdk/src/resources/linter/lint/composition.rs:161
```
Files naming the project-phase traits, by crate: core 6 (1 service, 1 fixture, 4 tests), execution 18 (1 interface,
17 tests and fixtures), execution_runtime 4. Composed at the edges: SDK `lint/composition.rs:161`, SDK
`query/run/composition.rs:89`, CLI `query/run/composition.rs:75`.

**Proof on main: 69 `#[test]`** in the concept's files (by `git grep -c '#\[test\]'`): the execution crate's boundary
suite `tests/boundary_contract/project_phase` 46 tests in 21 files (seam tests through `ProjectRootExecution`, e.g. the
pass test imports it, below), `refusal_order/frontiers` 2, `use_case_test/project_input` 1; execution_runtime
`project_pass` 2 (handovers 8 case rows, reads 3), `frontiers` 1 (3 rows), `project_source` 1; core `cross_file` 11 (16
case rows), `lint/use_case_test/project_budget` 3 (6 rows); execution_domain `frontier_coverage` 2. One acceptance
scenario names the cross-file worker (`execution/.../publication/acceptance.feature:12`). Eight `compile_fail`
doctests hold the seam: 4 on `ProjectRootExecution` (`interface.rs:14-47`), 1 on `ProjectPhase` (`types.rs:98`), 3 on
`ProgramObservationStagePort` (`observation_stage/interface.rs:17-45`).
```
use ryo_execution::inbound::use_cases::execution::interface::ProjectRootExecution;  @f4ae3bd802:crates/ryo/crates/execution/tests/boundary_contract/project_phase/pass/test.rs:12
name: CaseName("a_project_relation_handed_to_many_rules_is_held_once"),  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/project_pass/service_test/handovers/test.rs:41
name: CaseName("a_rule_handover_goes_after_its_last_occurrence"),  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/project_pass/service_test/handovers/test.rs:55
name: CaseName("an_occurrence_past_its_rules_count_reads_a_released_handover"),  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/project_pass/service_test/handovers/test.rs:95
name: CaseName("ordinary_budget_after_actual_first_cohort"),  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/lint/use_case_test/project_budget/test.rs:15
```
**Tastemaker on main:** `crate-core` (`crates/ryo/tastemaker.toml:128`) and `nu-hex-core` (`:188`) cover the core half;
`execution_runtime` has no crate entry, only its allowlists (`:18`, `:19`), `derived-view-acquisition` (`:65`),
`runtime-authority` (`:68`), `operator-meaning-once` (`:72`).

**The name already taken.** The language contract's own `ProjectUseCase` (a language reading its manifests), the same
on both sides and at build 133:
```
pub trait ProjectUseCase: Send + Sync {  @f4ae3bd802:crates/ryo/crates/language/src/inbound/use_cases/project/interface.rs:27
pub trait ProjectUseCase: Send + Sync {  @2a207464c:crates/ryo/crates/language/src/inbound/use_cases/project/interface.rs:27
```

## 3. On the perf branch (2a207464c)

**Where.** All inside the proof-of-concept `executor`, no contract. My ranges, without tests: `X/check_session/frontier.rs`
555, `X/check_session/project.rs` 648, `X/finish/population.rs` 673, `X/finish/session.rs` 626,
`X/finish/session/{contribution 414, demand 598, reuse 69, certificate 46}`, `X/check_session/replay/{closed_workers 331,
readers 209}`, `Batch::finish_projects` (`replay/batch.rs:664-886`, 223) and `hosted::execute_ids` (50): **4,442 lines**.
The plan's cut (`X/finish/partition.rs`, 525) is domain/plan's and not counted.

**The interface actually called.** One driver method runs the whole phase:
```
self.finish_projects(languages, reader_states, contributions, world, answers, populations, project_unread, store.as_deref_mut());  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/batch.rs:552
pub fn finish_projects<L: session::source::Source>(  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/batch.rs:664
```
It reads the project population (`project::collect`), groups the closed requests by rule, joins each request's carried
inputs, evaluates, places the rows, offers the answer to the kept store, then releases:
```
pub fn collect<L: super::FileSource>(  @2a207464c:crates/ryo/crates/executor/src/check_session/project.rs:403
|readable, steps| readers.inputs(i, || world.joined(readable, steps)));  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/batch.rs:808
pub fn execute_ids(  @2a207464c:crates/ryo/crates/executor/src/check_session/hosted.rs:136
pub fn evaluate_covered(  @2a207464c:crates/ryo/crates/executor/src/finish/session.rs:33
s.closed(i,&answer,pop,project.map(|id| (id, &self.projects[id].authority)));  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/batch.rs:860
if let Some(finished)=readers.release(i,pairs,*project,|slot| {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/batch.rs:864
```
With more than one worker and no store, the same work goes through `closed_workers::run`, which spreads jobs with rayon
(`jobs.rs` imports `rayon::prelude`), not through the scheduler port:
```
pub fn run(  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/closed_workers.rs:202
let (parts, cpu) = session::jobs::run(0..count.min(programs.len()), run_shard);  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/closed_workers.rs:312
pub fn run<I, T, F>(jobs: I, work: F) -> (Vec<T>, f64)  @2a207464c:crates/ryo/crates/executor/src/check_session/jobs.rs:5
pub trait SchedulerPort: Send + Sync + std::fmt::Debug {  @2a207464c:crates/ryo/crates/core/src/outbound/scheduler/interface.rs:8
```
**The hand-over.** Per file, the file's visit carries its cut inputs into the worker's `World`; a file that hands nothing
gets a certified empty frontier; workers move their `World` to the coordinator as a `Transfer`:
```
pub fn contribute(  @2a207464c:crates/ryo/crates/executor/src/check_session/frontier.rs:61
let value = shared.as_mut().expect("project output was bound").contribute(b, &mut exec, current, &mut run, id.0, &mut facts_view, cut,  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:1563
world.insert_empty(slot, &source.path, b, &mut run, cut);  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:1166
pub struct World {  @2a207464c:crates/ryo/crates/executor/src/check_session/frontier.rs:204
pub fn carry(  @2a207464c:crates/ryo/crates/executor/src/check_session/frontier.rs:304
pub fn insert_empty(  @2a207464c:crates/ryo/crates/executor/src/check_session/frontier.rs:290
pub struct Transfer {  @2a207464c:crates/ryo/crates/executor/src/check_session/frontier.rs:437
pub fn transfer(mut self, prefix: usize) -> Transfer {  @2a207464c:crates/ryo/crates/executor/src/check_session/frontier.rs:449
pub fn receive(&mut self, parts: Vec<Transfer>, sources: &[super::Source]) {  @2a207464c:crates/ryo/crates/executor/src/check_session/frontier.rs:498
pub fn receive_blocks(&mut self, part: Transfer, files: &[u32], symbols: &[u32]) {  @2a207464c:crates/ryo/crates/executor/src/check_session/frontier.rs:531
(a, links, slots, owned, part.transfer(prefix), time, local.bytes, wall.elapsed().as_secs_f64())  @2a207464c:crates/ryo/crates/executor/src/check_session/worker.rs:190
world.receive(transfers, sources);  @2a207464c:crates/ryo/crates/executor/src/check_session/worker.rs:236
pub fn joined(&self, slots: &[usize], cut: &[usize]) -> Result<finish::population::Inputs> {  @2a207464c:crates/ryo/crates/executor/src/check_session/frontier.rs:407
pub fn release(&mut self, slot: usize) {  @2a207464c:crates/ryo/crates/executor/src/check_session/frontier.rs:429
pub fn restore_frontier(&mut self, slot: usize, parts: Vec<Part>) {  @2a207464c:crates/ryo/crates/executor/src/check_session/frontier.rs:270
world.restore_frontier(slot, parts);  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/stored.rs:1160
```
The joined input of a cut input is a list of per-file pieces, read by key without making a whole table, and made whole
once only when an operator needs it:
```
pub struct Population {  @2a207464c:crates/ryo/crates/executor/src/finish/population.rs:374
pub fn matching_rows(&self, col: usize, key: u64, hits: &mut Vec<u32>) {  @2a207464c:crates/ryo/crates/executor/src/finish/population.rs:449
pub fn full(&self, names: &Names, tys: &Tys) -> Rc<Table> {  @2a207464c:crates/ryo/crates/executor/src/finish/population.rs:415
pub enum Words {  @2a207464c:crates/ryo/crates/executor/src/finish/population.rs:40
pub struct ColumnBlock<C> {  @2a207464c:crates/ryo/crates/executor/src/finish/population.rs:158
pub struct SharedBlock<'a> {  @2a207464c:crates/ryo/crates/executor/src/finish/population.rs:172
pub struct CarriedBlock {  @2a207464c:crates/ryo/crates/executor/src/finish/population.rs:572
```
Last-reader release and repeat answers live in `readers`:
```
pub struct Closed {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/readers.rs:25
pub fn release(  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/readers.rs:159
pub struct Evaluated {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/readers.rs:249
pub struct Reuse {  @2a207464c:crates/ryo/crates/executor/src/finish/session/reuse.rs:13
```
The project's relations and the disk:
```
pub struct Population {  @2a207464c:crates/ryo/crates/executor/src/check_session/project.rs:342
pub struct Layout {  @2a207464c:crates/ryo/crates/executor/src/check_session/project.rs:21
pub fn read_file<L: super::FileSource>(  @2a207464c:crates/ryo/crates/executor/src/check_session/project.rs:196
pub struct Contribution {  @2a207464c:crates/ryo/crates/executor/src/check_session/project.rs:277
pub enum Authority {  @2a207464c:crates/ryo/crates/executor/src/check_session/project.rs:284
pub fn observe(&self, files: &[l::FilePath], path: l::FilePath) -> EntryObservation {  @2a207464c:crates/ryo/crates/executor/src/check_session/project.rs:313
pub struct Environment {  @2a207464c:crates/ryo/crates/executor/src/check_session/project.rs:583
pub fn physical(&self, path: &Path) -> l::ObservedEntryState {  @2a207464c:crates/ryo/crates/executor/src/check_session/project.rs:615
```
`Authority::observe` calls `std::fs::metadata` (`project.rs:327`): disk I/O in what the tree calls an application crate.

**Public items: 183**, every one reachable from outside the crate (`lib.rs:9` `pub mod check_session`, `:10` `pub mod
finish`), counted by `^\s*pub (const|static|fn|struct|enum|type|trait) ` before any `#[cfg(test)]`: frontier.rs 26,
project.rs 22, population.rs 56, session.rs 14, contribution.rs 12, demand.rs 14, reuse.rs 13, certificate.rs 2,
closed_workers.rs 10, readers.rs 12, plus `finish_projects` and `execute_ids`. Two are re-exported at the top:
```
pub use frontier::World;  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:34
pub use frontier::SharedFrontier;  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:35
```
**Callers.** Inside the executor only (`check_session.rs`, `worker.rs`, `replay/{batch, stored, closed_workers}.rs`),
plus the SDK, which builds the project's disk memo and names its authority:
```
use ryo_executor::check_session::project::Authority;  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session.rs:22
environments: HashMap<Arc<str>, Arc<ryo_executor::check_session::project::Environment>>,  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:107
let mut environment = ryo_executor::check_session::project::Environment::new(  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:432
pub fn takes_environment<L: Source>(languages: &[L], reader: Option<&str>, path: &str) -> bool {  @2a207464c:crates/ryo/crates/executor/src/check_session/source.rs:66
```
The language contract's project view exists at the tip but nothing calls it; the project is read through the
executor's own 19-method `FileSource` and core's `LanguagePort::project`:
```
pub struct ProjectFacts<'p> {  @2a207464c:crates/ryo/crates/language/src/domain/facts/types.rs:69
fn project(  @2a207464c:crates/ryo/crates/core/src/outbound/language/interface.rs:59
```
**Proof at the tip: 4 `#[test]`** that are about this concept; no acceptance scenario, no `compile_fail`:
```
fn a_relation_read_in_the_visit_gives_each_file_the_rows_the_project_phase_gives_it() {  @2a207464c:crates/ryo/crates/executor/src/check_session/project_test.rs:127
fn a_language_that_does_not_answer_one_file_alone_keeps_its_relation_on_the_closed_path() {  @2a207464c:crates/ryo/crates/executor/src/check_session/project_test.rs:158
fn a_declaration_is_told_by_its_content_not_by_the_room_it_was_in() {  @2a207464c:crates/ryo/crates/executor/src/finish/population.rs:681
fn a_shared_frontier_lives_until_its_last_closed_reader() {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/readers.rs:215
```
(`session.rs:662, 668` test `keep_finding` and `batch.rs:934` tests report slots: application/execute's.) Beyond them,
the whole-run images and the Ruff, Oxlint, Clippy and Tastemaker parity suites cover the phase end to end (section 8).

**Tastemaker at the tip:** `crate-executor` (`crates/ryo/tastemaker.toml:105`), its allowlists (`:31`, `:32`);
`crate-core` (`:118`), `nu-hex-core` (`:178`). No entry names the cross-file phase.

**Build 133 (on neither side): the designed one-call contract.**
```
pub trait ProjectUseCase: Send + Sync {  @f0219b3b3:crates/ryo/crates/engine/src/inbound/use_cases/project/interface.rs:23
fn project(  @f0219b3b3:crates/ryo/crates/engine/src/inbound/use_cases/project/interface.rs:29
pub enum ProjectPassError {  @f0219b3b3:crates/ryo/crates/engine/src/inbound/use_cases/project/errors.rs:21
pub trait FrontierSource {  @f0219b3b3:crates/ryo/crates/engine/src/domain/findings/types.rs:121
pub trait FoundRows {  @f0219b3b3:crates/ryo/crates/engine/src/domain/findings/types.rs:30
pub struct ScanCounters {  @f0219b3b3:crates/ryo/crates/engine/src/domain/findings/types.rs:172
pub struct ProjectFacts<'p> {  @f0219b3b3:crates/ryo/crates/language/src/domain/facts/types.rs:69
```
`project(programs, frontier: &dyn FrontierSource, project: ProjectFacts, scratch, found: &mut dyn FoundRows)`:
`FrontierSource::rows` calls back once per row, `FoundRows::found` once per found row. Proof: 1 test, 2 scenarios:
```
fn succeeds() {  @f0219b3b3:crates/ryo/crates/engine/src/inbound/use_cases/project/use_case_test.rs:53
Scenario: an empty rule set's project pass finds nothing  @f0219b3b3:crates/ryo/crates/engine/src/inbound/use_cases/project/acceptance.feature:6
Scenario: a frontier row emitted by a file reaches the project pass  @f0219b3b3:crates/ryo/crates/engine/src/inbound/use_cases/project/acceptance.feature:12
```

## 4. What changed between them

**Removed** (all 26 Nov, `git log --diff-filter=D f4a32acb56..2a207464c`):
- core's `application/cross_file` (the caller, with its budget guard): `ff4533b99a` "core: the old engine arms are
  deleted".
- execution_runtime's project phase, observation stage, `project_pass`, `frontiers`, the facts port and **the 46-test
  boundary suite**: `861c7de10e` "the old executor goes" (546 files, 55,184 lines deleted).
- build 133's `engine` `ProjectUseCase` (added `3097109506`, 20 Nov): `fc94354b0c`.

**Added** at the tip: `World`, `Transfer`, `Population`, `readers::Closed`, `closed_workers`, in 28 commits on
`frontier.rs` (`git log --follow`), first `c581e59e48` (24 Nov, "Share projected file populations across closed
session readers"), last `786a8a6ca2` (26 Nov). The measured ones: `312e6c6d46` and `17f80fc862` (24 Nov; perf row 23:
peak 9,487.7 -> 1,874.2 MiB), `bdc48ef28f`, `1088432864`, `a7916c7e8e` (24-25 Nov, carry only what closed readers
need; release closed captures at completion; perf row 7: dagster 1,245 -> 477 MiB), `4d46c2f80e` (26 Nov; perf row 4:
the project environment only for files whose language uses the project).

**Changed in shape.** Main: three traits, four callbacks per occurrence, a frontier kept as packed words in a port with
a spill adapter. Tip: no contract; one 223-line method with 8 parameters besides `self`; carried rows are packed column pieces held in
`World` and joined by key. Both release after the last reader (main `uses_of`, tip `readers::Closed::release`).

**Lost that should not have been.**
- The seam itself and its proof: 69 tests on main against 4 at the tip; the 46 boundary tests and 8 `compile_fail`
  doctests went with the contract, not because they failed.
- The per-rule memory budget guard (`within(memory.resident_bytes()?, run.budget, ..)`, 6 case rows): nothing at the
  tip bounds the phase (`git grep -i budget` over the tip's frontier, replay, population and session files: no hit).
- The spill adapter for kept frontier words (main's facts port): at the tip carried rows live in memory only.
- Cross-file jobs through the scheduler: the tip spreads them with rayon directly (`jobs.rs`), beside `SchedulerPort`.

## 5. The drafted public interface
The listing is `application/project/api.txt` (15 public items, 1 variant, 1 impl of execute's port). This part explains
every line that is changed or new. All of it is UNPROVEN while it names `ryo_table::TablePort`. It names the types the
execute and programs sheets drafted on 28 Nov (`Carried`, `FileAnswers`, `RuleAnswer`, `ProjectTablesPort`,
`Programs`) and defines none of theirs.

### 5a. The claim it starts from, checked
Appendix A §3.10 drafts `ProjectUseCase { receive(state, file, carried), project(programs, state, project, out) }`.
Checked against the code, five corrections:
1. **The name clashes** (the language contract's `ProjectUseCase`, quoted above). Drafted as `CrossFileUseCase`
   (decision 1); main's own word for this concept was `cross_file` (`CrossFile`, `CrossFileError`).
2. **The hand-over crosses workers.** Each worker fills its own `World` and the coordinator joins them in path order
   (`World::receive`, "Worker completion order has no effect", `frontier.rs:495-497`). Appendix A's single `state`
   cannot say that. Drafted: one `Handover` per worker, all of them given to `run` as `Handovers`.
3. **Symbols cross too.** `World::carry` translates the file's symbol cells into the worker's own dictionary
   (`frontier.rs:315-320`), and `receive_blocks(part, files, symbols)` renumbers them on the coordinator. Drafted:
   `receive` takes the dictionary the file's cells are numbered in (execute's `ScanScratch::symbols()`), and
   `Handover::new` layers over `Programs::symbols()`, the run's frozen dictionary.
4. **The phase is run on workers.** The tip runs cross-file jobs in parallel (`closed_workers::run`). Drafted: the
   scheduler is bound at construction and `run` is lent one store per worker (`&mut [S]`).
5. **The project's relations are needed before the first file**, not only after the last: the tip builds
   `project::Layout` at registration (`batch.rs:295`) and every file's visit reads its slice (`check_session.rs:1392-1428`).
   Drafted: `project_population`, once per request, whose value implements execute's `ProjectTablesPort` and is read
   again by `run`.

### 5b. Each line against what exists
- `CrossFileUseCase::project_population(&Programs, ProjectInputs) -> ProjectPopulation`: the tip's `project::layout(readers,
  authority, inventory, environment, relations)`. The relations come from `Programs` (what loaded rules read with
  `RowScope::Project`), the language from the constructor, the inventory and the disk observation from the request
  (`ProjectInputs`). Crossed once per request.
- `ProjectPopulation` and its `impl ProjectTablesPort<S>`: the tip's `Layout { relations, parts, words, visit }` made
  opaque; `file_tables(file, notes, relations, store, out)` is `Layout::read_file` then `Layout::slice`, once per file,
  writing into the worker's store. It also holds what `run` needs to read the project population after the files
  (the tip's `project::collect`).
- `ProjectInputs { inventory, observe }`: the tip request's `inventory` and `repository: Option<Authority>`, with the
  SDK-built `Environment`, which implements the language's `FileProject` (`project.rs:629`). Only the trait object
  reaches this crate; the disk read stays outside (decision 4). Execute's `ScanFile.observe` is the same object.
- `CrossFileUseCase::receive(&mut Handover, FileId, Carried, &impl SymbolsUseCase)`: one call per file; replaces
  `World::carry` (one per carried table), `World::insert_empty` (`CarriedInput.rows: None`), the `world.unavailable`
  writes (`CarriedInput.partial`) and `contributions.set` (`Carried.notes`). The file's own visit decided what to carry
  (application/execute, from `Programs::cut`), so `receive` only holds and translates. `FileId` is a parameter because
  execute's `Carried` does not hold it.
- `CrossFileUseCase::run(CrossFileRun, Handovers, &mut [S], &mut CrossFileAnswers)`: build 133's `project(..)` with
  three changes: the hand-over comes in whole, not as a `dyn FrontierSource` pulled row by row; answers leave as one
  table per rule, not one `found` call per row (perf row 18); one store per worker is lent. Its body is the tip's
  `finish_projects`: read the project population once (only what loaded cross-file rules read), join each rule's inputs
  in file order, run each cross-file program through operators' `RunUseCase::run` on the scheduler's workers, release
  each carried table after its last reader, answer an equal rule over equal inputs once.
- `CrossFileRun { programs, tables, hosted, budget }`: what the tip passes `finish_projects` and holds in
  `self.bindings[binding]` (the request's host tables, typed as execute's `ScanFile.hosted: &[S::Table]`); `budget` is
  main's per-rule guard restored, may this rule start (decision 5). A struct, because five positional arguments is a
  long list (review criterion 12). `budget` is a `dyn` because it is crossed once per rule, not on a hot path.
- `Handover`, `Handover::new(&Programs)`: the tip's `World` (7 public fields) made opaque; `new` replaces
  `World::default()` + `declare_words`, over the frozen dictionary instead of a copy of every word (the copy
  `SharedWords::install` made is what perf row 13 found wrong).
- `Handovers<S>(pub Vec<Handover<S>>)`: the tip's `Vec<Transfer>`. A newtype because interface modules carry no bare
  `Vec` (scoper §2).
- `CrossFileUseCaseImpl::new(L, W)`: main's `CrossFile::new(corpus, publication)`; ports bound once, as generics. `L` is
  the language contract (project relations), `W` the scheduler.
- `CrossFileAnswer { rule, answer: ryo_execute::RuleAnswer<S>, skipped, partial_files, unknown }`: the answer is
  execute's type, so a rule's answer has one definition whether it ran per file or across files; `skipped` and
  `partial_files` are the files the tip passes as `skipped: Vec<String>` and `incomplete: BTreeMap<String,
  Vec<Option<u32>>>`; `unknown` is the tip's `unknowns: Vec<(String, String)>`. Main's `ProjectObservation { Complete,
  Partial }` maps to `RuleAnswer::Rows` with and without `partial_files`.
- `CrossFileAnswers { rules, words, unread }`: `words` is the coordinator's dictionary frozen, so the answers decode
  without the run (the symbols sheet's decision 5); `unread` is the tip's `project_unread`, the project inputs the tools
  refused.
- `CrossFileError::WorkersFailed`: the one phase-wide failure (the scheduler can refuse; rayon cannot). Every per-rule
  refusal is a `RuleAnswer::Refused(RuleRefusal)`: build 133's `ProjectPassError::Refused { rule, relation, cause }` is
  execute's `RuleRefusal::Relation { relation, cause }`; main's `PhaseStop::Operating` is `RuleRefusal::Operator`; two
  cases have no home in execute's draft yet (R1).

### 5c. What this draft needs from other sheets (requests, not definitions)
- **R1 application/execute.** `Carried<S> { inputs, notes }` and `CarriedInput<S> { rule, input, rows, partial }` as
  drafted serve `receive`; `ProjectTablesPort<S>` as drafted is implemented here. Asked: two cross-file refusals in
  `RuleRefusal`: an input a file withheld (main `PhaseStop::Frontiers(FrontiersError)`, the tip's
  `closed_workers::Inputs::Withheld`) and over budget (main `CrossFileError::Budget`, decision 5). `RuleRefusal::Relation`
  carries `PartialCause` where build 133 had `AbsenceCause`: to be settled there.
- **R2 domain/programs.** `Programs::cut(rule)` (plan's cut) tells which rules cross and what each cut input carries;
  asked beside it: the project relations each cross-file rule reads, and a producer identity equal across rules for an
  equal carried step, so equal hand-overs are held once (today found at run time by `contribution::Registry::identify`;
  main held a project relation read by many rules once). Programs names `ryo_plan::RuleCut`, the plan sheet `Cut`: one
  of the two is renamed.
- **R3 domain/table and outbound/store: two capabilities the table sheet does not carry.** (a) A carried table outlives
  its file and is read on another worker: the tip copies it into a `CarriedBlock` and back (`population.rs:618, 629`)
  or shares it read-only as an `Arc<[Words]>` (`SharedBlock`); main kept it through `FactsWorker::keep` / `finished` /
  `FactsFinish::kept` with a spill adapter. The plan says "one per worker; a table is released on the worker that made
  it" (Appendix A §3.12) and execute's `close` hands a worker's store back for exactly that reason, which carried tables
  break (decision 2). (b) A reader over many tables as one, without a copy, keyed through `TableRead::matching`: the
  tip's `Population { pieces }` with `matching_rows` and `full()` made once on demand. `Population::matching_rows` is a
  live equivalent of the table sheet's `matching`, which that sheet records as having none.
- **R4 domain/symbols.** `SymbolLayer::intern` and `SymbolsUseCase::text`, both drafted there, are all `receive` and
  `run` need to translate symbol cells. Join order is file order, so numbering never depends on worker timing.
- **R5 outbound/language.** Its draft keeps `LanguagePort::project(.., &mut dyn RowWriter)` (moved from core,
  `core/.../language/interface.rs:59` today) and the language's own `ProjectUseCase` [main]. Asked: a project read that
  fills through the table contract (a row at a time through `dyn RowWriter` is the layout and per-row crossing the rules
  forbid), given the files' notes and manifests; `uses_project` stays declared once at link (perf row 4).
- **R6 outbound/corpus.** `Environment` and `Authority` (decision 4), offered as the language's `FileProject`.
- **R7 domain/operators.** A cross-file run has no open file: `roots` answers no candidates (as main's
  `PhaseInputs::roots` did) and `PlanInputs::Tree` needs an empty tree; the operators draft makes `tree()` mandatory.

### 5d. Inside the crate (not public, listed so two builders build the same thing)
Nu-hex: `src/inbound/use_cases/cross_file/{interface, use_case, types, errors}.rs`; `src/domain/{handover, joined,
last_reader, population, repeats}/`; `src/outbound/` empty (the ports are handed in). The implementations of operators'
`PlanInputs` and `Resources` for a cross-file run are private (main's `PhaseInputs`, `PhaseResources`). No
`pub(crate) trait` is drafted.

### 5e. What it would take to remove this crate's design
None of the calls exists in the code today under these names; the shape exists twice (main's phase pair, build 133's
one call) and was deleted, not disproved. Nothing in the tip's speed depends on the absence of a seam here: the
phase's gains (section 7) are what is held and when it is released, not how it is called.

### 5f. Public items, counted, and what stops being public
| Side | Public items | Becomes |
|---|---|---|
| main | 20 + 5 trait methods (section 2) | all replaced: `CrossFileUseCase`, `Handover`, `CrossFileAnswer`, `CrossFileError` |
| tip | 183 (section 3) | 0 public; private here, or moved to execute, programs, plan, store, corpus, symbols (api.txt `# NO LONGER PUBLIC`, line by line) |
| drafted | 15 + 1 variant + 1 impl | |

## 6. Proof it brings
| Proof | Source | Fate |
|---|---|---|
| 46 boundary tests through `ProjectRootExecution` | main `execution/tests/boundary_contract/project_phase` (21 files) | **restated** against `CrossFileUseCase::run`. Of them, `cutting` (3), `rewrites` (7) and `lineage` (1) test the plan's cut and rewrites: they go to domain/plan's sheet; the other 35 (applicability, axis, consumers, declared_empty, grouping, members, merged, named_selection, negative/deferral, pass, population_reads, restoration (7), row_by_row, settle, skipped_frontier, text_identity, union) stay here |
| handovers 8 rows, reads 3 rows | main `RT/project_pass/service_test/{handovers, reads}` | **restated**: "held once", "goes after its last occurrence", "past its count reads a released handover" are the release contract of perf rows 7 and 23 |
| frontiers 3 rows | main `RT/frontiers/service_test.rs:34, 42, 50` | **restated** as withheld-input refusals (R1) |
| core `cross_file` 11 tests, 16 rows | main `core/src/application/cross_file` | **restated**; `rendering` (2 rows) goes with rendering to application/execute |
| project budget 3 tests, 6 rows | main `core/.../lint/use_case_test/project_budget` | **restated** against the budget `Fn` (decision 5) |
| `refusal_order/frontiers` 2, `project_input` 1, `frontier_coverage` 2 | main | **restated** |
| 8 `compile_fail` doctests | main `project_phase/interface.rs:14-47`, `types.rs:98`, `observation_stage/interface.rs:17-45` | **restated**: `Handover`'s fields cannot be reached; `CrossFileUseCaseImpl` cannot be built without its ports |
| `a_shared_frontier_lives_until_its_last_closed_reader` | tip `readers.rs:215` | **carried**, restated through `run` |
| `a_declaration_is_told_by_its_content_not_by_the_room_it_was_in` | tip `population.rs:681` | to outbound/store (carried schemas are layout) |
| the two `project_test.rs` tests | tip `project_test.rs:127, 158` | **carried here**: they compare a file's slice (`ProjectTablesPort::file_tables`) with what `run` reads of the same project; both sides are this crate's |
| 1 test, 2 scenarios | B133 `engine/.../project/{use_case_test.rs:53, acceptance.feature:6, 12}` | **carried** as this crate's acceptance feature |
| a contract test every implementation runs | none | **MISSING**: `fixtures/contract.rs` with the planted bugs: release before the last reader; a table kept after it; answers depending on worker order; a withheld input answered as empty; a project relation read twice; a file's slice differing from the population `run` reads |
| an allocation test of the phase | none | **MISSING**: peak must not grow with files times cross-file rules (the 9,488 -> 1,874 MiB gain has no gate but the images) |

## 7. Perf gains it carries
Rows of `perf-gains.tsv` whose crate column names project, and what in the draft keeps each:

| Row | Gain (measured) | What in the draft keeps it |
|---|---|---|
| 4 | project environment only for files whose language uses the project (without it: +25% CPU, +70 MiB on a workspace with no Python file) | the decision is the language's, at link (`uses_project`, `walk/interface.rs:80`), and the environment moves to outbound/corpus (decision 4); this crate never builds an environment, and `run` reads project relations once per request only for relations a loaded cross-file rule reads (`Programs`, R2), never per file |
| 7 | file-local rules leave the cross-file phase; closed inputs released after the last reader (dagster 26.13 -> 22.12 CPU-s, 1,245.4 -> 477.0 MiB) | `receive` takes only what `Carried` holds, and execute carries only the cut inputs of rules `Programs` marks cross-file (R1, R2); `run` keeps a last-reader count per carried table and releases it inside the run, with no caller call (the tip's `World::release` becomes private) |
| 23 | shared project populations; closed capture by subject (peak 9,487.7 -> 1,874.2 MiB; project 29.3 -> 3.6) | the joined input is read by key through `TableRead::matching` and made whole only on demand (R3b); equal producers share one held table across rules (R2's producer identity; main's "held once") |

Other rows that touch this crate's files: 18 (answers as packed tables per rule, no per-row callback: each
`CrossFileAnswer` holds one execute `RuleAnswer::Rows` table per rule, not build 133's `FoundRows::found`); 1 (the cut computed once when rules load: read from
`Programs`, never recomputed here); 2 and 53 (frontier columns never cross the cache port: no call in this interface
writes or restores carried rows, decision 3); 13, parked (frozen and shared, not copied per worker: `Handover::new`
layers over `Programs::symbols()`, no `SharedWords::install`). Made room for: the small kept store (row 53) by decision 3.

## 8. Benchmarks that cover it
| Benchmark | What it reaches for today | After the draft |
|---|---|---|
| the four whole-run images (`executor/benches/lint_image`: `vec`, `vec_ts`, `vec_rust`; `tm`) | the SDK only | unchanged: the measure of "not lost" for rows 4, 7, 23 (peak and CPU) |
| the phase clock in the SDK's measurement and the Tastemaker check's report | `c.project + c.project_workers_cpu` (`entry.rs:363`), printed as `project=` in `CHECK_SESSION` (`session/measure.rs:61`), read by the benchmark crate (`profile_evidence.rs:253`) and by the Tastemaker check (below) | the edge times `CrossFileUseCase::run`; the column keeps its name and meaning |
| Tastemaker check `measure.py` | the "project" column | unchanged |
| Tastemaker `benchmarks/project/live/capture.rs` | `replay::batch_mode*`, the executor's driver | calls the SDK, as the images do |
| Tastemaker `benchmarks/project/store_matrix.py` | a "closed-contribution" row of the kept store | changes meaning with decision 3: no carried rows are kept |
| Ruff, Oxlint, Clippy and Tastemaker parity suites | the SDK | unchanged; the 7 of 413 Ruff programs that are cross-file (`rows-trace.md`, "Only 7 of 413") are the parity proof here |
```
replay::batch_mode_with_workers(&readers(linked), manifest, output, None, workers, completed);  @2a207464c:crates/tastemaker/benchmarks/project/live/capture.rs:23
"project": "Project phase",  @2a207464c:crates/tastemaker/benchmarks/check/measure.py:52
.find_map(|line| line.strip_prefix("CHECK_SESSION live "))  @2a207464c:crates/ryo/crates/benchmark/src/application/profile_evidence.rs:253
[c.prepare, c.read_parse, c.screen, c.evaluate, c.project + c.project_workers_cpu, c.assemble + c.delivery_workers_cpu]  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:363
```
No divan bench measures the phase on either side.

## 9. Checks
IMPORTS: table, symbols, relations, operators, programs, execute, language, scheduler
CROSSED: per file

- Directions: domain (`table`, `symbols`, `relations`, `operators`, `programs`), outbound (`language`, `scheduler`),
  application `execute`, which comes before `project`. Not `store` (generic over `TablePort`), not `cache` (nothing
  crosses it), not `corpus` (the disk observation comes in as the language's `FileProject`), not `plan` (the cut is
  read through `Programs::cut`). `scheduler` has no sheet yet, so `check-graph.py` reports it until it does (run 28
  Nov: that failure only for this sheet).
- Layout words: none. `Handover` is opaque; rows cross as `S::Table`; `Words`, `Block`, `Population`, `Vec<u64>`,
  `Rc<Table>` are private or the store's.
- Crossings: `project_population` once per request; `receive` once per file; `file_tables` (execute's port, implemented
  here) once per file; `run` once per request; inside `run`, operators' `run` once per node per cross-file program and
  `TableRead::matching` once per key set, never once per row or cell. `S` is a generic everywhere. Two `dyn`s remain,
  neither on a hot path: `CrossFileRun.budget` (once per rule) and `ProjectInputs.observe` (once per entry looked up,
  as execute's `ScanFile.observe`); execute's own port is `&dyn ProjectTablesPort` by its choice (once per file).
- Nu-hex: section 5d. Sealed: no public trait here is meant to be implemented outside the crate except by tests; a
  sealing decision is not made (section 11).

## 10. Decisions for the owner
1. **The name.** Example: `pub trait ProjectUseCase` already exists at `language/src/inbound/use_cases/project/interface.rs:27`
   on both sides; inbound/lint imports both crates, so two traits of one name meet there.
   (A) crate `project`, trait `ryo_project::ProjectUseCase`: two definitions of one word. (B) crate renamed
   `idiomatic-sh-ryo-cross_file`, trait `CrossFileUseCase`; the language's untouched. (C) keep `project`, rename the
   language's trait `ManifestUseCase` in all nine languages and the SDK's author surface. Why it matters: "one
   definition per concept"; in Ryo "project" already means the repository's manifests (`RowScope::Project`,
   `ProjectFacts`, project relations), which this crate also reads. **My pick: B**; main's own word was `cross_file`.
   The draft's trait is already `CrossFileUseCase`; B renames only the package.
2. **How a carried table crosses workers** (R3a). Example: the tip copies each carried table into a `CarriedBlock` on the
   worker and back into a `Block` on the coordinator (`population.rs:618, 629`), and separately shares read-only
   `Arc<[Words]>` blocks with the closed workers (`SharedBlock`, `population.rs:172`); main kept them in a port with a
   spill adapter. Why: the store pools columns per thread and the plan releases a table on the worker that made it; carried
   tables break that. What we get: one way, owned by the store. **My pick:** a finished table can be frozen into a
   read-only, `Send + Sync` table any worker reads (the tip's sharing path, no copy), added to the table contract
   before its spike, with the spill adapter as a store adapter later.
3. **Restoring carried rows from the kept store: drop it.** Example: `world.restore_frontier(slot, parts)`
   (`stored.rs:1160`) refills `World` from the kept store; those blocks are 48.9 of fastapi's 53 MB store. Why: perf
   row 53 (findings only: fastapi 840 KB store, 23 ms warm) replays only file-local programs, so cross-file rules run
   again every time. What we get: no carried row ever crosses the cache port; the cost is re-running the cross-file
   rules on a warm run (7 of 413 Ruff programs). **My pick: drop**; measure the warm and one-edit cells of the bar
   table before agreeing.
4. **The disk leaves this crate.** Example: `Authority::observe` calls `std::fs::metadata` (`project.rs:327`) and the
   SDK builds `Environment` (`entry.rs:432`), a memo of disk lookups shared by every file's visit and the project read.
   Why: an application crate does no I/O; both execute and this crate need the memo, and execute cannot import this
   crate. **My pick:** `Environment` and `Authority` to outbound/corpus, offered as the language's `FileProject` (the
   tip already has `impl l::FileProject for Environment`, `project.rs:629`); this crate and execute see only the trait
   object (`ProjectInputs.observe`, `ScanFile.observe`). The SDK then names no type of this crate's. (The per-file read
   of project relations stays here, behind execute's `ProjectTablesPort`, as execute's sheet decided.)
5. **The budget guard.** Example: main checked `within(memory.resident_bytes()?, run.budget, ..)` before each project
   rule (`ordinary/service.rs:34`, 6 case rows); the tip has no bound. It is a designed contract with no caller today.
   (A) restore it as `CrossFileRun.budget: &dyn Fn(RuleOrdinal) -> bool` (drafted), the edge asking the memory port, a
   refused rule answered `RuleAnswer::Refused` with an over-budget case execute is asked for (R1); (B) drop it, in your
   words. **My pick: A.**

## 11. Not checked
- Nothing was compiled or run. The draft's signatures are not type-checked: `ProjectPopulation<'a>` borrowing both
  `self` and `Programs`, `CrossFileRun`'s lifetimes, and the generic `CrossFileUseCaseImpl<L, W>` are proposals.
- Run 28 Nov over the whole folder: `check-quotes.py` 0 failures; `check-types.py` no type of this sheet defined twice;
  `check-graph.py` fails for this sheet only on `scheduler` (no sheet yet). Every quote was also re-checked by hand
  (count in the final report).
- The execute and programs sheets were changing while this was written (execute's `Carried` gained `inputs`/`notes`,
  its port lost a layout number). Their names here, and the language sheet's `LanguagePort`, `FileProject`, `FilePath`,
  `Refused`, are as read on 28 Nov; a later change there must be carried here. `ryo_scheduler::SchedulerPort` is a
  placeholder (no scheduler sheet). The store's listing (no sheet yet) names the sendable form of a carried table as
  "reached through decision 4 (d)" of its sheet: R3a must be reconciled with it.
- Cross-request repeats: the tip answers an equal closed rule over equal inputs once across the requests of a batch
  (`readers.duplicate`, `Evaluated`, `batch.rs:800`). `run` is drafted per request, which loses that unless lint passes
  several requests to one `run`. Not measured how much it is worth.
- Whether `ryo_relations::UnknownName` holds what the tip's `unknowns: Vec<(String, String)>` holds; whether
  `partial_files` should be per relation (the tip's `incomplete` is keyed by relation) rather than a file list, given
  that execute's `PartialRead` already names the relation but not the file.
- The bodies of `finish/session/demand.rs`, `contribution.rs`, `closed_workers::inputs`, `evaluate_covered` and
  `project::collect`: read only by signature and call site. Which parts of `demand.rs` belong to domain/plan and which
  to execute is a placement claim, not a reading.
- Hosted rows are drafted as the request's `&[S::Table]`, as execute's `ScanFile.hosted`; bound observations (main's
  `PlanInputs::observation`) are not placed.
- The 46 main boundary tests were classified by directory name, not read; the split 35 here / 11 to plan is a first
  cut.
- Main's line count (5,192) includes `core::application::project_context` and `core::domain::project`, which may be
  the language's project context rather than this phase; not read.
- Whether `CrossFileUseCase` should be sealed; whether `Handover` needs `Send` stated (it must cross from workers to the
  coordinator; that depends on R3a).
- No measurement: no figure here was re-measured; rows 4, 7 and 23 are quoted from `perf-gains.tsv`.
- The Tastemaker `store_matrix.py` and retention benches were not read beyond the one row quoted.
