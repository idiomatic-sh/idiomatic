# application/execute
STATUS: drafted (28 Nov 2026; not reviewed, not agreed). Bead ryo-fmx3.1.13.23.4.11. Every line naming a `ryo_table` type is UNPROVEN until `domain/table/spike.md` records a pass.

Revisions: MAIN = f4ae3bd802 (release 0.2.0, live). TIP = 2a207464c (perf branch, build 139). B133 = f0219b3b3 (cited only where the
thing existed there and on neither side). Read with `git -C /workspaces/idiomatic-public-benchmark show "REV:PATH"`; no checkout.
X = `crates/ryo/crates/executor/src`. The drafted listing is `api.txt` beside this file.

## 1. Owns

Running a request's prepared programs over one file, and handing that file's answers up once.

One thing. "Run" and "hand up" are the two ends of the same per-file call, not two concepts: the answers exist only while the
file is lent (its tree, words and fact sources die when the language's callback returns), so whoever runs the file must also
locate and hand up its rows. What this crate does NOT own, though the tip's `visit` does all of it today: preparing and binding
programs (domain/programs), deciding which programs can answer in a file (application/screen, called from here), what each
operator means (domain/operators, called from here), spreading files over workers (inbound/lint and outbound/scheduler),
reading bytes and file metadata (outbound/corpus), the kept store (outbound/cache), and the cross-file phase
(application/project, which reads what this crate hands up and lends back what a file needs of the project).

## 2. On main (f4ae3bd802)

**Where.** The concept is spread over three crates, one call deep each:
- `core/src/application/file_analysis` (2,716 lines without tests, 3,315 test lines; 49 files): the per-worker visit of one
  file: read it, gate the rules, parse it, call the engine, decode what it answered.
- `execution/src/inbound/use_cases/execution` (187 lines without tests; 9,547 test and fixture lines in 80 files): the use
  case that begins a request's execution, and 75 `pub use` re-exports that make up its vocabulary (`interface.rs:5-79`).
- `execution_runtime/src/application/file_dag`: `program/request_execution` (260 lines), `program/file_session` (199),
  `fact_lookup` (220) and `file_pass` (2,099): the one per-file route, `run_file`.
- Total for the concept, without tests: about 5,681 lines (2,716 + 187 + 260 + 199 + 220 + 2,099).

**Its contract, word for word.** The use case that begins a request:
```
pub trait ExecutionUseCase {  @f4ae3bd802:crates/ryo/crates/execution/src/inbound/use_cases/execution/interface.rs:82
fn execution(  @f4ae3bd802:crates/ryo/crates/execution/src/inbound/use_cases/execution/interface.rs:85
pub enum Observing<O: ?Sized> {  @f4ae3bd802:crates/ryo/crates/execution/src/inbound/use_cases/execution/interface.rs:95
pub struct ExecutionUseCaseImpl;  @f4ae3bd802:crates/ryo/crates/execution/src/inbound/use_cases/execution/use_case.rs:11
pub const fn new() -> Self {  @f4ae3bd802:crates/ryo/crates/execution/src/inbound/use_cases/execution/use_case.rs:16
```
`execution(&self, program: &PreparedExecutionProgram, corpus: &[CorpusInput], observer: Observing<dyn ObserverPort>) ->
RequestExecution` (`interface.rs:85-90`). The handle it returns runs one file per call:
```
pub trait ProgramRequestExecutionPort {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/request_execution/interface.rs:18
fn observations<'a>(  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/request_execution/interface.rs:25
fn finish(self)  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/request_execution/interface.rs:31
fn run_file(  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/request_execution/interface.rs:37
fn project_phase(&self) -> Result<ProjectPhase<'_>, OperatingError>;  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/request_execution/interface.rs:49
pub fn begin(  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/request_execution/service.rs:123
```
`run_file(&self, input: FileRunInput<'_>, symbols: &mut Symbols, capture: CaptureContext<'_>, facts: &mut dyn FactsWorker) ->
FileRules` (`interface.rs:37-43`): one call per file, every rule of the file in rule order, answers back as one value. That is
the shape this draft keeps. Its types:
```
pub struct CaptureContext<'a> {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/file_session/types.rs:42
pub struct FileRunInput<'a> {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/file_session/types.rs:53
pub enum FileStop {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/file_session/types.rs:102
pub enum RuleAfter {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/file_session/types.rs:112
pub struct RanRule {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/file_session/types.rs:133
pub enum FileEnd {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/file_session/types.rs:143
pub struct FileRules {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/file_session/types.rs:153
pub struct Partial {  @f4ae3bd802:crates/ryo/crates/execution_domain/src/domain/coverage/types.rs:25
```
What `run_file` is handed for the worker's rows (main's per-worker store, the thing `TablePort` replaces):
```
pub trait FactsWorker: Send + Sync {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/outbound/facts/interface.rs:34
```
And the per-file driver in core, public only through its constructor:
```
pub struct FileAnalysis {  @f4ae3bd802:crates/ryo/crates/core/src/application/file_analysis/service.rs:46
pub fn new(  @f4ae3bd802:crates/ryo/crates/core/src/application/file_analysis/service.rs:101
pub(crate) fn visit_file(  @f4ae3bd802:crates/ryo/crates/core/src/application/file_analysis/service.rs:117
```
`FileAnalysis::new(corpus: Arc<dyn CorpusPort>, language: Arc<dyn LanguagePort>, digest: Arc<dyn DigestPort>, memory: Arc<dyn
MemoryPort>)` (`service.rs:101-106`).

**Public items, counted by reading the files above:** 29. `execution`: 5 (`ExecutionUseCase`, its method, `Observing`,
`ExecutionUseCaseImpl`, `new`) plus 75 re-exports; `execution_runtime`: 22 (`ProgramRequestExecutionPort` and its 4 methods,
`RequestExecution`, `ObservationArenas`, `begin`, and the 14 types of `file_session/types.rs`: `SourceLanguage`,
`SemanticSupport`, `KindSupport`, `FinishedReadiness`, `CaptureContext`, `FileRunInput`, `Partiality`, `FileStop`, `RuleAfter`,
`EmittedRoot`, `RanRule`, `FileEnd`, `FileRules`, `FrontierCaptures`); `core`: 2 (`FileAnalysis`, `new`). `FactsWorker` (1
trait, 5 methods) is counted with the store, not here.

**Who implements it.** `impl ProgramRequestExecutionPort for RequestExecution` and `impl ExecutionUseCase for
ExecutionUseCaseImpl`, one each:
```
impl ProgramRequestExecutionPort for RequestExecution {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/request_execution/service.rs:17
fn run_file(  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/request_execution/service.rs:43
```

**Who calls it.** Product: one call site, core's evaluation step, reached per job from core's streaming:
```
let ran = visit.setting.services.run_file(  @f4ae3bd802:crates/ryo/crates/core/src/application/file_analysis/evaluation/service.rs:112
.visit_file(worker, course.visit, course.files, job);  @f4ae3bd802:crates/ryo/crates/core/src/application/file_analysis/streaming/service.rs:181
```
`FileAnalysis::new` is called by the command line (`cli/src/resources/query/run/composition.rs:69`), core's lint acceptance
(`core/src/inbound/use_cases/lint/acceptance.rs:104`), two command-line test fixtures (`cli/tests/fixtures/replay/{base,
counting}.rs`), one core fixture and four core tests. Tests: `run_file` appears on 106 non-comment lines under
`execution/tests` and `execution/src/inbound/use_cases/execution/fixtures*` (by `git grep -n run_file`; one helper per suite
wraps it).

**Proof.**
- `execution/tests/boundary_contract`: 162 files, 149 `#[test]`, in 15 groups (`combined`, `compiled_rules`, `completion`,
  `file_intents`, `file_run`, `mode_order`, `preflight`, `project_phase`, `publication`, `refusal_order`, `resources`,
  `scheduler`, `shared_walk`, `submission`, `union_rows`), almost all through `run_file`.
- `execution/src/inbound/use_cases/execution`: 54 `#[test]` and one acceptance scenario ("An empty request begins and
  finishes").
- `execution_runtime/.../file_pass/service_test/{counters, demand, lifetimes, roots}`: 4 `#[test]` over case tables (19
  labelled cases in `demand` and `lifetimes`); `program/{preparation, request_identity}`: 3.
- `core/src/application/file_analysis`: 13 `#[test]` (`service_test.rs` and `service_test/{capture, failure, gating, manifest,
  recovery, replay, resident, visit}`, `language_source/service_test.rs`).
- One divan bench on the store the call is handed: `execution_runtime/benches/performance/facts/bench.rs` ("the facts hex's
  adapters back to back").

**Tastemaker entries on it** (`crates/ryo/tastemaker.toml` at f4ae3bd802): line 9 (`file_dag` is its own workspace member);
lines 18-19 (what `execution_runtime` may depend on); line 65 (`derived-view-acquisition`, owner concept `file_dag`); line 68
(`runtime-authority`); line 69 (`program-implementation-boundary`); line 72 (`operator-meaning-once`); lines 73-74
(`fact-reads-through-lookup` on `file_pass/**` and `fact_lookup/**`: the per-file pass reads facts only through its lookup);
line 75 (`driver-implementers`); line 76 (`entry-traits-witnessed`, naming `execution/interface.rs`); lines 198-201 (`nu-hex` and
`crate` for `execution_runtime` and `execution`). These long lines are cited by number only.

## 3. On the perf branch (2a207464c)

**Where.** No crate. The concept is the proof-of-concept `executor`, in about 4,700 lines across 19 files:
`X/check_session.rs:540-1654` (1,115 lines: `Bindings`, `Bound`, `bind`, `Version`, `versions`, `ReaderState`, `HeldFile`,
`visit`) and `:146-180` (`Clock`); `X/check_session/{worker.rs 238, jobs.rs 23, file_programs.rs 134, named.rs 119,
completeness.rs 86, report_slots.rs 167, refusal.rs 140, shared.rs 84}`; `X/finish/{session.rs 671, answer.rs 356,
answer/report.rs 69, frame.rs 70, known.rs 293, emptyprop.rs 297, fill_len.rs 34, lazy_view.rs 47}`; `X/reader_run/packet.rs`
(689); `X/host_columns.rs` (37). (Line counts include a few inline test modules.)

**The interface actually called.** One function of 17 parameters and 790 lines, entered once per worker shard and looping over
files inside:
```
pub fn visit<L: FileSource>(  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:864
pub struct ReaderState {  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:844
pub struct HeldFile {  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:851
pub struct Clock {  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:146
pub struct Source {  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:180
```
What one `visit` does, by line range (read at the tip):
- once per shard (`:883-990`): interns the reader's words into the worker's `Run`, merges the screens, then for every binding
  calls `host(binding, &mut run)`, the requirement cache, `BoundRequirements::bind` (interning through `run.sym`), `bind`, and
  `merged.add`; then one shared plan and one evaluator for the shard:
```
let (scope, screen, positions) = state.screens.merged(parser, programs, program_of, &needed, cut_of);  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:907
let requirements = Rc::new(BoundRequirements::bind(&clauses, &mut prerequisite_clauses, |word| run.sym(word)));  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:966
let mut b = bind(&mut registry, program, parser, &mut run, tables,  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:979
let (plan, supplied) = merged.finish();  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:989
let mut exec = Exec::new(&plan);  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:990
pub fn bind<'p, L: FileSource>(  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:601
pub struct Bound<'p> {  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:579
pub struct Bindings<'p> { index: Vec<usize>, entries: Vec<Result<Bound<'p>>> }  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:550
pub struct ReaderPlan {  @2a207464c:crates/ryo/crates/executor/src/check_session/shared.rs:10
pub fn finish(mut self) -> (Rc<Plan>, rustc_hash::FxHashMap<String, Rc<Table>>) {  @2a207464c:crates/ryo/crates/executor/src/check_session/shared.rs:75
```
- per path group (`:996-1006`): reads the bytes itself, with `std::fs::read`, one read per inode and content:
```
pub fn versions<'s>(  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:726
pub struct Version<'s> {  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:707
pub struct ReadFailed {  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:718
```
- per file version (`:1009-1645`): computes the file's demand, builds the parse request, opens the language's numbered reader
  once per worker and asks it to lend the file; everything else runs inside the language's callback, because the tree lives
  only there:
```
let mut consume = |loan: Option<(l::NumberedFile<'_>, &mut l::Symbols)>| {  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:1095
let reader = numbered_reader.get_or_insert_with(|| supplier.open());  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:1618
reader.visit(  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:1621
assert!(!called, "a file version was supplied twice");  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:1628
```
  Inside: the content screen (`:1175`), fill of the facts the surviving programs read (`:1223`, `packet.index` `:1275`), the
  bound-proof screen (`:1286`) which may answer a rule outright (`:1305`), named tables (`:1321-1361`), the line-count host
  table (`:1375-1390`), project-layout tables (`:1392-1430`), then per active rule either a closed contribution into the
  cross-file `World` or a file-only evaluation:
```
&& contents.excludes(b.test, &b.requirements, |out| {  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:1175
let filled = packet.fill_numbered(supplier, &missing, fact_words.as_deref_mut().expect("file word dictionary"));  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:1223
screen.exclude(&positions, &mut active, &mut bound, &mut run, tree, provider.projects_words.then_some(provider.text.as_slice()), &distinct, &held.literals, patterns, &mut held.proof_atoms,  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:1286
let version = version.get_or_init(|| {  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:1544
let make_frame = || finish::session::frame(  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:1547
let value = shared.as_mut().expect("project output was bound").contribute(b, &mut exec, current, &mut run, id.0, &mut facts_view, cut,  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:1563
let value = finish::session::pair(&mut exec, current, &mut run, &mut facts_view, b.plan, binding, &b.slots, &local_coverage);  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:1590
world.insert_empty(slot, &source.path, b, &mut run, cut);  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:1526
pub fn frame<'a, 't>(  @2a207464c:crates/ryo/crates/executor/src/finish/session.rs:398
pub fn pair(  @2a207464c:crates/ryo/crates/executor/src/finish/session.rs:412
pub fn covered_empty(  @2a207464c:crates/ryo/crates/executor/src/finish/session.rs:447
pub fn reachable_partials(  @2a207464c:crates/ryo/crates/executor/src/finish/session.rs:457
pub fn publish_locations(  @2a207464c:crates/ryo/crates/executor/src/finish/session.rs:546
```
- answers go into slots, per (request, file) pair, located with the lent tree:
```
pub fn set(&mut self, slot: usize, answer: Answer) {  @2a207464c:crates/ryo/crates/executor/src/check_session/report_slots.rs:67
pub fn locate(&mut self, path: &str, tree: Option<&crate::languages::NodeTable>, bytes: &[u8], coordinates: &mut crate::languages::contract::FindingCoordinates) -> Result<(), crate::check_session::refusal::Refusal> {  @2a207464c:crates/ryo/crates/executor/src/finish/answer.rs:199
pub struct Reported {  @2a207464c:crates/ryo/crates/executor/src/finish/answer/report.rs:6
pub struct Row {  @2a207464c:crates/ryo/crates/executor/src/finish/answer.rs:101
pub struct Words {  @2a207464c:crates/ryo/crates/executor/src/finish/answer.rs:18
```
- failures, per slot, as one enum that mixes load-time and run-time causes:
```
pub enum Refusal {  @2a207464c:crates/ryo/crates/executor/src/check_session/refusal.rs:7
pub enum ProviderFault {  @2a207464c:crates/ryo/crates/executor/src/check_session/refusal.rs:77
answers.set(slot, Err("a relation filled on first use was not a complete answer".into()));  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:1607
|| "provider did not deliver exactly one numbered file".to_owned(), |e| e.to_string());  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:1642
```
- fill on first use: the file's lazy fact reader is the executor's own `Provider` implementation:
```
pub struct Lazy<'a, 'd> {  @2a207464c:crates/ryo/crates/executor/src/reader_run/packet.rs:35
pub struct Packet {  @2a207464c:crates/ryo/crates/executor/src/reader_run/packet.rs:153
impl finish::Provider for View<'_, '_, '_> {  @2a207464c:crates/ryo/crates/executor/src/reader_run/packet.rs:471
```
- I/O inside the engine, besides `versions`: the executable bit is a `std::fs::metadata` per file:
```
pub fn executable(path: &Path) -> std::io::Result<bool> {  @2a207464c:crates/ryo/crates/executor/src/host_columns.rs:14
let path = format!("{}/{}", run.root_dir, run.paths[file]);  @2a207464c:crates/ryo/crates/executor/src/host_columns.rs:28
```

**Callers.** One chain: the SDK's `execute_round` -> `Batch::epoch_with_store` / `epoch` -> `read_pending` -> `dispatch` ->
`visit` (twice in `dispatch`: inline for one shard, through `jobs::run` for many):
```
pub(crate) fn execute_round(  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:37
self.batch.epoch_with_store(&self.languages, &mut self.state, Some(store));  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:392
pub fn epoch_with_store<L: session::source::Source>(  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/batch.rs:475
pub fn read_pending<L: session::source::Source>(  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/batch.rs:561
session::worker::dispatch(  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/batch.rs:604
pub fn dispatch<L: FileSource>(  @2a207464c:crates/ryo/crates/executor/src/check_session/worker.rs:55
visit(  @2a207464c:crates/ryo/crates/executor/src/check_session/worker.rs:85
visit(  @2a207464c:crates/ryo/crates/executor/src/check_session/worker.rs:149
let (results, children) = jobs::run(&shards, |shard| {  @2a207464c:crates/ryo/crates/executor/src/check_session/worker.rs:193
let _budget = finish::PoolBudget::for_workers(shards.len());  @2a207464c:crates/ryo/crates/executor/src/check_session/worker.rs:194
world.receive(transfers, sources);  @2a207464c:crates/ryo/crates/executor/src/check_session/worker.rs:236
pub fn run<I, T, F>(jobs: I, work: F) -> (Vec<T>, f64)  @2a207464c:crates/ryo/crates/executor/src/check_session/jobs.rs:5
```
Files per crate that reach the concept: executor 3 (`replay/batch.rs`, `worker.rs`, `replay.rs:225` for the command line's
replay), sdk 1 (`session/entry.rs`). The SDK also reads the shard's counters to emit the event the benchmarks read:
```
tracing::event!(name: "run", target: "ryo::node_counters", tracing::Level::INFO, runs = runs, nodes = nodes);  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:405
```

**Public items:** about 205 `pub` item declarations in the 19 files (192 in the 18 helper files by
`grep -c -E '^\s*pub (struct|enum|trait|type|fn|const|static|mod|use) '`, 12 in `check_session.rs:540-1654`, `Clock` with 27
public fields). Every one is public only because the executor is one crate with no internal seams.

**Proof.** 13 `#[test]` touch the concept: `refusal_test.rs` 5 (provider faults), `reader_run/packet.rs` 4 (the file's fill),
`finish/session.rs` 2, `finish/answer.rs` 1 (`closed_span_after_an_unlocated_node_places_the_finding`), `file_programs.rs` 1.
No test calls `visit` or `dispatch`; the per-file route is proved only end to end, by the twelve workloads of
`build/ports/short-check.sh` (evidence repository) and the SDK's tests. No acceptance scenario.

**Tastemaker entries** (`crates/ryo/tastemaker.toml` at 2a207464c): line 105 (`crate-executor`, the executor "keeps its own,
narrower lint table"); lines 31-32 (what `executor` may depend on); no `nu-hex-executor` entry. Line 71
(`entry-traits-witnessed`) still names `crates/execution/src/inbound/use_cases/execution/interface.rs` and
`crates/execution/tests/boundary_contract/submission/witness/fixtures.rs`, which do not exist at the tip (`git ls-tree` of
`crates/ryo/crates/execution/src` lists only `metadata` under `inbound/use_cases`): a stale entry.

## 4. What changed between them

- **Removed:** main's whole route. `execution_runtime` (`file_dag`, the facts, observer and operators ports) and `execution`'s
  use case went in `861c7de10e` (26 Nov, "the old executor goes"); core's engine arms in `ff4533b99a` (26 Nov); build 133's
  `engine` crate with `ScanUseCase` in `fc94354b0c` (26 Nov). With them went 149 boundary-contract tests, 54 use-case tests, 13
  file-analysis tests, 7 file-pass and program tests, the facts bench, and 133's 22 scan scenarios (section 6).
- **Added:** the executor's per-shard driver. It arrived by rename (`f2869d1f74`, 26 Nov, "move the vector executor to
  crates/ryo/crates/executor/src") from benchmark code; 34 commits touch `check_session.rs` since, and `git log --follow`
  counts 166 on it in all, so most of the gains it carries were made before the rename, under `crates/ryo/crates/engine/benches/vec`.
- **Changed, and measured:** one shared plan per reader with step results reused per file version (`5aee5787c1`, 24 Nov;
  64.064 -> 52.952 CPU-s); reader state released at delivery, the keep switch and parse cache removed (`887a8531e7`, 25 Nov;
  engine CPU 19.00 -> 16.61 s); fill on first use (`4b6cf1c93b`, 25 Nov); typed packed answers (`a28abaab18`, 24 Nov; peak
  1,845 -> 1,510 MiB); the pack lowered once per request (`28017029a9`, 25 Nov). And the loss: joining three drivers on 25 Nov
  with the most general as base (perf-gains row 6: fastapi 0.941 -> 1.702 CPU-s, start-up 73.6 -> 295.5 ms).
- **Lost that should not have been:**
  1. A per-file contract. Main had `run_file`, 133 had `scan`; the tip has none, so the screen, the fill, the cross-file
     hand-off, the store and the corpus are all reached from inside one function (`visit` takes `World`, `Run`, slots, and
     calls `std::fs`).
  2. Its proof: 149 + 54 + 13 + 7 tests and 22 scenarios (section 6) against 13 today.
  3. A frozen prepared set: 133's `EnginePort::state_for(&Programs)` built a worker's scratch from shared programs; the tip
     binds every program again in every shard (`:952-986`). Measured cost in perf-gains row 13: prep 1.637 s against 0.351 s.
  4. I/O out of the engine: main read bytes through `CorpusPort` (in `FileAnalysis`); the tip reads them in `versions` and
     stats files in `host_columns`.

## 5. The drafted public interface        (explains every line of api.txt that is changed or new)

The listing has 40 lines: 27 public items and 13 enum variants (1 `[133]`, 6 `[perf]`, 23 `[changed]`, 10 `[new]`). Against
29 public items on main (plus 75 re-exports) and about 205 on the tip.

### 5a. The claim it starts from, checked

Appendix A 3.9 drafts `ScanUseCase::scan<S: TablePort>(&self, programs: &Programs, enabled: &EnabledRules, file:
FileFacts<'_>, scratch: &mut ScanScratch<S>, out: &mut FileAnswers<S>) -> Result<ScanCounters, ScanError>` and `FileAnswers<S>
{ rows, carried, notices }`, "handed in: ScreenUseCase, operators' RunUseCase, a TablePort". Checked against the code:
- **Right:** one crossing per file; answers as one value per file, not 133's per-row callback; generic over the store; the
  screen and the run handed in. 133's names are real:
```
pub trait ScanUseCase: Send + Sync {  @f0219b3b3:crates/ryo/crates/engine/src/inbound/use_cases/scan/interface.rs:24
fn scan(  @f0219b3b3:crates/ryo/crates/engine/src/inbound/use_cases/scan/interface.rs:32
pub trait FoundRows {  @f0219b3b3:crates/ryo/crates/engine/src/domain/findings/types.rs:30
pub struct ScanScratch(pub(crate) ScratchEngine);  @f0219b3b3:crates/ryo/crates/engine/src/domain/scratch/types.rs:41
fn state_for(&self, programs: &Programs) -> ScanScratch;  @f0219b3b3:crates/ryo/crates/execution_runtime/src/outbound/engine/interface.rs:56
```
- **Wrong, 1: `FileFacts` is not what a language hands the engine at the tip.** It exists and the nine languages' `walk`
  contracts still fill it, but the language crate adapts every walk into the numbered visit before the engine sees it
  (`walk/numbered.rs:1`, below), and the executor never names `ryo_language`'s `FileFacts` (its own `fact_view::FileFacts`
  is a different trait of the same name, `finish/fact_view.rs:172`). The live hand-off is the numbered lend: the engine opens
  the language's reader once per worker and the language calls back once per file with a `NumberedFile` that lives only for
  that callback:
```
pub struct FileFacts<'f> {  @2a207464c:crates/ryo/crates/language/src/domain/facts/types.rs:35
//! The one adapter from a language's walk to the numbered visit the one-path driver consumes: every language that implements  @2a207464c:crates/ryo/crates/language/src/inbound/use_cases/walk/numbered.rs:1
pub trait FileFacts {  @2a207464c:crates/ryo/crates/executor/src/finish/fact_view.rs:172
pub struct NumberedFile<'a> {  @2a207464c:crates/ryo/crates/language/src/inbound/use_cases/parse/named.rs:53
pub trait NumberedVisit: Send + Sync {  @2a207464c:crates/ryo/crates/language/src/inbound/use_cases/parse/named.rs:139
fn open(&self) -> Box<dyn NumberedReader + '_>;  @2a207464c:crates/ryo/crates/language/src/inbound/use_cases/parse/named.rs:151
fn visit(&mut self, request: super::types::ParseRequest<'_>,  @2a207464c:crates/ryo/crates/language/src/inbound/use_cases/parse/named.rs:161
```
  So `scan` cannot take an already-lent file without inverting the language's loop. The draft has `scan` take the file's
  identity and bytes and do the lend itself (decision 1).
- **Wrong, 2: `programs` per file.** The programs, screens and language are the same for every file of a worker; passing them
  per file is noise and invites per-file set-up. They go into `open`, once per worker (decision 2).
- **Wrong, 3: `Carried<S>` was not placed.** `application` is ordered `screen < execute < project`, so execute cannot import a
  type of project. The draft declares `Carried<S>` and `FileNotes` here and project imports them (decision 3).
- **Missing:** a way for project-level tables to reach a file (the tip's `Layout::slice` inside `visit`), the per-worker
  store's return, the dictionary answers are numbered in, and `notices`: 133's `WorkNotice` has no code on main or the tip,
  so it is not drafted.

### 5b. The calls: `open`, `scan`, `close`

133's trait name and bounds are kept; the three methods are drafted:
```
pub trait ScanUseCase: Send + Sync {  @f0219b3b3:crates/ryo/crates/engine/src/inbound/use_cases/scan/interface.rs:24
    fn open<'p, S: TablePort>(&self, prepared: Prepared<'p, S>, store: S) -> ScanScratch<'p, S>;   @new
    fn scan<S: TablePort>(&self, file: ScanFile<'_>, scratch: &mut ScanScratch<'_, S>, out: &mut FileAnswers<S>) -> Result<ScanCounters, ScanError>;   @new
    fn close<S: TablePort>(&self, scratch: ScanScratch<'_, S>) -> S;   @new
```
- `open` is 133's `state_for` given the store it will make tables in. It is what one worker calls once per language before its
  first file of that language: it opens the language's numbered reader (the tip does that lazily per worker, `:1618`), makes
  the operators' `FileBuffers::new(store)` and a `SymbolLayer` from the frozen dictionary the programs carry. It does **no**
  binding, merging or interning: the tip's `:952-990` moves to domain/programs, once per language when rules load.
- `scan` is one file, start to end: work out what the enabled rules read; ask the language to lend the file once; inside the
  lend call the screen with the file's lazy fact reader; fill on first use; run each surviving rule's emitted roots through
  `RunUseCase::run`; read project-layout tables through the port; locate rows while the tree is lent; write `out`. The
  language's per-file `Symbols` are cleared per file as today (`:1617`).
- `close` gives the store back, so its tables are released into the pool of the worker that made them (perf row 37).

### 5c. What `open` and `scan` take

- `Prepared<'p, S> { programs, screens, language, project }`. `programs: &Programs` is domain/programs' frozen set
  (`Send + Sync`, perf row 13); `screens: &Screens` is application/screen's prepared screens, frozen once per session (perf row
  19); `language: Option<&LanguageConnection>` is the one shape of a linked language the language sheet settles (Appendix A
  3.11), `None` for files no language claims (the tip's `reader_id = None`); `project: Option<&dyn ProjectTablesPort<S>>` is
  how project-level tables reach a file. All four replace `visit`'s parameters `parser`, `programs`, `program_of`,
  `contracts`, `cut_of`, `layouts` and its `state.screens`.
- `ScanFile<'f> { file, path, read, rules, named_language, project }`. `file: FileId` and `path: SourcePath` are the run's
  number and the path a rule reads; `read: Result<&FileVersion, &CorpusError>` is the corpus's file version (bytes, content key,
  stamp, and, a requirement this sheet puts on outbound/corpus, the executable permission) or why it could not be read (with
  "missing" told apart from "unreadable", the tip's `ReadFailed { reason, missing }`); a failed read is still scanned as a
  host-only file, as the tip does (`:761-769`). `rules: EnabledRules` are the prepared programs that apply to this file (the
  tip's `FilePrograms` jobs without the answer slots, which are lint's). `named_language` is the tip's
  `reader.explicit(source.reader.is_some())` (`:1620`). `project: Option<&dyn FileProject>` is the repository observation the
  language may consult while the file is lent (the tip's `source.environment`).

### 5d. What `scan` hands up

`FileAnswers<S> { rules, carried, notes }`, one per file, reused across files (`clear`):
- `rules: Vec<(RuleOrdinal, RuleAnswer<S>)>`: per enabled rule, `Rows { table, partial }` (one typed table in the worker's
  store, rows already located), `Empty { partial }` (no table made: most rule runs answer nothing) or `Refused(RuleRefusal)`.
  `partial: Vec<PartialRead>` is the tip's per-answer "unknowns" (relations read in part that matter to the rule).
- `carried: Vec<Carried<S>>`: what a cross-file rule takes out of this file, per (rule, input), only the columns the plan's
  cut says cross; `rows: None` is the tip's `world.insert_empty`. project receives it (its `World::carry` today).
- `notes: Option<FileNotes>`: the provider's project notes and the manifest text, the tip's `project::Contribution`
  (`project.rs:277`), handed up even when no rule runs on the file.
- `ScanScratch::store` and `ScanScratch::symbols` let the caller read, decode and release the answer tables before the next
  file; answer `Symbol` cells are numbered in the worker's layer (symbols sheet), replacing the tip's
  `Arc<RwLock<answer::Words>>`.

### 5e. The port and the failures

- `ProjectTablesPort<S>::file_tables(layout, notes, file, store, out) -> Result<Vec<PartialRead>, RuleRefusal>`: the tables of
  one project layout restricted to one file, written into the worker's store, named by the plan's `RelationInput`. It is the
  tip's `Layout::read_file` then `Layout::slice` (`project.rs:196`, `:159`), which `visit` reaches directly today
  (`:1392-1428`). Implemented by application/project.
```
pub fn slice(&self, run: &mut finish::Run, aliases: &[(l::Relation, String)], path: &str, file: l::FileId, read: Option<&FileRead>) -> Vec<(String, Rc<Table>)> {  @2a207464c:crates/ryo/crates/executor/src/check_session/project.rs:159
pub fn read_file<L: super::FileSource>(  @2a207464c:crates/ryo/crates/executor/src/check_session/project.rs:196
```
- `RuleRefusal { Operator(OperatorFault), Provider { relation, fault }, Relation { relation, cause }, IncompleteFill }`: one
  rule's failure in one file. The tip's `Refusal` minus `MissingParameter`, `UnknownKind`, `UnknownRelation` (decided when rules
  load: domain/programs) and minus `Other(String)` (typed here, or the operators' `OperatorFault`). `ProviderFault` is the tip's
  four variants verbatim.
- `ScanError { NotLent { cause }, ProjectTables(RuleRefusal) }`: what stops the whole file; every enabled rule of the file is
  then answered with it by the caller, as the tip does (`:1640-1644`, `:1416-1420`). 133's `ScanError` variants were one rule's
  faults; they are `RuleRefusal` here.
```
pub enum ScanError {  @f0219b3b3:crates/ryo/crates/engine/src/inbound/use_cases/scan/errors.rs:22
pub struct PartialCause(pub Box<str>);  @2a207464c:crates/ryo/crates/values/src/domain/stored_facts/types.rs:8
pub struct ReadInPart {  @2a207464c:crates/ryo/crates/values/src/domain/stored_facts/types.rs:44
```
- `ScanCounters` (8 counts in two newtypes, `RuleCount` and `StepCount`): the tip's per-file `Clock` counts without its
  timings (the caller times). `step_runs` and `nodes_made` are what lint sums into the `ryo::node_counters` event.
```
pub struct ScanCounters {  @f0219b3b3:crates/ryo/crates/engine/src/domain/findings/types.rs:172
```

### 5f. How it is built

`ScanUseCaseImpl::<Sc, R>::new(screen: Sc, run: R)`, generic over `Sc: ScreenUseCase` and `R: RunUseCase`, so neither hot call
goes through `dyn`. 133's took a pattern port, which the operators sheet removes:
```
pub struct ScanUseCaseImpl {  @f0219b3b3:crates/ryo/crates/engine/src/inbound/use_cases/scan/use_case.rs:35
pub fn new(matcher: Arc<dyn PatternSemantics + Send + Sync>) -> Self {  @f0219b3b3:crates/ryo/crates/engine/src/inbound/use_cases/scan/use_case.rs:61
```
The edge (lint's composition) builds it once; lint's worker job calls `open`, then `scan` per file, then `close`.
Privately, the crate implements `ryo_operators::PlanInputs` (the file's lazy facts, the lent tree, carried and hosted rows) and
`ryo_operators::Resources` (the worker's `SymbolLayer`, the file's bytes, batch rendering) for the open file.

### 5g. What stops being public, and the counts

| Side | Public items in the concept | In the draft |
|---|---|---|
| main f4ae3bd802 | 29 (execution 5, execution_runtime 22, core 2) + 75 re-exports | all go; `run_file` -> `scan`, `begin` -> `open` |
| build 133 f0219b3b3 | `ScanUseCase`, `scan`, `FoundRows` (2 methods), `FoundTarget`, `FoundCells`, `ScanCounters`, `ScanScratch`, `ScanError`, `WorkNotice`, `EnginePort::{state_for, measuring_state_for, ledger, scan}` | 4 names carried (`ScanUseCase`, `ScanScratch`, `ScanCounters`, `ScanError`), all changed |
| tip 2a207464c | about 205 in 19 files | none public; listed in api.txt `# NO LONGER PUBLIC` |
| drafted | 27 items + 13 variants | — |

## 6. Proof it brings

| Proof | Source | Fate |
|---|---|---|
| 149 boundary-contract tests through `run_file` | main `execution/tests/boundary_contract/**` (162 files) | **restated**, by group, against `scan`. They build main's `PreparedExecutionProgram`, `FileRunInput` and `FactsWorker`, which go; what they assert (rule order, intents, delivery, partiality, refusal order, shared walk, union rows) is behaviour this crate keeps. How many port is not checked (section 11) |
| 54 execution use-case tests, 1 scenario | main `execution/src/inbound/use_cases/execution/**` | restated where they test a file's run; the request-begin ones have nothing to test (`open` holds no request) |
| 13 file-analysis tests | main `core/src/application/file_analysis/service_test*` | split: `visit`, `capture`, `failure`, `recovery`, `resident` restated against `scan`; `gating` to screen, `replay` to cache, `manifest` to project |
| 4 file-pass case tests (19 labelled cases) | main `execution_runtime/.../file_pass/service_test/{counters, demand, lifetimes, roots}` | **restated**: `demand` (a relation no running rule reads is not asked for) and `lifetimes` (rows released after their last reader) are the two properties this crate must keep |
| 22 scan scenarios, 3 tests | 133 `engine/src/inbound/use_cases/scan/{acceptance.feature, use_case_test*}` | **restated**: the scenarios "a relation no enabled program reads is never filled", "a file without a rule's anchor costs that rule no step", "a node whose input is empty in the file is never evaluated", "a table is made on its first read and once, however many programs read it", "a refused or partial relation is a fault, never empty rows" are this crate's contract tests; the operator-meaning ones (as-of, group-max, follow) belong to the operators sheet; the three work-allowance scenarios are not carried (no code on main or the tip) |
| 5 provider-fault tests | tip `X/check_session/refusal_test.rs` | carried as they are (they test `ProviderFault`, which is moved verbatim) |
| 4 fill tests | tip `X/reader_run/packet.rs` (`resolving_published_terms_...`, `reset_reclaims_a_local_population_...`, `a_relation_the_provider_does_not_advertise_...`, `a_diagnostic_identity_is_local_...`) | carried, private to the crate |
| 3 answer tests | tip `X/finish/answer.rs` 1, `X/finish/session.rs` 2 | carried; `closed_span_after_an_unlocated_node_places_the_finding` is the one test of locating rows while the tree is lent |
| twelve workloads, identical findings | evidence repository `build/ports/short-check.sh` | the regroup's gate; not a seam test |
| **missing** | — | a seam test that `scan` crosses the language once per file (row 24); one that a relation is filled at most once per file and only when read (row 12); one that `open` binds nothing (row 13: count `intern` calls on a frozen `Programs`); one that `FileAnswers` is the only output (no write to project state); none exists on either side |

## 7. Perf gains it carries

From `perf-gains.tsv` (54 rows). Rows whose crate column names execute, then the three the brief marks at risk, then rows whose
code sits in `visit` though another crate owns them.

| Row | Gain (measured) | State | What in the draft keeps it |
|---|---|---|---|
| 6 | three drivers joined (LOSS: fastapi 0.941 -> 1.702 CPU-s, start-up 73.6 -> 295.5 ms) | live | No admission in this crate: `open` takes `Programs` already admitted and `scan` takes `EnabledRules`; nothing here re-shapes a request. The double admission (SDK `admit`, `entry.rs:477`, then `Batch::admit`, `batch.rs:384`) is lint's and programs' to make one |
| 9 | keep switch and parse cache removed (19.00 -> 16.61 s; 1,748 -> 1,479 MiB) | live | No keep flag and no cache in any signature; a file's tree, words and bytes are borrowed for one `scan` and gone at its end; the memory budget reaches the store's constructor, not this crate |
| 12 | fill on first use (not measured) | live | The crate's private `PlanInputs::facts(relation, demand)` calls the lent file's `fill_columns` the first time a step or the screen reads a relation, once per file (`NumberedFacts::fill_columns`, `named.rs:77`; `fills_are_complete`, `:91`); nothing is filled up front |
| 15 | several files through one operator start (slower, +0.894 CPU-s) | lost | Nothing to keep: `scan` is one file; the interface does not batch files |
| 18 | typed packed answers (peak 1,845 -> 1,510 MiB; answer rows 329 -> 58 MB) | live | `RuleAnswer::Rows { table: S::Table }`: one typed table per rule per file, handed up once in `FileAnswers`; no per-row callback (133's `FoundRows::found`) |
| 20 | one shared plan per reader, step results reused per file version (64.06 -> 52.95 CPU-s) | live | **At risk.** `scan` runs every enabled rule of the file over one `FileBuffers`, so a step shared by two rules of the one plan is made once per file; the plan itself (equal steps merged, each rule an emit of it) is domain/plan's and domain/programs' and must arrive in `Programs`, not be merged in `open` (the tip merges per shard: `merged.add`, `:981`) |
| 38 | empty-run propagation (Django 16.28 -> 14.34) | live | Inside `RunUseCase::run` (operators sheet: "stops at the first empty gate"); this crate calls `run` per emitted root, never per step, and its `PlanInputs::facts` answers `Demand::Exists` without filling (the tip's `fill_len`) |
| 41 | known-empty before the step; per-node data flat at plan load (26.49 -> 18.14) | live | Same: the flat per-node data is `PhysicalPlan`'s (operators), built once; the known-empty test is in `run`; `scan` adds no per-step crossing |
| 47 | old engine stages 1-9 (228.6 -> 32.6) | lost | Its two ideas live on as rows 7 and 19, both kept (below and the screen sheet) |
| 8 | pack lowered once per request (888 -> 586 M instructions) | live | **At risk.** `scan` takes rule ordinals into one prepared set, never a plan per rule; nothing in `open` or `scan` lowers |
| 13 | one frozen prepared set for every worker (ten cores fastapi 3.62 -> 1.38 CPU-s; prep 1.637 against 0.351 s) | parked: 89d13a19e | **At risk.** `open(&Programs, ..)` builds a worker's scratch from a shared, frozen set and binds nothing: `bind`, `BoundRequirements::bind` with `run.sym`, `host(binding, &mut run)` and `layout.declare` (`:952-986`) move to load. If programs cannot be frozen, `open` would have to bind, and the draft says so by making that impossible (no `&mut Programs`, no interning call on it) |
| 7 | file-local rules leave the cross-file phase; inputs released after the last reader (dagster 26.13 -> 22.12; 1,245 -> 477 MiB) | live | `Carried` holds only what the plan's cut says crosses; file-local rules answer in `RuleAnswer` and carry nothing |
| 22 | columns and indexes once per file version (-5.6 s) | live | `scan` is called once per file version (aliases are lint's mapping); one frame per file serves every rule (`frame.get_or_insert_with`, `:1556`) |
| 24 | one provider call per file (dagster 13.18 -> 12.33) | live | `scan` lends the file once (`reader.visit`, `:1621`); screen, fill and every rule read through that one lend |
| 37 | pools for columns and tables (-14 to -17%) | live | `open` takes the store by value, `close` returns it: a table is made and released on one worker |
| 49 | requirement cache per prepared program and reader | live | moves with the binding to load (domain/programs); `scan` reads the cached requirements |
| 50, 51 | file versions shared by inode and content; absence known from the read | live | `ScanFile.read` is the corpus's `FileVersion` or `CorpusError`; `versions` and its `std::fs::read` leave this crate |
| 54 | `PoolBudget::for_workers` | live | the store's constructor (store sheet); `dispatch`'s call at `worker.rs:194` leaves |

## 8. Benchmarks that cover it

| Benchmark | Where | Reaches for today | After the draft |
|---|---|---|---|
| the lint images `vec`, `vec_ts`, `vec_rust` (and `tm`) | tip `X/../benches/lint_image` (own workspace) | the SDK only (`ryo::languages::Language`, `Linter`) | unchanged: they call the SDK; nothing in this listing is named by them. They are the measure of "not lost" for every row in section 7 |
| the twelve workloads and the bar table | evidence `build/ports/short-check.sh`, `build/tm-int/scripts/window-nine.sh` | the built binaries | unchanged |
| the node-counter readers | tip `crates/ryo/crates/benchmark/src/application/profile_evidence.rs:325`, `.../outbound/engine/adapters/local/writer.rs:24` | the tracing event `ryo::node_counters` with fields `runs`, `nodes`, emitted by the SDK from `Clock` (`entry.rs:404-405`) | lint emits the same event from the sum of `ScanCounters::{step_runs, nodes_made}`; target and field names must not change |
| the facts bench | main `execution_runtime/benches/performance/facts/bench.rs` | `FactsPort` memory and spill adapters | gone at the tip; its subject is the store's (store sheet), not this crate's |
| `benchmark/benches/performance/execution` | both sides | the benchmark crate's own process-execution port | not this crate (same word, different thing) |
| a per-file bench | — | — | none on either side: nothing measures one `scan` alone. The instruction counts by crate (`price.py`, `price-root-split.py`) are the only per-crate measure the gate has |

## 9. Checks
IMPORTS: table, symbols, relations, operators, programs, screen, language, corpus
CROSSED: per file

- Directions: `application` may import `domain` and `outbound`, and `screen` before `execute`: table, symbols, relations,
  operators and programs are domain; screen is the lower application crate; language and corpus are outbound. It imports no
  `plan`, `ryoql` (the programs carry everything), no `project` (project imports this crate: `Carried`, `FileNotes`,
  `ProjectTablesPort`), no `store` (generic over `TablePort`), no `scheduler` or `cache` (lint's).
- No layout word in a signature: answers are `S::Table`; no `Vec<u64>`, `Box<[u64]>`, `cols`, `Rc<Table>`; no slot, no
  `World`, no `Run`, no `Batch`.
- Crossing frequency: `open` and `close` once per worker and language; `scan` once per file version; the port once per file and
  layout; `RunUseCase::run` once per emitted root per file (inside); the language's lend once per file. Nothing per row or per
  cell crosses this crate's boundary. Store type fixed at compile time (`S: TablePort`); the screen and the run generic. The
  two `dyn` left are once per file or less: the language's numbered reader (the language sheet's shape) and
  `&dyn ProjectTablesPort<S>`.
- nu-hex inside: `src/inbound/use_cases/scan` (the trait and impl), `src/outbound/project_tables` (the port), `src/domain`
  (answers, refusals, counters). The tip's executor is flat `src` with its own narrower lint table (PLAN 10.5): the crate must
  take the workspace's lints.

## 10. Decisions for the owner              (each: the example from the code; why; what we get; my recommendation)

1. **How a file comes in: `scan` lends it, or is handed a lent file.** Example: the first draft's `scan(.., file:
   FileFacts<'_>, ..)` is 133's hand-off, but at the tip the language owns the per-file loop (`NumberedReader::visit` calls
   back once per file, `named.rs:161`) and the tree it lends dies with the callback (`check_session.rs:1095`). Why: if `scan`
   takes a lent file, lint must call the language and compute each file's demand itself, and locating rows, named tables and
   project-layout tables would have to happen in lint's callback, outside this crate. What we get: `scan(ScanFile)` with the
   file's identity and bytes; this crate asks the language once per file (row 24) and keeps "one file" whole. **Recommend: `scan`
   lends the file**, through whatever single lent-file shape the language sheet settles (`NumberedFile` today).
2. **No binding in `open` (the frozen set).** Example: every shard of `dispatch` runs `bind` for every binding and merges a
   new shared plan (`check_session.rs:952-990`), interning into its own `Run`. Why: that is perf row 13's lost prep (1.637 s
   against 0.351 s; ten cores 3.62 -> 1.38 CPU-s) and row 20's plan built per shard. What we get: `open(&Programs, ..)` with no
   mutable access to programs; binding, host-table numbering, requirement interning and the reader plan move to domain/programs,
   once per language. **Recommend: as drafted**; it puts a hard requirement on the programs sheet (programs bound per
   language, frozen, `Send + Sync`, symbols frozen with them).
3. **Where cross-file data crosses a file.** Example: `visit` writes into `World` itself (`contribute`, `:1563`;
   `insert_empty`, `:1526`; `world.unavailable`, `:1502-1506`) and calls `Layout::slice` (`:1426`). Why: project is above
   execute, so execute cannot name project's types; and a file must neither write the cross-file state nor reach into it.
   What we get: `FileAnswers.carried` and `.notes` going up, `ProjectTablesPort` (declared here, implemented by project) coming
   down. **Recommend: as drafted.** The alternative, moving `Carried` down into domain/plan, puts a run-time table in a domain
   crate that has no store.
4. **One fact reader for the screen and the steps.** Example: the tip's content screen fills facts through a closure
   (`contents.excludes(.., |out| ..)`, `:1175`) and the steps through the executor's `Provider` (`packet.rs:471`), sharing one
   `Lazy` per file. Why: fill on first use (row 12) holds only if both read through the same per-file cache. What we get: this
   crate implements operators' `PlanInputs` for the open file and lends it to the screen; the screen's per-file call takes
   `&impl ryo_operators::PlanInputs` rather than `FileFacts`. **Recommend: yes**; it is a requirement on the screen sheet.
5. **Answers as store tables, not packed rows.** Example: the tip hands up `Reported { rows: Vec<answer::Row> }` with `Row {
   layout: Arc<Layout>, words: Box<[u64]>, place }` and a shared `Arc<RwLock<Words>>` dictionary (`answer.rs:18, 101`). Why:
   `Box<[u64]>` is a layout word, and the lock is shared by every worker. What we get: `RuleAnswer::Rows { table: S::Table }`
   read and released through `ScanScratch::store`, words decoded through `ScanScratch::symbols`, all on the worker. The cost:
   lint must decode before `close`. **Recommend: store tables**; keep row 18's numbers as the check (peak and retained answer
   bytes on dagster no worse than build 139).

## 11. Not checked

- **Sheets this one leans on that do not exist yet.** domain/programs (`Programs`, `LayoutNumber`), application/screen
  (`ScreenUseCase`, `Screens`), outbound/language (`LanguageConnection`, `SourcePath`, `FileProject`, `ManifestText`),
  outbound/corpus (`FileVersion`, `CorpusError`): the names are Appendix A's or the tip's, assumed, and listed in api.txt as
  owned elsewhere. `ryo_relations::Bytes` exists at the tip (`relations/src/domain/relations/types.rs:63`) but is not on the
  relations api.txt. When those sheets land, this listing must be re-checked against them.
- **`RuleOrdinal` as the rule's key.** The draft uses operators' `RuleOrdinal` for a prepared program (the tip's binding:
  rule x host tables x layout). Whether programs keeps one ordinal per binding, and how lint maps it back to (request, rule,
  slot), is not checked.
- **The 149 boundary-contract tests:** counted, not read one by one; how many can be restated against `scan` is not known.
  Same for the 54 use-case tests. The 22 scenarios of 133 were read by title only.
- **The middle of `visit`** (`:1130-1374`: provider facts and words, the content screen with `populate`, the lazy fills,
  partial coverage, named tables, operator columns) was read in outline, not line by line; how much of it is screen's against
  this crate's is decided by the screen sheet and not verified here.
- **Hosted tables keyed by `File` cells** are numbered per worker today through `run.paths` (`batch.rs:623-637`); that they can
  be bound once at load (decision 2) assumes file numbers are fixed before load. Not checked.
- **Whether `open` per language per worker is the right grain.** The tip keeps one `ReaderState` per language per epoch
  (`batch.rs:571`) and one `Run` per shard. A worker that reads files of several languages would hold several scratches; the
  memory of that is not measured.
- **Line and column location** is drafted inside `scan` because the tree is lent; whether `FindingCoordinates` needs more than
  the tree and bytes (`answer.rs:199`) is not checked.
- **The tip's `Clock` timings** (`prepare`, `read_parse`, `screen`, `evaluate`, ...) are dropped from the contract on the claim
  that only `node_runs` and `nodes_made` are read outside the executor (found in `entry.rs:404-405` and the two benchmark
  files); other readers of `Clock` under `bench-hooks` were not searched.
- **No check script has run** over this sheet or api.txt (`contracts/checks/` does not exist). Every `@rev:path:line` quote
  in this sheet was re-checked by a small loop running `git show "REV:PATH" | sed -n 'LINEp'` and comparing the line with
  leading spaces stripped. `check-types.py` cannot pass until the four unwritten sheets above exist.
- **Tastemaker** entries for the new crate (`crate-execute`, `nu-hex-execute`, a `fact-reads-through-lookup` like main's lines
  73-74) are not drafted here; the stale tip line 71 is reported, not fixed.
- **133's `WorkNotice` and work allowance**: dropped because no code exists on main or the tip; whether the owner wants it back
  is not asked.
