# outbound/cache
STATUS: drafted (28 Nov; not reviewed, not agreed)

Revisions: MAIN = f4ae3bd802 (release 0.2.0, live). TIP = 2a207464c (perf branch, build 139). K = ba657e79b (branch
`claude/kept-cache-findings-only`, the third side the brief names: the small kept store, parked; merge base with the
tip `043698327d`). Read only through `git -C /workspaces/idiomatic-public-benchmark show|grep|ls-tree|log`. Every
quote is one line followed by `@rev:path:line`.

## 1. Owns
What a run keeps for the next run: kept records under kept keys, read once and kept once per lint call.

One thing. It does not own what a record says (a file's findings, the rules that may be replayed: inbound/lint), how a
file is identified or whether it changed (its bytes, mode and stamp: outbound/corpus), or which programs are file-local
(domain/programs and domain/plan). On the tip and on K those are all in one place; the draft splits them (section 5).

## 2. On main (f4ae3bd802)
**Where.** `crates/ryo/crates/core/src/outbound/lint_store` (the `lint_store` port of `core`): 55 `.rs` files, 7,889
lines, 3,947 of them outside `_test.rs` files and `fixtures/`; three adapters (`postcard`, `memory`, `discard`) and a
test fake. The data its calls carry are in `ryo_values::domain::stores` (498 lines, 24 public items). Counted with
`git ls-tree -r` and `git show | wc -l` per file.

**The contract, verbatim** (doc comments omitted):
```
pub trait LintStorePort: Send + Sync {  @f4ae3bd802:crates/ryo/crates/core/src/outbound/lint_store/interface.rs:27
fn keeps(&self) -> StoreKeeping;  @f4ae3bd802:crates/ryo/crates/core/src/outbound/lint_store/interface.rs:29
fn stored(&self) -> Result<Arc<dyn ReadStore>, LintStoreError>;  @f4ae3bd802:crates/ryo/crates/core/src/outbound/lint_store/interface.rs:38
pub trait ReadStore: Send + Sync + std::fmt::Debug {  @f4ae3bd802:crates/ryo/crates/core/src/outbound/lint_store/interface.rs:42
fn tables(&self) -> Stored;  @f4ae3bd802:crates/ryo/crates/core/src/outbound/lint_store/interface.rs:44
fn handed(&self, wanted: &BucketsWanted) -> Result<ReadHanded, LintStoreError>;  @f4ae3bd802:crates/ryo/crates/core/src/outbound/lint_store/interface.rs:52
fn read_old(  @f4ae3bd802:crates/ryo/crates/core/src/outbound/lint_store/interface.rs:59
fn begin_old_rows(  @f4ae3bd802:crates/ryo/crates/core/src/outbound/lint_store/interface.rs:71
fn stage_words(  @f4ae3bd802:crates/ryo/crates/core/src/outbound/lint_store/interface.rs:82
fn read_staged(  @f4ae3bd802:crates/ryo/crates/core/src/outbound/lint_store/interface.rs:92
fn keep(&self, stored: Stored, kept: &KeptRows) -> Result<(), LintStoreError>;  @f4ae3bd802:crates/ryo/crates/core/src/outbound/lint_store/interface.rs:106
pub trait OldRowBatch: Send {  @f4ae3bd802:crates/ryo/crates/core/src/outbound/lint_store/interface.rs:13
pub struct CacheDirectory(pub Box<str>);  @f4ae3bd802:crates/ryo/crates/core/src/outbound/lint_store/types.rs:5
pub struct BuildIdentity(pub Box<str>);  @f4ae3bd802:crates/ryo/crates/core/src/outbound/lint_store/types.rs:9
pub enum StoreKeeping {  @f4ae3bd802:crates/ryo/crates/core/src/outbound/lint_store/types.rs:13
pub struct Store {  @f4ae3bd802:crates/ryo/crates/core/src/outbound/lint_store/types.rs:22
pub enum LintStoreError {  @f4ae3bd802:crates/ryo/crates/core/src/outbound/lint_store/errors.rs:5
NothingRead,  @f4ae3bd802:crates/ryo/crates/core/src/outbound/lint_store/errors.rs:24
pub fn postcard(  @f4ae3bd802:crates/ryo/crates/core/src/outbound/lint_store/factory.rs:8
pub fn memory() -> impl crate::outbound::lint_store::interface::LintStorePort {  @f4ae3bd802:crates/ryo/crates/core/src/outbound/lint_store/factory.rs:16
pub fn discard() -> impl crate::outbound::lint_store::interface::LintStorePort {  @f4ae3bd802:crates/ryo/crates/core/src/outbound/lint_store/factory.rs:22
```
**Public items: 21** (3 traits, 10 trait methods, 4 types, 1 error, 3 factory functions), 26 with the 5 enum variants.
What it keeps: per run, the old engine's tables (`Stored`), the words each file handed the cross-file steps (`handed`,
read back by bucket), and staged fresh words; `keep` replaces the whole store atomically against the snapshot it read.

**Implementers.** `LintStorePostcardAdapter`, `LintStoreMemoryAdapter`, `LintStoreDiscardAdapter`, and the fake in
`fixtures/fake.rs`.

**Callers** (non-test files, by `git grep -l` on the port's names): core 12 (`application/file_analysis` 4,
`application/persisting` 1, `application/storing` 3, `inbound/use_cases/lint` 4), core benches 2, sdk 4, cli 4. The edges
choose the adapter:
```
Some(store) => Arc::new(lint_store::factory::postcard(store)),  @f4ae3bd802:crates/ryo/crates/sdk/src/resources/linter/lint/composition.rs:116
None => Arc::new(lint_store::factory::discard()),  @f4ae3bd802:crates/ryo/crates/sdk/src/resources/linter/lint/composition.rs:117
let store: Arc<dyn LintStorePort> = Arc::new(lint_store::factory::postcard(kept_in));  @f4ae3bd802:crates/ryo/cli/src/resources/query/run/composition.rs:63
store: Arc<dyn LintStorePort>,  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/lint/use_case.rs:125
Listed::Stored(self.store.stored().map(Some).map_err(StoringError::from))  @f4ae3bd802:crates/ryo/crates/core/src/application/storing/service.rs:129
```
and the SDK's public switch, which every host sets:
```
pub enum Cache {  @f4ae3bd802:crates/ryo/crates/sdk/src/resources/linter/lint/types.rs:94
```
(`None`, `At(PathBuf)`). The build identity the edge hands in is the package name and version
(`cli/src/resources.rs:85-87`).

**Proof.** 14 `#[test]` over 63 case rows. The port's contract is one case table every adapter runs:
```
pub fn check(fresh: &dyn Fn() -> Box<dyn LintStorePort>) {  @f4ae3bd802:crates/ryo/crates/core/src/outbound/lint_store/fixtures/contract.rs:293
```
with ten cases:
```
name: CaseName("a_store_nothing_was_kept_in_is_empty"),  @f4ae3bd802:crates/ryo/crates/core/src/outbound/lint_store/fixtures/contract.rs:56
name: CaseName("what_was_kept_is_what_is_read"),  @f4ae3bd802:crates/ryo/crates/core/src/outbound/lint_store/fixtures/contract.rs:62
name: CaseName("the_last_keep_stands"),  @f4ae3bd802:crates/ryo/crates/core/src/outbound/lint_store/fixtures/contract.rs:68
name: CaseName("a_file_no_language_could_parse_reads_back_with_what_was_said"),  @f4ae3bd802:crates/ryo/crates/core/src/outbound/lint_store/fixtures/contract.rs:74
"the_words_of_a_bucket_kept_as_they_were_stay_and_only_the_buckets_asked_are_read",  @f4ae3bd802:crates/ryo/crates/core/src/outbound/lint_store/fixtures/contract.rs:81
name: CaseName("the_words_of_a_file_used_again_move_to_its_number_now"),  @f4ae3bd802:crates/ryo/crates/core/src/outbound/lint_store/fixtures/contract.rs:88
name: CaseName("a_keep_that_must_copy_words_from_a_store_that_holds_none_is_refused"),  @f4ae3bd802:crates/ryo/crates/core/src/outbound/lint_store/fixtures/contract.rs:94
name: CaseName("a_run_keeps_against_the_store_it_read_whatever_another_run_kept_after"),  @f4ae3bd802:crates/ryo/crates/core/src/outbound/lint_store/fixtures/contract.rs:100
name: CaseName("words_staged_during_a_run_read_back_as_they_were_given"),  @f4ae3bd802:crates/ryo/crates/core/src/outbound/lint_store/fixtures/contract.rs:106
name: CaseName("an_old_row_read_in_a_batch_is_the_row_read_alone"),  @f4ae3bd802:crates/ryo/crates/core/src/outbound/lint_store/fixtures/contract.rs:112
```
The other 11 tests are the postcard adapter's own (`adapter_test.rs` and nine `schemas_test.rs`). End to end through the
SDK: three command-line replay tests use `Cache::At`
(`every_inventoried_file_is_a_row_whatever_it_holds`, `public_sdk_cache_tracks_given_source_edits_deletion_and_filters`,
`a_kept_walk_finds_what_a_whole_walk_finds_after_each_edit`, `cli/tests/replay/filesystem.rs:20, 54, 150`), and
Tastemaker's check has one case on the store:
```
name: CaseName("the_lint_store_holds_every_file_read_and_one_edit_keeps_the_rest"),  @f4ae3bd802:crates/tastemaker/cli/src/resources/workspace/check/command_test.rs:54
```
No acceptance scenario names the store (core's `lint/acceptance.feature` has two outlines, neither about keeping).
A divan bench of the three adapters:
```
use ryo_core::outbound::lint_store::factory::{discard, memory, postcard};  @f4ae3bd802:crates/ryo/crates/core/benches/performance/lint_store/bench.rs:11
```

**Tastemaker entries.** No `crate-` or `nu-hex-` entry (the port is a module of `core`). The SDK spec lists the public
switch, and two shared rules of the Rust spec apply to any cache:
```
{ path = "ryo::linter::Cache" },  @f4ae3bd802:crates/ryo/tastemaker.toml:137
{ rule = "idiomatic.arch/one-cache-one-engine@0.1.0", include = ["**/*.rs"],  @f4ae3bd802:tastemaker/specs/idiomatic-rust/0.1.17/spec.toml:97
{ rule = "idiomatic.arch/digest-not-stamp-alone@0.1.0", include = ["**/*.rs"],  @f4ae3bd802:tastemaker/specs/idiomatic-rust/0.1.17/spec.toml:108
```

## 3. On the perf branch (2a207464c)
**Where.** No port. The proof of concept's own store: `executor/src/check_session/replay/stored.rs` (1,665 lines),
`stored/request_answers.rs` (653), `stored/shards.rs` (425), `check_session/stored_block.rs` (206): 2,949 lines
including 7 inline tests. Core's port was deleted on 27 Nov (`f2d1845a4f`: "the lint store port is deleted (Cache::At
opens the executor's own store, check_session/replay/stored; nothing opened core's)"; 72 files, 8,507 lines removed).

**What is public** (160 `pub` declarations in the four files, about 200 public fields). The centre:
```
pub type Key = [u8; 32];  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/stored.rs:16
pub fn key(value: impl Hash) -> Key {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/stored.rs:35
pub struct Store {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/stored.rs:367
pub fn open(root: &Path, readers: &[impl session::source::Source]) -> Res<Self> {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/stored.rs:442
pub fn directory(dir: PathBuf, readers: &[impl session::source::Source]) -> Res<Self> {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/stored.rs:445
pub fn publish(&mut self) -> Res<usize> {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/stored.rs:541
```
`Store` has **53** public fields (`stored.rs:368-420`; the plan's "about 55") and **33** public methods
(`stored.rs:442-1472`; the plan's "35"). Eleven take the driver's own types and cannot be separated from it as written:
```
pub fn save(&mut self, id: Key, idx: usize, r: &super::Receipt<'_>) -> Option<Key> {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/stored.rs:685
pub fn decline(&mut self, id: Key, reason: &crate::check_session::refusal::Refusal) {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/stored.rs:717
pub fn prepare(&mut self, b: &Batch) {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/stored.rs:780
pub fn probe(&mut self, b: &Batch) -> (Vec<usize>, Vec<PriorClosed>) {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/stored.rs:865
pub fn reuse_closed(&mut self, b: &mut Batch, candidates: Vec<PriorClosed>) {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/stored.rs:919
pub fn admit(  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/stored.rs:982
pub fn local(  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/stored.rs:1238
pub fn closed(  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/stored.rs:1371
pub fn reuse_context(&mut self, environment: Option<&project::Environment>, proof: Option<Proof>) -> bool {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/stored.rs:1409
pub fn remember_entries(&mut self, environment: &project::Environment) {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/stored.rs:1440
pub fn population_proof(&mut self, p: &Population, root: &str) -> Option<Proof> {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/stored.rs:1472
```
It keeps the whole session: per file version and binding a `Local` record (report, frontier block numbers, unavailable
relations), the frontier column blocks of the cross-file operators, per request a packed answer `Packet` and a `Record`
of proofs, file stamps, groups and notes. A kept answer is valid when the build key, the file's stamp and blake3
content hash, the file version key, the rule text key and the binding key match (`stored.rs:449-466, 786-821`;
`request_answers.rs:202-293`). The frontier blocks are restored into the driver's `World`:
```
pub fn encode(block: &Block, world: &World) -> Vec<u8> {  @2a207464c:crates/ryo/crates/executor/src/check_session/stored_block.rs:41
pub fn restore(bytes: BlockBytes, world: &mut World) -> Option<Rc<Block>> {  @2a207464c:crates/ryo/crates/executor/src/check_session/stored_block.rs:139
pub struct Stamp {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/stored/request_answers.rs:29
```
The build key is the executable's build id or the blake3 of the whole executable, combined with each reader's name and
identity (`stored.rs:446-458`), worked out inside the store.

**Callers.** executor 2 files (`replay/batch.rs`, where the driver calls `prepare`, `probe`, `local`, `reuse_closed`,
`admit`; `replay.rs`, the command line's replay), sdk 2 files (`session/entry.rs`, `session/call_cache.rs`):
```
store.admit(self, &mut state.world, &mut state.answers, &mut state.contributions, &probe);  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/batch.rs:527
let mut store=root.map(|p|stored::Store::open(p, readers).expect("open request store"));  @2a207464c:crates/ryo/crates/executor/src/check_session/replay.rs:152
sdk::Cache::At(path) => Some(path.as_path()),  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:48
session.publish();  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:58
self.store=directory.map(|p|replay::stored::Store::directory(p.to_owned(), &self.languages))  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:297
let digest = replay::stored::key((&query, &parameters));  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:574
pub(super) struct CallCache {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/call_cache.rs:117
pub(super) fn query(&mut self, query: &str) -> (Rc<str>, stored::Key) {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/call_cache.rs:216
```
The public switch is unchanged from main:
```
pub enum Cache {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/types.rs:77
```
**Measured** (bead ryo-fmx3.1.11.14): the store is 53 MB on fastapi (frontier blocks 48.9 MB of it), 290 MB on Django,
518 MB on dagster; with `Cache::At` the Tastemaker check costs +27% engine CPU, +60% wall cold and a 291 MB store
against the 2.48 MB bar; one edit on Django 1,542 ms against the 177 ms bar.

**Proof.** 7 inline tests in the store files:
```
fn mapped_replay_copies_untouched_rows_into_a_changed_file_record() {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/stored.rs:1519
fn contextual_proofs_recheck_absence_and_reject_unknown_entries() {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/stored.rs:1562
fn answers_replay_after_publish_with_each_requests_inputs() {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/stored.rs:1598
fn interleaved_requests_keep_their_own_effective_inputs() {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/stored/request_answers.rs:533
fn prior_leaf_proves_only_one_physical_change_in_the_same_scope() {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/stored/request_answers.rs:571
fn both_timestamps_must_precede_the_snapshot_tick() {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/stored/request_answers.rs:606
fn restored_rows_preserve_nulls_negative_one_and_multiplicity() {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/stored/request_answers.rs:624
```
and 7 SDK tests through `Linter::lint` with `Cache::At`:
```
fn cold_warm_and_edited_runs_answer_as_a_cold_run_over_the_same_files() {  @2a207464c:crates/ryo/crates/sdk/tests/lint_cache.rs:93
fn a_lint_leaves_what_the_cache_keeps_on_disk_before_it_returns() {  @2a207464c:crates/ryo/crates/sdk/tests/lint_cache.rs:109
fn a_file_that_cannot_be_read_is_unread_cold_and_warm_over_a_kept_cache() {  @2a207464c:crates/ryo/crates/sdk/tests/lint_cache.rs:124
fn a_deleted_file_has_no_finding_in_the_next_run() {  @2a207464c:crates/ryo/crates/sdk/tests/lint_cache.rs:145
fn a_refused_run_leaves_the_store_as_it_was() {  @2a207464c:crates/ryo/crates/sdk/tests/lint_cache.rs:156
fn a_damaged_store_answers_as_if_there_were_none() {  @2a207464c:crates/ryo/crates/sdk/tests/lint_cache.rs:169
fn a_rule_that_needs_accepted_syntax_answers_alike_cold_and_warm() {  @2a207464c:crates/ryo/crates/sdk/tests/lint_cache.rs:212
```
The three command-line replay tests of main are still there (`filesystem.rs:19, 52, 143`). Tastemaker's
`the_lint_store_holds_every_file_read...` case is not (`git grep` at the tip: no hit).

**Tastemaker entries.** The same three: `{ path = "ryo::linter::Cache" },` (`crates/ryo/tastemaker.toml:127`) and the
two shared rules (`tastemaker/specs/idiomatic-rust/0.1.16/spec.toml:95, 106`). The tip's docs still describe core's
three adapters (`crates/ryo/docs/README.md:160`), which no longer exist.

**The third side, K (ba657e79b): the small store.** Two commits on `043698327d`: `e4cb337bc1` ("the kept cache is the
findings of each file (what the old image kept), the whole-session store only under bench-hooks") and `ba657e79b9`
("hashes each rule's query text and modules, not their debug text (warm 0.6 s to 0.02 s on fastapi)"). One file of the
SDK, 762 lines, one inline test:
```
pub(super) fn execute(  @ba657e79b:crates/ryo/crates/sdk/src/resources/linter/lint/kept.rs:95
const SCOPES: usize = 8;  @ba657e79b:crates/ryo/crates/sdk/src/resources/linter/lint/kept.rs:29
fn plan(request: &LintRequest, engine: &LinterLintEngine, directory: &Path, build: &Digest, store: &Store) -> Option<Plan> {  @ba657e79b:crates/ryo/crates/sdk/src/resources/linter/lint/kept.rs:328
let scope: Key = hasher.finalize().as_bytes()[..16].try_into().ok()?;  @ba657e79b:crates/ryo/crates/sdk/src/resources/linter/lint/kept.rs:374
hasher.update(&metadata.permissions().mode().to_le_bytes());  @ba657e79b:crates/ryo/crates/sdk/src/resources/linter/lint/kept.rs:392
fn dependencies_current(root: &Path, dependencies: &[ProjectDependency]) -> bool {  @ba657e79b:crates/ryo/crates/sdk/src/resources/linter/lint/kept.rs:404
fn build_identity() -> Digest {  @ba657e79b:crates/ryo/crates/sdk/src/resources/linter/lint/kept.rs:463
hasher.update(&metadata.len().to_le_bytes());  @ba657e79b:crates/ryo/crates/sdk/src/resources/linter/lint/kept.rs:467
fn load(directory: &Path, build: &Digest) -> Option<Store> {  @ba657e79b:crates/ryo/crates/sdk/src/resources/linter/lint/kept.rs:640
fn publish(directory: &Path, build: &Digest, store: &Store) -> std::io::Result<()> {  @ba657e79b:crates/ryo/crates/sdk/src/resources/linter/lint/kept.rs:684
let not_file_local = super::session::take_not_file_local();  @ba657e79b:crates/ryo/crates/sdk/src/resources/linter/lint/kept.rs:138
if !rest.is_empty() {  @ba657e79b:crates/ryo/crates/sdk/src/resources/linter/lint/kept.rs:122
Cache::At(directory) => super::kept::execute(directory, engine, requests, &mut |requests| {  @ba657e79b:crates/ryo/crates/sdk/src/resources/linter/lint/entry.rs:63
Session(PathBuf),  @ba657e79b:crates/ryo/crates/sdk/src/resources/linter/lint/types.rs:86
```
How it works: a request over a listed corpus gets a scope key (blake3 of the build, each language's name and build,
the request's other fields, each rule's id, query text and modules, the sorted file names, and the names and kinds of
the tree below the root); each file a key (scope, name, mode, content digest). The store keeps per scope the rules that
are not file-local, the project files the run observed (with a fingerprint, or "missing"), and per file key the file's
findings, unknowns, unread reason and skipped rules. A run replays the file-local rules' records, hands the executor
the file-local rules over the files with no record, and runs the other rules over every file every time (`kept.rs:122`).
One file `findings.bin` (checksum, magic, build, scopes), at most 8 scopes, written by temp file and rename once per
lint call. The whole-session store remains as `Cache::Session` under `bench-hooks`. Tests: K's `lint_cache.rs` has 10,
four of them new:
```
fn the_store_holds_the_findings_and_no_session() {  @ba657e79b:crates/ryo/crates/sdk/tests/lint_cache.rs:214
fn a_finding_that_depends_on_another_files_content_is_never_replayed() {  @ba657e79b:crates/ryo/crates/sdk/tests/lint_cache.rs:228
fn a_request_of_both_kinds_answers_as_a_cold_lint_after_an_edit() {  @ba657e79b:crates/ryo/crates/sdk/tests/lint_cache.rs:246
fn an_unchanged_rerun_evaluates_no_file_and_one_edit_evaluates_one() {  @ba657e79b:crates/ryo/crates/sdk/tests/lint_cache.rs:267
fn a_record_is_current_only_while_the_files_the_project_read_observed_are_as_they_were() {  @ba657e79b:crates/ryo/crates/sdk/src/resources/linter/lint/kept.rs:736
```
(K branched before `22b59bd30e`, so it lacks the tip's unread-file test.) Measured (bead ryo-fmx3.1.11.14, notes 20:39
UTC 26 Nov; store, warm, one edit): fastapi 840 KB, 23 ms, 301 ms; Django 2.6 MB, 87 ms, 475 ms; pandas 2.7 MB, 51 ms,
389 ms; dagster 3.7 MB, 111 ms, 397 ms; TypeScript and Rust 11 to 117 KB, 5 to 20 ms, 40 to 57 ms; cold equal to plain
cold. Not file-local: 7 of 413 Ruff programs, 0 of 96 Oxlint, 0 of 70 Clippy.

## 4. What changed between them
- **Removed:** core's port, its three adapters, its 14 tests and its bench (`f2d1845a4f`, 27 Nov). Nothing called it any
  more: since `f2869d1f74` (26 Nov, the executor moved into the workspace) the SDK's `Cache::At` opened the executor's
  own store.
- **Added:** the executor's whole-session store, born as bench code on 24 Nov (`c38f9a272a`, "Retain typed file
  contributions and consumed closed answers across sessions") and grown by 57 commits; `964de529e7` (24 Nov) moved
  Tastemaker's whole-session retained store into it (1,043 lines added to `stored.rs`): the bead's suspect for the 100x
  store; `b640148bed` (25 Nov) added per-rule hashing and the file record bitset (perf row 10).
- **Changed:** what is kept. Main kept the old engine's tables and handed words; the tip keeps the whole session
  including frontier columns; K keeps findings per file. Main's edge passed the build as name and version; the tip
  reads the executable; K hashes the executable's size and modification time.
- **Lost that should not have been:** the port and its ten contract cases (the tip's store has no contract, only
  seven tests of its internals); the adapter bench; Tastemaker's store case. **Lost on the way, and right to lose:** the
  old image's per-file store (`crates/ryo/crates/engine/benches/vec/retained.rs`, 770 lines at `ab8b13b6cc`, deleted by
  `efe8dd97de` on 25 Nov), whose figures are the warm and one-edit bars; it replayed every program per file with no
  file-local guard (no such check in the file; the bead's 19:41 note). K restores it with the guard.
- **Stale on the tip:** the retention bench reads `.ryo_cache/findings.postcard`, which nothing writes (the tip writes
  `session.postcard`, K writes `findings.bin`):
```
store = root / ".ryo_cache" / "findings.postcard"  @2a207464c:crates/ryo/benchmarks/retention/edit_matrix.py:121
let previous = fs::read(dir.join("session.postcard")).unwrap_or_default();  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/stored.rs:463
```

## 5. The drafted public interface
The listing is `api.txt` beside this sheet: 31 public items, 35 lines. This part explains each changed or new line.

**The claim it starts from** (PLAN.md Appendix A 3.13): `CachePort { keeps, answers(&AnswerKey) -> Option<KeptAnswers>,
keep(AnswerKey, &KeptAnswers) }` with `AnswerKey { Request(Key), File(Key) }`, keyed "from the corpus's file identity,
the rule text and the build". Checked against the code: (1) the small store's code exists (K), contrary to the
appendix's "NOT FOUND"; (2) K reads the whole store once and writes it once per lint call, and so did main
(`stored()` then `keep`); a per-key `keep` would write per file or need a hidden flush. So the draft keeps main's
two-step shape (read a snapshot, keep a snapshot) and K's content; (3) K needs no request-level key: a request is a
scope, a file is a key under it; (4) `KeptAnswers` would have to name lint's `Finding`, which outbound may not import
(the directions: outbound imports domain only). So records are bytes and the keys are the cache's.

### 5a. The port
```
pub trait CachePort: Send + Sync {  @new
fn keeps(&self) -> CacheKeeping;  @new
fn read(&self) -> Result<Kept, CacheError>;  @new
fn keep(&self, kept: &Kept) -> Result<(), CacheError>;  @new
```
- `keeps` is main's call with the type renamed (`StoreKeeping` -> `CacheKeeping`: `store` is outbound/store's word).
  Lint asks it first and, on `Discards`, computes no key and reads nothing: a run without a cache pays nothing for one
  (perf row 10's lesson, section 7).
- `read` is main's `stored()` as a value instead of a second trait object (`Arc<dyn ReadStore>`), and K's `load`. An
  empty store, another build's, or a damaged one (bad checksum or magic) reads as an empty `Kept`, as K and the tip do
  (tip test `a_damaged_store_answers_as_if_there_were_none`); `Unreadable` is kept for a directory that exists and
  cannot be read.
- `keep` is main's `ReadStore::keep` and K's `publish`: the whole snapshot, atomic (a reader sees the old or the new),
  on disk before it returns (the tip's "a host may end its process without running destructors", `entry.rs:57-58`).
  Main's `NothingRead` goes: a keep writes the snapshot it is given, which already holds the scopes it read.

### 5b. The snapshot
`Kept` is opaque (K's private `Store { scopes }`); `Kept::scope(&KeptKey)` and `Kept::put(KeptKey, KeptScope)` are the
two things K does with it (`kept.rs:376-381` find, `:179-180` retain and push). The adapter drops the least recent
scopes beyond its limit when it keeps (K: 8, `kept.rs:29, 185-186`), so the limit is not in a signature.
`KeptScope { whole, files }`: `files` is K's `BTreeMap<Key, Answer>` with the answer as a record; `whole` is the
scope's own record: what K keeps per scope (`not_file_local`, `dependencies`) plus what K does not keep, the answers of
the rules that read across files as of the last run over these exact file keys (decision 3).

### 5c. Keys and records
`KeyWriter { new, field, key }` is the one way a `KeptKey` is made: K's `field` (length then bytes, `kept.rs:422-425`)
over blake3, cut to 16 bytes (`kept.rs:374`). Lint feeds it the scope's fields and, for a file, the scope key, the name,
the mode and the content digest the corpus hands it. Making the key here is what the programs sheet means by "the
digest is the cache's" (its NO LONGER PUBLIC lines for `Modules.digest`, `PrepareRequest.digests`, `rule_digest`).
`RecordWriter { new, number, bytes, record }` and `RecordReader { new, number, bytes }` are K's private `Writer` and
`Reader` (`kept.rs:479-565`): the one form a record is written in. What lint writes with them (a finding's rule, slot,
place and columns: K's `write_answer`/`read_answer`, `kept.rs:568-638`) is lint's, because the finding is lint's type.
K's `text`, `slot`, `unreadable` and `rule` helpers are lint's; text is `bytes` read as UTF-8.

### 5d. How it is built
`file(CacheLocation)`, `memory()`, `discard()` are main's three factories (`postcard` renamed `file`: the form is the
adapter's). `CacheLocation { directory, build }` is main's `Store` renamed. The SDK edge maps its unchanged public
switch: `Cache::None` -> `discard()`, `Cache::At(dir)` -> `file(..)`; the edge also adds the directory to the corpus's
skipped directories (main `cli/src/resources.rs:59-68`; the tip's `walk.rs:32`). Lint holds the port as
`Arc<dyn CachePort>` (Appendix A 3.16): it is crossed once per lint call, so `dyn` costs nothing measurable here.

### 5e. Where each part of K and the tip goes
| Code today | Goes to |
|---|---|
| K `load`, `publish`, `Writer`, `Reader`, `field`, the file form, `SCOPES` (about 220 lines) | here |
| K `plan`, `remember`, `replay`, `execute`'s job split (about 400 lines) | inbound/lint |
| K `dependencies_current`, `tree` (file reads), the stamp and content digest | outbound/corpus, called by lint |
| K `build_identity`; the tip's build-id read (`stored.rs:446-458`) | the edge (decision 2) |
| K `take_not_file_local` (the session reports it) | domain/programs (`Programs::cut`: a rule with no cut is file-local) |
| the tip's four store files (2,949 lines) | not carried (decision 1) |
| the SDK's `CallCache` key digests | `KeyWriter`; its whole-call memo is inbound/lint's to keep or drop |

### 5f. Checked against the five points and the owner's rules
One port, one path: there is one cache (no `Cache::Session` second store). No layout word: no table, column, row or
cell type crosses; records are bytes. Nothing is looked up while a rule runs: lint reads the snapshot before any
program is prepared. Crossed once per lint call (read, keep); scope and file look-ups are reads of an owned value.

### 5g. Public items, the three sides and the draft
| Side | Public items for the concept | Stop being public / go |
|---|---|---|
| main `f4ae3bd802`, core `lint_store` | **21** (26 with variants), plus 24 in `values::domain::stores` | 10 gone (`ReadStore` and its 7 methods, `OldRowBatch` and its `read`), `stored` and `postcard` replaced; the rest renamed in |
| tip `2a207464c`, the four store files | **160** declarations, no trait, plus the SDK's `Cache` (1 + 2 variants) | all 160 |
| K `ba657e79b`, `kept.rs` | **1** (`files_evaluated`, bench-hooks) plus `Cache::Session` | the variant goes (decision 1) |
| drafted | **31** (35 lines) | — |

## 6. Proof it brings
**Carried as they are, restated against `CachePort`** (one case table, `fixtures/contract.rs`, run by `file`, `memory`
and `discard`):
| Proof | Source | Fate |
|---|---|---|
| `a_store_nothing_was_kept_in_is_empty` | main `contract.rs:56` | carried |
| `what_was_kept_is_what_is_read` | main `contract.rs:62` | carried (a `Kept` of two scopes round-trips) |
| `the_last_keep_stands` | main `contract.rs:68` | carried |
| `a_run_keeps_against_the_store_it_read_whatever_another_run_kept_after` | main `contract.rs:100` | carried: the last whole keep stands |
| `a_file_no_language_could_parse_reads_back_with_what_was_said` | main `contract.rs:74` | restated in lint (what a record says is lint's) |
| the bucket, word, staged and old-row cases | main `contract.rs:81, 88, 94, 106, 112` | dropped with what they test (no words or tables kept) |
| `a_damaged_store_answers_as_if_there_were_none` | tip `lint_cache.rs:169` | restated here as a port case (a truncated or bad-checksum file reads empty); the SDK test stays |
| the 11 postcard adapter tests | main `adapter_test.rs`, `*/schemas_test.rs` | dropped: the postcard tables go |
| the tip's 7 store tests | tip `stored.rs:1519-1598`, `request_answers.rs:533-624` | dropped with the whole-session store (decision 1); `both_timestamps_must_precede_the_snapshot_tick` is corpus's if it keeps stamps |
| K's `kept.rs:736` dependency test | K | lint's (with corpus reads) |
| `lint_cache.rs`, 7 at the tip, 10 at K (union: 11) | tip, K | stay as the SDK's end-to-end proof of `Cache::At`; K's `the_store_holds_the_findings_and_no_session` is the check that no plan or column is kept (decision 4) |
| the 3 command-line replay tests | main and tip `filesystem.rs` | stay at the edge |
| Tastemaker `the_lint_store_holds_every_file_read_and_one_edit_keeps_the_rest` | main only | restore, against the findings store |

**Missing, to write** (new port cases): another build's store reads empty; a keep beyond the adapter's limit drops the
least recent scope; `discard` reads empty after a keep; two field lists differing only at a boundary give two keys
(`["ab","c"]` against `["a","bc"]`); a record reads back field by field and a truncated record reads `None`; a keep is
atomic (a reader opened before it sees the old snapshot). Planted bugs and the case that catches each: drop the length
prefix -> the boundary case; skip the checksum -> the damaged case; keep without fsync and rename -> the atomic case;
drop the build -> the other-build case; evict the most recent -> the limit case. Not observable at the seam: that
`keep` is on disk before the process exits (argued by fsync; the SDK test `a_lint_leaves_what_the_cache_keeps_on_disk_
before_it_returns` holds it end to end).

## 7. Perf gains it carries
From `perf-gains.tsv`, every row whose crate column names cache:

| Row | State | Measured | What in the draft keeps it |
|---|---|---|---|
| 2 (bench composition in the product; the kept store holds the whole session) | live; defect | Django store 690 KB -> 290 MB, fastapi 53 MB, dagster 518 MB | The composition half is the SDK edge's (row 52) and untouched. The store half is the defect, and the draft removes it: no call takes a plan, a table or a frontier block, so the store holds findings per file and per scope (K: 0.84 to 3.7 MB on Python). Depends on: records only, decided by lint; the edge picks the mode (`Cache::None`/`At`) |
| 10 (per-request hashing and a whole-call cache, `b640148be`) | live, unjustified | +600 M instructions; 205 of 46,831 calls reused | Keys are made only when `keeps()` is `Keeps`; one `KeyWriter` field per rule text (K hashes the text, not its debug form: warm 0.6 -> 0.02 s on fastapi, `ba657e79b9`). The whole-call memo is not kept between runs, so it is not here; inbound/lint keeps or drops it on its own measurement |
| 14 (warm store: findings by file version, full hit before preparation) | live | unchanged rerun Django 62.0 -> 32.1 ms, dagster 104.5 -> 49.6 | `read` once, then scope and file look-ups before any program is prepared. A full hit prepares nothing only if the cross-file rules' answers are kept too: K runs them every rerun (Django 87 ms); `KeptScope.whole` holds them, replayed when every file key hits and the observed project files are current (decision 3). **Not measured** |
| 48 (fused engine: compiled program cached, kept copies) | lost | compile 62.5% of a one-file run | nothing here: programs are never kept between runs (programs sheet decision 1 drops main's plan cache) |
| 53 (findings-only kept cache, K) | parked: ba657e79b | fastapi 840 KB, 23 ms, 301 ms; Django 2.6 MB, 87 ms, 475 ms | this draft is K behind a port: the file key is scope, name, mode and content digest; only file-local programs are replayed (lint reads it from `Programs`); 8 scopes |

**The gains this crate must make room for:** the small kept store (row 53) is the draft. The shared prepared plan is not
this crate's. **What the draft does not reach:** the one-edit bars (Django 177 ms; K 475 ms). The remaining cost is the
7 cross-file Ruff programs re-run over every file; keeping each file's cross-file contribution would be needed to cut it,
and that is the frontier columns the project sheet's decision 3 keeps out of the cache. The bar stands; reaching it is
a stage 3 item for the cross-file phase, not a change to this port (decision 5).

**History, against the draft (aim 3).** Would have landed inside this crate alone: `fc1631d5b1` (fewer shards for small
tables), `cdde3779e1` (digest a short key at once), `1654f52371` (append only changed records), `9d7c6fc751` (one
bounded append table): adapter and key internals. Would still have crossed: `ba657e79b9` (which fields go in the key:
lint), `b640148bed` (request read, hashing and the store: SDK, lint, here), `964de529e7` (what the driver hands the
store), `e4cb337bc1` (lint's replay plan with the executor's file-local report), `897419e522` and `a198051797` (frontier
blocks: no home under the draft).

## 8. Benchmarks that cover it
| Benchmark | What it calls in this concept today | After the draft |
|---|---|---|
| the lint images `vec`, `vec_ts`, `vec_rust` (`executor/benches/lint_image`) | `sdk::Cache::At(request.root.join(".ryo_cache"))` when retaining, else `None` (`main.rs:121-125`) | unchanged call; `At` now builds `ryo_cache::file`; measures the warm, one-edit and store-size cells against the findings store |
| the `tm` image and Tastemaker's check | `ryo::linter::Cache::At(...join(RYO_STORE))`, `None` on a dry run (`tastemaker/.../adapters/ryo/schemas.rs:533-535`); `TM_CHECK_SESSION=retained` uses the whole-session store (bead) | unchanged for `At`; the `retained` mode has nothing to open if decision 1 is taken |
| the benchmark crate's local engine adapter | `WorkerState::Retained(path)` -> `Cache::At(path.clone())` (`benchmark/.../local/adapter.rs:190-192`) | unchanged |
| main's adapter bench `core/benches/performance/lint_store` | the three factories | **restore**, over `file`, `memory`, `discard`: `read` and `keep` of a store of N scopes and M file records |
| the retention matrix `crates/ryo/benchmarks/retention/edit_matrix.py` | reads `.ryo_cache/findings.postcard` (stale on the tip and on K) | point it at the file `file()` writes; until then it measures nothing of the cache |
| Tastemaker `benchmarks/project/store_matrix.py` | a "closed-contribution" row of the kept store (project sheet) | changes meaning: no carried rows are kept |
| Ruff, Oxlint, Clippy parity suites | `Cache::None` | unchanged |
```
sdk::Cache::At(request.root.join(".ryo_cache"))  @2a207464c:crates/ryo/crates/executor/benches/lint_image/main.rs:122
Cache::At(path.clone())  @2a207464c:crates/ryo/crates/benchmark/src/outbound/engine/adapters/local/adapter.rs:192
ryo::linter::Cache::At(std::path::PathBuf::from(&*store.directory.0).join(RYO_STORE))  @2a207464c:crates/tastemaker/crates/core/src/outbound/analysis/adapters/ryo/schemas.rs:533
```

## 9. Checks
IMPORTS: none
CROSSED: per request

- Imports: no Ryo crate (the draft names no domain type: keys and records are the cache's own). Outside: `blake3` (K
  and the tip both use it). `CacheLocation.directory` is a `Box<str>` as on main; reading and writing it is the only I/O.
- Layout words: none. No `Vec<Vec<u64>>`, table, column, row or plan type in any signature.
- Crossed: `read` and `keep` once per lint call; `keeps` once per lint call; `Kept::scope` once per request and a map
  look-up per file, both on an owned value (no port call).
- Generic or `dyn`: lint may hold `Arc<dyn CachePort>`; not a hot path.
- Tastemaker: new `crate-cache` and `nu-hex-cache` entries; the two shared rules `one-cache-one-engine` and
  `digest-not-stamp-alone` apply as they are; the SDK spec's `ryo::linter::Cache` re-export stays.

## 10. Decisions for the owner
1. **The whole-session store: drop it, not keep it under `bench-hooks`.** Example: the tip's `Store` (53 public fields,
   33 methods, 11 over the driver's `Batch`/`World`, `stored.rs:367`) and K's `Session(PathBuf)` (`types.rs:86`). Why: it
   cannot sit behind a port without the driver's types, it is a second path (five points: one path), and it is the
   cause of row 2's 100x store. What we get: 2,949 lines go; one cache; benchmarks that used `retained` lose that mode.
   What we lose: its warm figure (Django 32 ms against K's 87 ms) unless decision 3 recovers it. **My pick: drop.**
2. **Who makes the build identity, and from what.** Example: main's edge passes `CARGO_PKG_NAME` and version
   (`cli/src/resources.rs:85-87`: a rebuild without a version bump reuses old records); the tip reads the executable's
   build id or hashes the whole executable inside the store (`stored.rs:446-455`); K hashes the version, the
   executable's size and modification time (`kept.rs:463-475`: a stamp, which `digest-not-stamp-alone` forbids in
   spirit though its query would not see it). Why: a wrong identity replays another build's findings. What we get: one
   definition, made once. **My pick: the edge makes it once, the tip's way (build id, else blake3 of the executable),
   and hands it to `file`.**
3. **An unchanged rerun replays the cross-file rules too.** Example: K runs the non-file-local rules on every rerun
   (`kept.rs:122`), so Django's warm run is 87 ms against the 32 ms bar. If every file key of a scope hits and the
   observed project files are current, those rules' inputs are unchanged, so their last answers can be replayed from
   `KeptScope.whole`. Why: it is row 14's "a full hit prepares nothing". What we get: no port change (one record more per
   scope); unmeasured. **My pick: yes, with a K-style test that a rerun after an edit of any file runs them again.**
4. **Records are bytes the cache frames; their contents are lint's.** Example: K's `write_answer` writes a `Finding`
   (`kept.rs:568`), the SDK's type, which outbound may not import. Alternatives: move a finding's vocabulary down to a
   domain crate so the cache can type its records; or make the port generic over a record type lint supplies. Why: typed
   records would hold "no plan or column is kept" by the compiler; bytes hold it by K's test
   `the_store_holds_the_findings_and_no_session` and by the store size in the bar table. What we get with bytes: no Ryo
   import and a port that does not change when lint's report does. **My pick: bytes, with that test carried.**
5. **Confirm, from the cache side, that no file's cross-file contribution is kept** (the project sheet's decision 3).
   Example: the tip's frontier blocks are 48.9 of fastapi's 53 MB (bead) and they are what made one edit cheap to
   replay. Why: keeping them breaks the 2.48 MB store bar; not keeping them leaves one edit at K's 475 ms on Django
   against the 177 ms bar, whose figure came from a store that replayed cross-file findings with no guard. What we get:
   the size bar holds and the port stays small; the one-edit cell becomes the cross-file phase's stage 3 work. **My
   pick: confirm.**

## 11. Not checked
- No figure here was re-measured; all come from bead ryo-fmx3.1.11.14 and `engine/STATE.md:8610-8613, 8865, 10975`.
  Decision 3's warm figure is a claim, not a measurement.
- K's 762 lines were read in parts (`1-260, 320-480, 636-730`); lines 260-320 (`replay`'s merge) and 480-636 (the
  encoders) were not read line by line. The split in 5e ("about 220" and "about 400" lines) is an estimate by function.
- That the old image's `retained.rs` had no file-local guard rests on a grep of that file for `local`, `cross`, `guard`
  (no such check) and on the bead's note; its 770 lines were not read.
- Main's `ryo_values::domain::stores` items were counted (24), not read one by one; which of the tip's 27 are still read
  by anything (`core/src/inbound/use_cases/lint/types.rs` and `core/src/outbound/language` name the module) is not
  checked; their fate is the values split's.
- The count of callers on main (core 12, sdk 4, cli 4) excludes tests and fixtures by file name; 19 cli files and 35
  core files name the port in all.
- Whether `memory()` has a product caller after the regroup (main's SDK `query run` used it,
  `sdk/src/resources/query/run/composition.rs:101`) is not checked.
- How the tip's `retained` Tastemaker mode is switched (`TM_CHECK_SESSION`, `--tm-batch-retained`) was taken from the
  bead; no file at the tip was found by `git grep` in `crates/tastemaker/cli/src` or the analysis adapter.
- The stamp rules the corpus would take over from `request_answers.rs` (`older_than`, both timestamps before the run's
  start) are named here, not drafted; they belong to the corpus sheet, not written yet.
- `check-graph.py` and `check-types.py` were run over the contracts folder; `check-ledger.py` was not (no ledger yet).
