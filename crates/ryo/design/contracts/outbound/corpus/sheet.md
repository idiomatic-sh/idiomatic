# outbound/corpus
STATUS: drafted (28 Nov 2026; not reviewed, not agreed). Bead ryo-fmx3.1.13.23.4.16. No line names a `ryo_table` type, so nothing here waits on the table spike.

Revisions: MAIN = f4ae3bd802 (release 0.2.0, live). TIP = 2a207464c (perf branch, build 139). Read only through
`git -C /workspaces/idiomatic-public-benchmark show|grep|ls-tree|log|diff`. A quote is one line, two spaces, `@rev:path:line`.
`C` below is `crates/ryo/crates/core/src/outbound/corpus`; `X` is `crates/ryo/crates/executor/src`.

## 1. Owns
The files a request covers, as the disk or the caller has them.

One thing. Listing them, reading one, telling whether a path is there and choosing which of them a rule's globs name
are four questions about that one set, asked of one authority, which is why main put the first three behind one port.
The "and" test passes. The path globs belong here because they choose files: the owner decided "path globs belong here"
(PLAN 8.2, agent F). The text globs a rule's `where` compares (`glob_any`) are not files, and stay with domain/operators.

## 2. On main (f4ae3bd802)
**Where.** Three homes, 1,036 Rust lines without tests or fixtures, 2,405 with them:
- `core/src/outbound/corpus` (+ `corpus.rs`): 735 lines, the port, two adapters (`ignore` 371 + 19 + 28 lines; `memory`
  170 + 4), errors, types, factory; fixtures `contract.rs` 141, `fake.rs` 124.
- `execution/src/outbound/matching` (+ `matching.rs`): 117 lines, `MatchingPort` and a globset adapter over `ryo_matching`.
- `values/src/domain/files` (+ `files.rs`): 184 lines, the data types the port names.
- plus 3 of the 11 methods of `ryo_matching`'s `MatchingUseCase` (the path globs), and the five file words it borrows from
  `ryo_language` (`FilePath`, `RootPath`, `EntryObservation`, `ObservedEntryState`, `GivenProjectInventory`).

**The contract, word for word.**
```
pub trait CorpusPort: Send + Sync {  @f4ae3bd802:crates/ryo/crates/core/src/outbound/corpus/interface.rs:10
fn inventory(&self) -> Result<CorpusInventory, CorpusError>;  @f4ae3bd802:crates/ryo/crates/core/src/outbound/corpus/interface.rs:17
fn read(&self, entry: &CorpusEntry, into: &mut FileBytes) -> Result<(), CorpusError>;  @f4ae3bd802:crates/ryo/crates/core/src/outbound/corpus/interface.rs:25
fn stat(&self, path: &FilePath) -> Result<PathStat, CorpusError>;  @f4ae3bd802:crates/ryo/crates/core/src/outbound/corpus/interface.rs:33
fn observe_entry(&self, path: &FilePath) -> EntryObservation;  @f4ae3bd802:crates/ryo/crates/core/src/outbound/corpus/interface.rs:36
pub trait MatchingPort: Send + Sync {  @f4ae3bd802:crates/ryo/crates/execution/src/outbound/matching/interface.rs:8
fn compile(&self, globs: &[Glob]) -> Result<GlobsMatcher, MatchingError>;  @f4ae3bd802:crates/ryo/crates/execution/src/outbound/matching/interface.rs:14
fn parse(&self, glob: &Glob) -> Result<(), MatchingError>;  @f4ae3bd802:crates/ryo/crates/execution/src/outbound/matching/interface.rs:21
fn compile_globs(&self, globs: &[GlobPattern<'_>]) -> Result<CompiledGlobs, MatchingError>;  @f4ae3bd802:crates/ryo/crates/matching/src/inbound/use_cases/matching/interface.rs:49
fn parse_glob(&self, glob: GlobPattern<'_>) -> Result<(), MatchingError>;  @f4ae3bd802:crates/ryo/crates/matching/src/inbound/use_cases/matching/interface.rs:55
fn match_globs(&self, globs: &CompiledGlobs, path: &std::path::Path, hits: &mut GlobHits);  @f4ae3bd802:crates/ryo/crates/matching/src/inbound/use_cases/matching/interface.rs:57
```
Types it names: `pub struct CorpusInventory(pub Vec<CorpusInput>);` (corpus/types.rs:7), `pub enum PathStat {` (corpus/types.rs:16), `pub enum CorpusError {` (corpus/errors.rs:5) (`Unreadable`, `Unstable`, `TooLarge`), and in values:
```
pub enum ExecutablePermission {  @f4ae3bd802:crates/ryo/crates/values/src/domain/files/types.rs:10
pub struct CorpusEntry {  @f4ae3bd802:crates/ryo/crates/values/src/domain/files/types.rs:24
pub enum CorpusInput {  @f4ae3bd802:crates/ryo/crates/values/src/domain/files/types.rs:38
pub struct CorpusFile {  @f4ae3bd802:crates/ryo/crates/values/src/domain/files/types.rs:64
pub enum CorpusListing {  @f4ae3bd802:crates/ryo/crates/values/src/domain/files/types.rs:75
pub struct FileBytes(pub Vec<u8>);  @f4ae3bd802:crates/ryo/crates/values/src/domain/files/types.rs:84
pub struct GlobsMatcher {  @f4ae3bd802:crates/ryo/crates/values/src/domain/files/types.rs:121
pub matches: Arc<dyn Matching>,  @f4ae3bd802:crates/ryo/crates/values/src/domain/files/types.rs:123
```
**Public items: 38.** core's corpus 11 (`CorpusPort`, `CorpusInventory`, `SkippedDirectories`, `PathStat`, `CorpusError`,
`factory::{ignore, memory}`, `fixtures::contract::{Scenario, CASES, check}`, `fixtures::fake::CorpusFake`); execution's
matching 6 (`MatchingPort`, `factory::globset`, `fixtures::contract::{CASES, answer, check}`, `fixtures::fake::MatchingFake`);
values' files 21 (counted by `^\s*pub (trait|struct|enum|fn|type|const) ` over the three directories).

**Implementors.** `CorpusIgnoreAdapter` (the disk; a parallel `ignore` walk, `builder.build_parallel().run(|| {` (ignore/adapter.rs:106); a listing stats each named path
in parallel, `fn listed(` (ignore/adapter.rs:159); a read
is stable or read once more, `for _ in 0_i32..2_i32 {` (ignore/adapter.rs:280), with an `fstat` before and after
it), `CorpusMemoryAdapter` (files given as text, and the host's `GivenProjectInventory`), `CorpusFake`. `MatchingPort`:
`MatchingGlobsetAdapter`, `MatchingFake`.

**Callers (files per crate, by `git grep -l "CorpusPort|corpus::factory|outbound::corpus"`, tests included).** core 43
(the lint use case `corpus: Arc<dyn CorpusPort>`; `FileAnalysis` reads every file through it,
`if let Err(error) = self.corpus.read(file, &mut worker.tables.bytes) {` (file_analysis/service.rs:204); storing lists it
`.map(|inventory| inventory.0)` (storing/service.rs:125); project
context stats consulted paths `.any(|path| !matches!(corpus.stat(path), Ok(PathStat::Missing)))` (project_context/service.rs:132); cross-file is built over it
`pub fn new(corpus: Arc<dyn CorpusPort>, publication: Arc<dyn PublicationUseCase>) -> Self {` (cross_file/service.rs:62)); sdk 3 (the composition builds
`Arc::new(corpus::factory::ignore(root, listing, beside, skipped))` (lint/composition.rs:143) or
`Arc::new(corpus::factory::memory(files, inventory))` (lint/composition.rs:146)); cli 7. `MatchingPort`: core's lint
compiles each rule's placement globs once per request, `matching: &dyn MatchingPort,` (lint/service.rs:82), `.compile(&placement.globs)` (lint/service.rs:90); the SDK parses each glob of a request.

**Proof.** 9 `#[test]`: ignore adapter 2 (`succeeds`, `refuses`) over 17 case rows, memory adapter 2 over 11 rows, the fake
2, the globset adapter 2, the matching fake 1. The port contract, 5 rows every adapter runs
(`pub fn check(adapter: &impl CorpusPort) {` (fixtures/contract.rs:81)):
`every_file_listed_reads_back_with_the_size_listed`, `every_file_listed_stats_as_it_is_listed`,
`a_path_where_no_file_is_stats_as_none`, `every_file_listed_is_observed_as_a_file`, `a_file_that_is_not_listed_is_refused`.
The globs contract, 3 rows (`a_star_crosses_no_slash_and_two_stars_cross_any`, `a_path_that_matches_nothing_has_no_hits`,
`a_glob_that_is_not_one_is_refused`). `ryo_matching` adds 5 tests and 2 scenario outlines, shared with the regex and text
globs. One divan bench (section 8).

**Tastemaker entries.** None names the concept (`git grep -i corpus` over `crates/ryo/tastemaker.toml` and
`crates/ryo/tastemaker/` finds only the allocation test's description and two changelogs).

Quoted in this section, one per line:
```
pub struct CorpusInventory(pub Vec<CorpusInput>);  @f4ae3bd802:crates/ryo/crates/core/src/outbound/corpus/types.rs:7
pub enum PathStat {  @f4ae3bd802:crates/ryo/crates/core/src/outbound/corpus/types.rs:16
pub enum CorpusError {  @f4ae3bd802:crates/ryo/crates/core/src/outbound/corpus/errors.rs:5
builder.build_parallel().run(|| {  @f4ae3bd802:crates/ryo/crates/core/src/outbound/corpus/adapters/ignore/adapter.rs:106
fn listed(  @f4ae3bd802:crates/ryo/crates/core/src/outbound/corpus/adapters/ignore/adapter.rs:159
for _ in 0_i32..2_i32 {  @f4ae3bd802:crates/ryo/crates/core/src/outbound/corpus/adapters/ignore/adapter.rs:280
if let Err(error) = self.corpus.read(file, &mut worker.tables.bytes) {  @f4ae3bd802:crates/ryo/crates/core/src/application/file_analysis/service.rs:204
.map(|inventory| inventory.0)  @f4ae3bd802:crates/ryo/crates/core/src/application/storing/service.rs:125
.any(|path| !matches!(corpus.stat(path), Ok(PathStat::Missing)))  @f4ae3bd802:crates/ryo/crates/core/src/application/project_context/service.rs:132
pub fn new(corpus: Arc<dyn CorpusPort>, publication: Arc<dyn PublicationUseCase>) -> Self {  @f4ae3bd802:crates/ryo/crates/core/src/application/cross_file/service.rs:62
Arc::new(corpus::factory::ignore(root, listing, beside, skipped))  @f4ae3bd802:crates/ryo/crates/sdk/src/resources/linter/lint/composition.rs:143
Arc::new(corpus::factory::memory(files, inventory))  @f4ae3bd802:crates/ryo/crates/sdk/src/resources/linter/lint/composition.rs:146
matching: &dyn MatchingPort,  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/lint/service.rs:82
.compile(&placement.globs)  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/lint/service.rs:90
pub fn check(adapter: &impl CorpusPort) {  @f4ae3bd802:crates/ryo/crates/core/src/outbound/corpus/fixtures/contract.rs:81
```

## 3. On the perf branch (2a207464c)
**Where.** The designed homes are still there and shrank: core's corpus 551 lines without tests (541 + 10; the memory
adapter, its factory function, the fake and their 361 test lines deleted; an orphan 7-line `adapters/memory.rs` is left that
nothing declares, `pub(crate) mod adapter;` (adapters/memory.rs:3)),
execution's matching 117 (the fake deleted), values' files 184 (unchanged: `git diff --stat` empty). The port's text is
unchanged (`git diff` of `interface.rs`, `types.rs`, `errors.rs`: empty):
```
fn read(&self, entry: &CorpusEntry, into: &mut FileBytes) -> Result<(), CorpusError>;  @2a207464c:crates/ryo/crates/core/src/outbound/corpus/interface.rs:25
pub fn ignore(  @2a207464c:crates/ryo/crates/core/src/outbound/corpus/factory.rs:9
```
**Public items: 35** in those homes (9 + 5 + 21), plus about 14 public items of the concept in the executor (below).

**Who calls the port: one file.** The SDK walks a `Corpus::Walked` request and hands the executor a list:
```
let walk = ryo_core::outbound::corpus::factory::ignore(  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/walk.rs:38
let inventory = walk.inventory().map_err(|error| LintError::Io { what: Port::Corpus, reason: error.to_string() })?;  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/walk.rs:44
```
`read`, `stat` and `observe_entry` have no caller outside the adapter's own tests. The cli names only
`SkippedDirectories` (4 files). `MatchingPort::compile` has no product caller; only `parse` is called,
`match super::composition::build_matching().parse(&CoreGlob(text.into())) {` (lint/schemas.rs:45).

**What the run actually calls: the executor's own disk reads, in six places.**
- Reading: `pub fn versions<'s>(` (src/check_session.rs:726), one call per group of
  sources that share a relative path across roots: alone,
  `let bytes: Arc<[u8]> = std::fs::read(path)?.into();` (src/check_session.rs:744);
  in a group, stat, key by identity, read once:
  ```
  let meta = std::fs::metadata(&path)?;  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:782
  let inode = (meta.dev(), meta.ino(), src.environment.as_ref().map(|e| &e.id));  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:783
  Arc::from(std::fs::read(path)?),  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:789
  let key = (data.len(), src.provided.as_ref().map(|_| src.permission), perm, src.environment.as_ref().map(|e| &e.id), Arc::clone(&data));  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:799
  ```
  Files given as text skip the disk: `pub provided: Option<Arc<[u8]>>,` (src/check_session.rs:189). Its answers: `pub struct Version<'s> {` (src/check_session.rs:707), `pub struct ReadFailed {` (src/check_session.rs:718) (`reason`, `missing`).
- Absence: each source remembers what its read found, `pub seen: std::sync::atomic::AtomicU8,` (src/check_session.rs:191), `fn saw(&self, read: &std::io::Result<()>) {` (src/check_session.rs:217), and `pub fn is_absent(&self) -> bool {` (src/check_session.rs:200) asks the disk only of a file no read reached,
  `let absent = std::fs::metadata(Path::new(&*self.root).join(&self.path))` (src/check_session.rs:209); its one caller,
  `Some(&id) => schedule.sources[id].is_absent(),` (replay/closed_workers.rs:32), still stats paths no source holds,
  `None => std::fs::metadata(std::path::Path::new(&**root).join(path))` (replay/closed_workers.rs:33).
- Sizes for shard weights, one stat per file:
  `std::fs::metadata(Path::new(src.root.as_ref()).join(&src.path))` (check_session/worker.rs:117).
- A refusal's text, by opening the file again:
  `std::fs::File::open(std::path::Path::new(&*self.schedule.sources[slot].root).join(&self.schedule.sources[slot].path))` (replay/batch.rs:740).
- The executable bit, one stat per file for `filesystem.executable`:
  `Ok(std::fs::metadata(path)?.permissions().mode() & 0o111 != 0)` (src/host_columns.rs:15).
- The project's entries: `pub enum Authority {` (check_session/project.rs:284)
  (`Given`, `Disk`, `Scoped`), `pub fn observe(&self, files: &[l::FilePath], path: l::FilePath) -> EntryObservation {` (check_session/project.rs:313), which on disk is
  `Some(root) => match std::fs::metadata(Path::new(root).join(&*path)) {` (check_session/project.rs:327) (main's `observe_entry` again, line for line);
  memoised per admission epoch in `pub struct Environment {` (check_session/project.rs:583) by
  `pub fn physical(&self, path: &Path) -> l::ObservedEntryState {` (check_session/project.rs:615), listed for the store by
  `pub fn observed(&self, mut read: impl FnMut(&Path, l::ObservedEntryState)) {` (check_session/project.rs:603), and lent to languages through
  `impl l::FileProject for Environment {` (check_session/project.rs:629).

And a seventh, in the kept store (outbound/cache's sheet): `pub struct Stamp {` (stored/request_answers.rs:29), built by
`pub fn from_metadata(m: &fs::Metadata) -> Self {` (stored/request_answers.rs:38), and
`pub fn observe(&mut self, p: &Path, mode: Probe) -> Option<Key> {` (stored/request_answers.rs:202), which stats
(`fs::metadata(p).ok()?` (stored/request_answers.rs:220)),
opens and reads (`let f = fs::File::open(p).ok()?;` (stored/request_answers.rs:222)) and hashes
(`let digest = *blake3::hash(&self.buffer).as_bytes();` (stored/request_answers.rs:265)) on its own.

**Path globs: a second implementation in the SDK**, with its own `globset` dependency (added to `sdk/Cargo.toml` on the tip):
```
pub(super) enum PathFilter {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/call_cache.rs:13
pub(super) fn compile(names: &[sdk::Glob]) -> Result<Self, String> {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/call_cache.rs:19
fn prefix_ranges(&self, inventory: &[String]) -> Option<Vec<std::ops::Range<usize>>> {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/call_cache.rs:55
pub(super) fn files(  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/call_cache.rs:161
.filter(|p| inventory.binary_search_by(|v| v.as_str().cmp(p)).is_ok())  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/call_cache.rs:185
let matcher = PathFilter::compile(names)?;  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:306
```
Literal include lists probe the sorted listing; patterns with a fixed prefix search ranges of it; anything else scans.

**Proof.** 4 `#[test]` in the designed homes (ignore adapter 2 over the same 17 rows; globset adapter 2), the port contract's
5 rows (now run by one adapter), the globs contract's 3 rows, and `walk_test.rs` 1 (the store-directory skip). Nothing tests
`versions`, `is_absent`, `Authority::observe` or `PathFilter` as a contract; `project_test.rs` exists beside `project.rs`
(not read).

Quoted in this section, one per line:
```
pub(crate) mod adapter;  @2a207464c:crates/ryo/crates/core/src/outbound/corpus/adapters/memory.rs:3
match super::composition::build_matching().parse(&CoreGlob(text.into())) {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/schemas.rs:45
pub fn versions<'s>(  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:726
let bytes: Arc<[u8]> = std::fs::read(path)?.into();  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:744
pub provided: Option<Arc<[u8]>>,  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:189
pub struct Version<'s> {  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:707
pub struct ReadFailed {  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:718
pub seen: std::sync::atomic::AtomicU8,  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:191
fn saw(&self, read: &std::io::Result<()>) {  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:217
pub fn is_absent(&self) -> bool {  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:200
let absent = std::fs::metadata(Path::new(&*self.root).join(&self.path))  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:209
Some(&id) => schedule.sources[id].is_absent(),  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/closed_workers.rs:32
None => std::fs::metadata(std::path::Path::new(&**root).join(path))  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/closed_workers.rs:33
std::fs::metadata(Path::new(src.root.as_ref()).join(&src.path))  @2a207464c:crates/ryo/crates/executor/src/check_session/worker.rs:117
std::fs::File::open(std::path::Path::new(&*self.schedule.sources[slot].root).join(&self.schedule.sources[slot].path))  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/batch.rs:740
Ok(std::fs::metadata(path)?.permissions().mode() & 0o111 != 0)  @2a207464c:crates/ryo/crates/executor/src/host_columns.rs:15
pub enum Authority {  @2a207464c:crates/ryo/crates/executor/src/check_session/project.rs:284
pub fn observe(&self, files: &[l::FilePath], path: l::FilePath) -> EntryObservation {  @2a207464c:crates/ryo/crates/executor/src/check_session/project.rs:313
Some(root) => match std::fs::metadata(Path::new(root).join(&*path)) {  @2a207464c:crates/ryo/crates/executor/src/check_session/project.rs:327
pub struct Environment {  @2a207464c:crates/ryo/crates/executor/src/check_session/project.rs:583
pub fn physical(&self, path: &Path) -> l::ObservedEntryState {  @2a207464c:crates/ryo/crates/executor/src/check_session/project.rs:615
pub fn observed(&self, mut read: impl FnMut(&Path, l::ObservedEntryState)) {  @2a207464c:crates/ryo/crates/executor/src/check_session/project.rs:603
impl l::FileProject for Environment {  @2a207464c:crates/ryo/crates/executor/src/check_session/project.rs:629
pub struct Stamp {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/stored/request_answers.rs:29
pub fn from_metadata(m: &fs::Metadata) -> Self {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/stored/request_answers.rs:38
pub fn observe(&mut self, p: &Path, mode: Probe) -> Option<Key> {  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/stored/request_answers.rs:202
fs::metadata(p).ok()?  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/stored/request_answers.rs:220
let f = fs::File::open(p).ok()?;  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/stored/request_answers.rs:222
let digest = *blake3::hash(&self.buffer).as_bytes();  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/stored/request_answers.rs:265
```

## 4. What changed between them
- **Removed:** the memory adapter, its factory function, the corpus fake, their tests (`f2d1845a4f`: "the lint store port
  is deleted ... and the memory corpus adapter with it"); the matching fake and the corpus fake's fixtures (`35b6eaf0cf`:
  "delete what audit B lists (modules nothing calls ...)"). The memory adapter is a designed contract dropped because
  nothing called it, the case the owner forbade on 27 Nov; the bench lost its second variant with it,
  `Self::Memory => Box::new(memory(files.to_vec(), GivenProjectInventory::default())),` (corpus/bench.rs:44).
- **Bypassed, not changed:** the port is intact but the run reads, stats and observes the disk from the executor (section
  3). The inode and content sharing came with `ec0f8b6c7d` ("Share bound programs, file versions and rule-file answers
  across a check", 23 Nov, not on main); the absence-from-the-read with `eb28df5638` (27 Nov, build 136). The path-glob
  selection moved into the SDK with the session glue (`ce8077f9af`, `5094c6add3`); its literal probe and prefix ranges were
  measured on 24 Nov (section 7).
- **Added:** nothing to the port. The executor's reads use `Arc<[u8]>` where the port fills a caller buffer.
- **Lost that should not have been:** the memory adapter and its 11 case rows, the corpus fake and its "a file that changes
  while it is read is refused as unstable" test, the matching fake; the read-stability check (main reads twice and refuses
  an unstable file; the tip's `std::fs::read` takes whatever it gets); one place that reads the disk (the tip reads it in
  seven).

Quoted in this section, one per line:
```
Self::Memory => Box::new(memory(files.to_vec(), GivenProjectInventory::default())),  @f4ae3bd802:crates/ryo/crates/core/benches/performance/corpus/bench.rs:44
```

## 5. The drafted public interface        (explains every line of api.txt that is changed or new)
`api.txt` beside this sheet: **35 public items** (2 traits, 8 methods, 19 types, 3 factories, 3 fixtures; 16 enum variants on
their own lines), against 38 on main and 35 + about 14 on the tip. Of its 51 lines: 28 `[main]`, 19 `[changed]`, 4 `[new]`.

### 5a. The claim it starts from, checked
Appendix A 3.14 drafts `CorpusPort { inventory, read -> FileVersion, stat, observe_entry }`,
`FileVersion { bytes: Arc<[u8]>, stamp: Stamp, content: Key }` and `GlobsUseCase { compile, parse }`. Checked against the code:
- The four port methods are main's verbatim (`interface.rs:17, 25, 33, 36`), unchanged on the tip. Kept, `read` changed.
- `content: Key` is **not** carried. `Key` is the kept store's (`stored::Key`, a blake3 digest), and hashing every read is the
  cost perf row 10 measured as unjustified (+600 M instructions). The tip keys a version by identity and bytes, not by a
  hash (`check_session.rs:783`, `:799`). Decision 3.
- `GlobsUseCase { compile, parse }` is not enough for its one real caller: the tip's selection is `CallCache::files` over a
  sorted listing, and its speed is the listing search, not the compile. `select` is added; the trait is named
  `PathGlobsUseCase` so it is not read as the text globs operators owns (`Function::GlobAny`). Decision 2.
- Appendix A puts the executor's `versions` and the store's stamps behind the port. Checked: the disk part goes behind it;
  the slot grouping inside `versions` (`aliases`, `FilePrograms::admit`, the `environment.id` in the key) is execute's,
  because it is about which answers a version serves, not about files.

### 5b. The port
```
pub trait CorpusPort: Send + Sync {                                        @main
    fn inventory(&self) -> Result<CorpusInventory, CorpusError>;           // behaviour: full stamps; Missing told apart
    fn read(&self, entry: &CorpusEntry) -> Result<FileVersion, CorpusError>;   // CHANGED
    fn stat(&self, path: &FilePath) -> Result<PathStat, CorpusError>;      // main
    fn observe_entry(&self, path: &FilePath) -> EntryObservation;          // behaviour: memoised per corpus value
    fn observed(&self, into: &mut Vec<EntryObservation>);                  // tip's Environment::observed
}
```
- `read` returns `FileVersion { bytes: Arc<[u8]>, stamp: Stamp }` (`@new`). Main filled a caller buffer
  (`interface.rs:25`), which forces a copy into an owned buffer whenever bytes outlive the visit; the tip's whole run holds
  `Arc<[u8]>` (`check_session.rs:744`, `:789`) and keeps it past the visit for a pending closed reader. Given files are
  `CorpusFile { bytes: Arc<[u8]> }` and are handed out by `Arc::clone`, as `Source::provided` is today. The stamp is the one
  the read saw, so the store can keep it without a second stat.
- `inventory` keeps its signature; what it says grows. Every `CorpusEntry` carries `stamp: Stamp` with the file's identity
  (device, inode), size, write and change times and executable bit, which the walk already has (`entry.metadata()`) and a
  listing already stats (`fn listed(`, one parallel stat per named file). A listed file that is not there becomes
  `CorpusInput::Missing { path }` (`@new`), told apart from `Unreadable`; `read` likewise answers `CorpusError::Missing`
  (`@new`), where main folded "not found" into `Unreadable`. With these, no caller stats a listed file again: the shard
  weight stat (`worker.rs:117`), the inode stat (`check_session.rs:782`), `is_absent` (`:209`), `closed_workers.rs:33`, the
  executable stat (`host_columns.rs:15`) and the refusal's reopen (`batch.rs:740`) are each answered by the inventory entry
  or the read's error.
- `stat` is main's verbatim. `PathStat::Found(CorpusEntry)` now carries the full stamp, so the kept store's validity check
  (`request_answers.rs:220`) asks the corpus instead of the disk.
- `observe_entry` is main's signature. Its memo is the tip's `Environment::physical`: one disk lookup per path for the life
  of the corpus value; the edge builds a new corpus value for a new round, which replaces the tip's epoch counter. `observed`
  lists what it looked up, for the kept store's proof (the tip's `Environment::observed`, `project.rs:603`). The `Given`
  authority is the memory adapter (`GivenProjectInventory`'s completeness claim decides `Missing` against `Unknown`), `Disk`
  is the ignore adapter. `Scoped { roots }` is not the corpus's: the roots are a language request's search scope
  (`ryo_language::ProjectSearchScope`).
- The `impl FileProject for Environment` cannot move here: `FileProject` is outbound/language's trait and language must not
  depend on corpus. The bridge, about 20 lines over `CorpusPort::observe_entry` and the request's supplied paths, goes to
  application/execute, which imports both. The project sheet's decision 4 says "Environment and Authority to
  outbound/corpus"; this sheet takes their disk part and their memo, and hands the bridge to execute (decision 1).

### 5c. Path globs
```
pub trait PathGlobsUseCase: Send + Sync {                                  // changed: MatchingPort + 3 matching methods
    fn compile(&self, globs: &[Glob]) -> Result<PathGlobs, GlobError>;
    fn parse(&self, glob: &Glob) -> Result<(), GlobError>;
    fn select(&self, listing: &[FilePath], all_of: &[&PathGlobs], none_of: &PathGlobs, into: &mut Vec<ListedNumber>);
}
```
- `PathGlobs` is opaque and concrete: the tip's `PathFilter { Literals, Patterns { set, prefixes } }` (`call_cache.rs:13`).
  Main's `GlobsMatcher { matches: Arc<dyn Matching> }` (`types.rs:121`, `:123`) made every path a `dyn` call; it goes.
- `select` is `CallCache::files` (`call_cache.rs:161`) without its memo: `listing` is sorted and unique (the caller
  interns it, as `CallCache::inventory` does), `all_of` are the request's and the rule's include sets, `none_of` the rule's
  exclude set; it writes the places of the chosen files as `ListedNumber(u32)` (`@new`), so no path is copied. Literal sets
  probe the listing by binary search (`call_cache.rs:185`); prefixed patterns search ranges of it when that costs fewer
  comparisons than a scan (`call_cache.rs:55`); otherwise it scans. The memo of selections (`selections`, keyed by the four
  interned ids) stays with the caller, inbound/lint, which owns the call cache.
- `GlobError { glob, reason }` is main's `MatchingError::BadGlob`; `InvalidRegex` cannot come from a path glob.
- `globset` becomes this crate's dependency and leaves the SDK; `ryo_matching` loses its three path methods (the operators
  sheet takes the other eight). One implementation remains.

### 5d. How it is built
`factory::ignore(root, listing, beside, skipped)` is main's verbatim (`factory.rs:9`); `factory::memory(files, inventory)`
is main's, restored (`factory.rs:20`); `factory::globset()` is execution's, moved. Each returns `impl Trait`: callers are
generic over `C: CorpusPort` and `P: PathGlobsUseCase`, never `dyn`. Only an edge calls a factory; the SDK and the command
line build the corpus and hand it to inbound/lint.

### 5e. What stops being public, and the counts
| Side | Public items in the concept | Stop being public / leave |
|---|---|---|
| main `f4ae3bd802` | 38 (11 + 6 + 21) | 20 named in api.txt's `# NO LONGER PUBLIC` (`FileBytes`, `GlobsMatcher`, `Matching`, `MatchCandidate`, `Hits`, `PlatformPath`, `NativeSeparator`, `with_separator`, `input_path`, `separator`, `FileNumber`, `SourceView`, `MatchingPort`, `MatchingFake`, `answer`, the 3 matching methods with their 3 types, `Scenario`, `CASES`) |
| tip `2a207464c` | 35 + about 14 in the executor | the executor's 14 (`versions`, `Version`, `ReadFailed`, `Source::is_absent`, three `SEEN_*`, `Authority` with `disk`/`with_roots`/`observe`, `Environment` with `new`/`observed`/`physical`, the store's `Stamp`, `host_columns::executable`) leave the executor; the SDK's `PathFilter` was never public |
| drafted | 35 | |

`FileNumber` is named by nothing outside `values` at the tip (word grep); `SourceView` serves findings' line starts (core's
`finish`, lint's `coordinates`), not files; both leave this concept.

## 6. Proof it brings
| Proof | Source | Fate |
|---|---|---|
| the port contract, 5 rows | `fixtures/contract.rs` (both revisions) | **carried**, run by both adapters; restated where `read` returns `FileVersion` and `Missing` is its own answer |
| ignore adapter, 17 rows (`succeeds`, `refuses`) | `adapters/ignore/adapter_test.rs` (both) | **carried**; `a_file_that_is_not_there_is_refused_naming_its_path_once` restated to `CorpusError::Missing` |
| memory adapter, 11 rows | `adapters/memory/adapter_test.rs` (main only) | **restored from main** |
| corpus fake, 2 tests | `fixtures/fake_test.rs` (main only) | **restored from main** (the unstable-read test is the only test of `Unstable`) |
| globs contract, 3 rows; globset adapter, 2 tests | `execution/.../matching/fixtures/contract.rs`, `adapter_test.rs` | **carried**, restated against `PathGlobsUseCase` |
| `walk_test.rs`, 1 (store directory skipped) | sdk (tip) | stays with the SDK (the skip is the edge's composition) |
| `select` equals a plain scan for literal, prefixed and other sets | none | **MISSING**: the case table that makes the probe and the ranges safe |
| a listed file read once per identity; nothing stats a listed file again | none | **MISSING**: a counting fake in the contract (rows 50, 51) |
| `observe_entry` asks the disk once per path; `observed` lists exactly those | none | **MISSING** |

Mutation table for the new behaviour: answer `Missing` as `Unreadable` (caught by the restated not-there row); stat in
`observe_entry` on every call (caught by the counting row); drop the prefix bound in `select` (caught by the scan-equality
table); hand out a copy instead of the given `Arc` (caught by an `Arc::ptr_eq` row on a given file).

## 7. Perf gains it carries
| Row | What | State | What in this draft keeps it |
|---|---|---|---|
| 50 | file versions shared by inode and content (Ryo workspace: 38,645 shared versions; 23,076,649 duplicate source bytes avoided; `build/tm-int/ONE-PATH.md:129`) | live | the inode half lands here: `CorpusEntry.stamp.identity` is known from the inventory, so the caller groups entries by identity **before** reading and calls `read` once per identity; `FileVersion.bytes` is an `Arc` shared by every slot. The content half (equal bytes under two identities, keyed with the permission and the project environment, `check_session.rs:799`) stays in execute, comparing `FileVersion.bytes`; it needs no new call |
| 51 | a listed file's absence known from the read made of it (stat-family calls on the Tastemaker check 119,444 -> 57,985; +2.0% CPU removed; `engine/STATE.md:11167`, `:11175`) | live | `CorpusInput::Missing` and `CorpusError::Missing`: absence comes from the inventory or the read, and the draft goes further than the tip, which still stats in `closed_workers.rs:33`, `worker.rs:117`, `host_columns.rs:15` and `batch.rs:740`. The check: stat-family syscalls on the Tastemaker check at or below 57,985 |
| not in perf-gains.tsv | literal include lists probe the sorted listing (52,833 probes replace 9,416,109 inventory visits; rule checks and path selection 1.879 -> 0.976 CPU-s; `engine/STATE.md:6746`), and prefix ranges where cheaper by count (2,917,414 matcher visits avoided on Ryo; `engine/STATE.md:7130`) | live, in the SDK | `PathGlobsUseCase::select` over a sorted `&[FilePath]`, with `PathGlobs` concrete; **a finding for the perf-gain inventory: a landed gain with no row** |

Gains this crate makes room for: row 10 (whole-call keys) depends on "a request's key computed once per rule text from one
place (the corpus's file identity ...)": the identity is `Stamp.identity`, one type, which the cache sheet uses instead of
its own `Stamp`. Row 2's kept store and row 14's warm store read file stamps; they read them from `PathStat` and `FileVersion`.

**Would have landed inside this crate alone under the draft:** row 51 (all of it), row 50's inode half, the literal probe and
the prefix ranges. **Would still cross:** row 50's content half (execute), row 10's keying (cache and lint).

**A risk the draft must not carry in:** main's ignore adapter reads with an `fstat` before and after
(`let before = stamp(&file).map_err(|error| unreadable_file(entry, &error))?;` (ignore/adapter.rs:285),
`let after = stamp(&file).map_err(|error| unreadable_file(entry, &error))?;` (ignore/adapter.rs:296)), where the tip's `std::fs::read`
makes one. Taken verbatim it adds a stat per file and undoes part of row 51. The adapter compares the open file's one stamp
with the inventory entry's instead (same check, one call); the syscall count above is the gate.

Quoted in this section, one per line:
```
let before = stamp(&file).map_err(|error| unreadable_file(entry, &error))?;  @f4ae3bd802:crates/ryo/crates/core/src/outbound/corpus/adapters/ignore/adapter.rs:285
let after = stamp(&file).map_err(|error| unreadable_file(entry, &error))?;  @f4ae3bd802:crates/ryo/crates/core/src/outbound/corpus/adapters/ignore/adapter.rs:296
```

## 8. Benchmarks that cover it
| Benchmark | What it calls in this concept today | After the draft |
|---|---|---|
| `crates/ryo/crates/core/benches/performance/corpus/bench.rs` (divan; main and tip) | `fn inventory_and_read(bencher: divan::Bencher<'_, '_>, variant: Variant) {` (corpus/bench.rs:57), over `factory::ignore`, `let inventory = adapter.inventory().expect("the tree is listed");` (corpus/bench.rs:69) and `adapter.read(entry, &mut buffer).expect("the file is read");` (corpus/bench.rs:77); one variant (main had `Memory` too) | moves into this crate (`benches/performance/corpus`); the same 200-file tree, list and read every file; `read` returns a `FileVersion` instead of filling the buffer; the `Memory` variant restored. No Asymptote lock names it on either revision |
| Ruff, Oxlint and Clippy linter benchmarks (the `vec`, `vec_ts`, `vec_rust` images, `crates/ryo/crates/executor/benches/lint_image`) | nothing of the port: the image sends `corpus: sdk::Corpus::Listed(request.files.into_iter().map(sdk::RelativePath).collect()),` (lint_image/main.rs:128) and the executor reads the files with `std::fs::read` | the SDK builds `factory::ignore(.., CorpusListing::Listed(..), ..)`; lint reads each file through `CorpusPort::read`. What they measure is unchanged; their CPU, peak and the stat count are the check that the read path costs no more |
| the Tastemaker check (`crates/tastemaker/benchmarks/project`, image `tm`) | the SDK's `walk.rs` (Walked), `PathFilter` (selection), the executor's reads; rows 50, 51 and the selection gain were measured here | the SDK's walk uses `factory::ignore` as today; selection is `PathGlobsUseCase::select`; reads as above. The one-worker CPU cell and the stat count (57,985) are the check |
| `benchmark` crate (`benchmark-collector`), `benchmark_contracts`, the retention matrix | nothing (their "corpus" is a benchmark project, a different word; no file names `CorpusPort` or `corpus::factory`) | unchanged |

Quoted in this section, one per line:
```
fn inventory_and_read(bencher: divan::Bencher<'_, '_>, variant: Variant) {  @2a207464c:crates/ryo/crates/core/benches/performance/corpus/bench.rs:57
let inventory = adapter.inventory().expect("the tree is listed");  @2a207464c:crates/ryo/crates/core/benches/performance/corpus/bench.rs:69
adapter.read(entry, &mut buffer).expect("the file is read");  @2a207464c:crates/ryo/crates/core/benches/performance/corpus/bench.rs:77
corpus: sdk::Corpus::Listed(request.files.into_iter().map(sdk::RelativePath).collect()),  @2a207464c:crates/ryo/crates/executor/benches/lint_image/main.rs:128
```

## 9. Checks
IMPORTS: language
CROSSED: per file
- **Imports.** `language` only, for `FilePath`, `RootPath`, `EntryObservation`, `ObservedEntryState` and
  `GivenProjectInventory` (main's corpus already imports them from `ryo_language`). outbound/language's drafted
  `IMPORTS: table, symbols, relations, operators, store` does not name corpus, so there is no cycle. This is an
  outbound -> outbound import: Appendix A section 2 and `checks/check-graph.py` allow it; the plan's section 4 and the common
  brief (8.1) say "outbound and languages import domain" only. The two texts disagree (decision 1).
- **No layout word, no I/O type** in a signature (`std::fs::Metadata`, `std::io::Error` stay inside the adapters; errors are
  text). `std::sync::Arc<[u8]>` in `FileVersion` and `CorpusFile` is a sharing decision the whole run already makes, not a
  table layout.
- **How often it is crossed.** `inventory`: once per request (Walked or Listed). `read`: once per file identity. `stat`: per
  path the store checks on a warm run. `observe_entry`: per project lookup, memoised. `select`: once per distinct (listing,
  include, exclude) per request, memoised by lint. Never per row or per cell. Generic over the adapter type.
- **Enforcement.** Cargo: only an edge depends on `ryo_corpus`'s factories; the executor and lint lose `std::fs` reads of
  source files (an existing Tastemaker `io-at-boundary`-style rule would hold it; not checked which rule names `std::fs`).
  Proposed entries: `crate-corpus`, `nu-hex-corpus`.

## 10. Decisions for the owner              (each: the example from the code; why; what we get; my recommendation)
1. **Where the file words live, and whether outbound may import outbound.** Example: `CorpusPort::stat` takes
   `&FilePath`, which is `ryo_language`'s (main imports it at `interface.rs` line 5), and the language sheet keeps
   `FilePath`, `RootPath`, `EntryObservation`, `ObservedEntryState` as its own. Why: Appendix A 2 and the graph check let
   outbound import outbound; plan 4 and 8.1 do not. (A) corpus imports language, as main does (drafted; the bridge
   `impl FileProject` goes to execute). (B) the five words move down to domain/relations (which already owns `FileId` and the
   `filesystem_*` relations) and both outbound crates import domain only. What we get: (A) no change to two sheets; (B) the
   stricter rule holds. **My pick: B**, if the relations sheet takes them; until then the listing is written as A.
2. **One path-glob implementation, with `select`.** Example: the SDK compiles path globs with its own `globset`
   (`PathFilter::compile`, `call_cache.rs:19`) while `MatchingPort::compile` has no caller and makes every path a
   `dyn Matching` call. Why: two implementations of one concept, and the fast one is private to the SDK. What we get: one
   `PathGlobsUseCase` with the literal probe and the prefix ranges, the SDK free of `globset`, `MatchingPort`,
   `GlobsMatcher`, `Matching`, `Hits` gone. **My pick: yes**, with the scan-equality case table first.
3. **No content key in `FileVersion`.** Example: Appendix A's `FileVersion { bytes, stamp, content: Key }`; the store hashes
   with blake3 (`request_answers.rs:265`) and row 10 measured per-request hashing at +600 M instructions, unjustified. Why:
   a key computed on every read is paid by runs that keep nothing. (A) no key: the corpus gives identity and bytes, the
   cache hashes what it keeps (drafted). (B) the corpus computes the key on every read. **My pick: A.**
4. **Restore the memory adapter and the corpus fake.** Example: `pub fn memory(` (corpus/factory.rs:20) and `pub struct CorpusFake {` (fixtures/fake.rs:14), deleted in `f2d1845a4f` and `35b6eaf0cf`
   because nothing called them. Why: your rule (27 Nov) that a designed contract is not dropped for having no caller; and the
   tip's `Corpus::Given` and `Authority::Given` re-implement it inside the executor (`Source::provided`, `Authority::observe`'s
   `None` arm). What we get: `Given` requests and Tastemaker's `Corpus::Given` go through the port; 4 tests (11 case rows and the
   unstable-read test) and a bench variant back. **My pick: restore.**
5. **A read checks stability against the inventory's stamp.** Example: main stamps before and after every read
   (`adapter.rs:285`, `:296`) and reads an unstable file twice; the tip does neither. Why: correctness (a file rewritten
   mid-read is not any version of it) against a syscall per file (row 51's measure). What we get: the `Unstable` refusal kept
   at one `fstat` per read. **My pick: keep the check, one `fstat`, compared with the entry's stamp.**

Quoted in this section, one per line:
```
pub fn memory(  @f4ae3bd802:crates/ryo/crates/core/src/outbound/corpus/factory.rs:20
pub struct CorpusFake {  @f4ae3bd802:crates/ryo/crates/core/src/outbound/corpus/fixtures/fake.rs:14
```

## 11. Not checked
- **The syscall arithmetic.** That the draft's stat count on the Tastemaker check is at or below 57,985 is reasoned, not
  measured: I did not count which of the tip's per-file stats (`worker.rs:117`, `host_columns.rs:15`, `closed_workers.rs:33`)
  fire on that workload, nor whether a `Listed` inventory's parallel stat costs more wall than the tip's lazy stats.
- **The content half of row 50.** Whether execute's content grouping can key on `FileVersion.bytes` plus the permission and
  the environment exactly as `check_session.rs:799` does, without the corpus, is argued from the code, not tried.
- **`select`'s exact contract.** Whether the request filter of `CallCache::files` (`request.matches(path, true)` with
  "empty matches") is one more `all_of` set or needs its own flag was read, not traced through `entry.rs:630`.
- **`Scoped { roots }`.** That the roots are the language request's search scope and not the corpus's is read from
  `Environment`'s `FileProject::roots`, not from every caller of `Authority::with_roots`.
- **The epoch.** That a new corpus value per admission round replaces the tip's `observation_epoch` counter
  (`entry.rs:431`) without re-walking: the edge's composition was not traced.
- **The orphan file** `C/adapters/memory.rs` at the tip (declared by nothing) and whether the tip's `memory/adapter.rs`
  is absent (it is absent from `ls-tree`; the module file is all that is left).
- **`project_test.rs`** and the project tests that exercise `Authority::observe`: not read.
- **Tastemaker:** which existing rule would hold "no `std::fs` read of a source file outside outbound"; not looked up.
- **`SourceView`'s new home** (named as leaving the concept): not settled; inbound/lint is a guess.
- **The perf-gain row** for the path selection (section 7): the figures are quoted from `engine/STATE.md` lines 6746 and
  7130 as written; their commits (`2fd0b6d686`, "Prune SDK glob selection with sorted inventory ranges", and the admission
  batch) were not matched figure by figure.
- **`ryo_matching`'s own tests** (5 tests, 2 scenario outlines): which rows are path globs and come here was not sorted.
- **Sheets written beside this one, read only for their lines on corpus.** inbound/lint's listing defines its own
  `ryo_lint::ExecutablePermission` (the SDK's copy, `sdk/src/domain/file/types.rs:46`, beside values' at `types.rs:10`):
  one concept, two definitions; lint should name `ryo_corpus::ExecutablePermission` (`check-types.py` flags it). Lint also
  expects `ryo_corpus::GlobsUseCase` (Appendix A's word); this sheet names it `PathGlobsUseCase` (section 5a), so one of the
  two listings must follow the other. outbound/cache reads `FileVersion { bytes, stamp }` and `Stamp.executable` as drafted
  here, and leaves the stamp-trust rules (`older_than`: both times before the run's start) to "the corpus sheet". This sheet
  does not take them: `Stamp` is data, and whether a stamp may vouch for a later run is a keeping policy, the cache's. Not
  agreed between the two sheets.
- **`check-types.py`** reports `Missing`, `Unreadable`, `Unknown`, `Unsupported`, `TooLarge`, `Listed`, `Walked`,
  `PosixExecutable`, `PosixNotExecutable` as defined twice: these are enum variants (the script counts a variant line as a
  type); only `ExecutablePermission` above is a real double.
