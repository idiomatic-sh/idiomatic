# application/screen
STATUS: drafted (28 Nov 2026; not reviewed, not agreed). Bead ryo-fmx3.1.13.23.4.10.

Revisions: MAIN = f4ae3bd802, TIP = 2a207464c, B133 = f0219b3b3. Every quote is one line, two spaces,
`@rev:path:line`; all were re-checked with `git show "REV:PATH" | sed -n 'Np'` (section 11 gives the count).
"X" below is `crates/ryo/crates/executor/src` at TIP; "CS" is `X/check_session`.
Lines of `api.txt` that name a table type (`TablePort`, `TreeRead`, `Position`, `NameText`) are UNPROVEN until
`domain/table/spike.md` records a pass (that file does not exist on 28 Nov).

## 1. Owns
Which of a file's programs can produce a row in that file, decided before they run.
(One thing. "Can produce" has four answers: no, maybe, only from these carried inputs, and "exactly these nodes"
when the screen can prove the rows itself; the last is decision 2. Running a program, writing its rows, the
cross-file phase and rule applicability are other crates'.)

## 2. On main (f4ae3bd802)
**There is no unit of this name on main, and no screen in the tip's sense** (no per-file test of a rule's necessary
witnesses before evaluation). Three precursors exist, in three crates:
- **A per-operator presence contract, declared and read by nobody.** `execution_domain` states for each operator
  which inputs must have a row in a file for it to answer any row there:
  ```
  pub enum PresenceMask {  @f4ae3bd802:crates/ryo/crates/execution_domain/src/domain/operator/contract/presence/types.rs:12
      pub presence: PresenceMask,  @f4ae3bd802:crates/ryo/crates/execution_domain/src/domain/operator/contract/types.rs:100
  fn presence(kind: OperationKind) -> PresenceMask {  @f4ae3bd802:crates/ryo/crates/execution_domain/src/domain/operator/contract/service.rs:219
          Union => PresenceMask::AnyOf(InputMask(3)),  @f4ae3bd802:crates/ryo/crates/execution_domain/src/domain/operator/contract/service.rs:227
  ```
  3 public items (`PresenceMask`, `InputMask`, `RowFailure`, in `presence/types.rs`, 50 lines). `git grep
  "\bpresence\b"` over `crates/ryo/crates/*.rs` finds the field written at `service.rs:213` and read nowhere: the
  destructuring at `service.rs:334-339` takes `contract`, `effects`, `absence` and drops the rest with `..`. Proof:
  doctests that name the types only; no behaviour test.
- **Emptiness at run time, per plan node** (the ancestor of perf row 38, which is execute's):
  ```
  pub(crate) enum NodeMove {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/file_pass/types.rs:264
      Empty(PlanNodeId),  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/file_pass/types.rs:270
  ```
- **Rule applicability per file** (accepted syntax, and whether a rule governs the path):
  ```
      pub(in crate::application::file_analysis) fn gate_rules(  @f4ae3bd802:crates/ryo/crates/core/src/application/file_analysis/gating/service.rs:90
      fn demanded_rules(&self, governs: impl Fn(RuleIndex) -> Demand) -> Demands;  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/program/prepared_execution/interface.rs:30
  ```
  Proof: one case row, `syntax_applicability_skips_only_gated_rules_and_names_the_reason`
  (`core/src/application/file_analysis/service_test/gating/test.rs:260`), one `#[test]`.
- Who calls them: `gate_rules` and `active_wanted` are called inside `core`'s `file_analysis` only; `NodeMove` inside
  `execution_runtime`'s file pass only; `PresenceMask` by no one.
- Tastemaker: no entry names any of the three.

**Build 133** (cited because the screen existed there and on neither side as a promise of the scan contract): the
`engine` crate's scan said "every program whose anchor occurs in the file runs" (doc of
`engine/src/inbound/use_cases/scan/interface.rs:1`), decided inside the scan by need verdicts and kind/field sets:
```
pub trait ScanUseCase: Send + Sync {  @f0219b3b3:crates/ryo/crates/engine/src/inbound/use_cases/scan/interface.rs:24
pub(crate) enum NeedVerdict {  @f0219b3b3:crates/ryo/crates/engine/src/domain/needs/types.rs:9
pub(crate) struct NodePresence {  @f0219b3b3:crates/ryo/crates/engine/src/domain/scratch/types.rs:330
```
No public screen item there either: it was private to the scan.

## 3. On the perf branch (2a207464c)
**Where.** 19 files of the proof-of-concept crate, **6,882 lines without tests** (counted per file up to its
`#[cfg(test)]`): `X/pair_summary.rs` (1,892) and its nine sub-modules `X/pair_summary/{compact, cost, exact, groups,
paths, patterns, precheck, project, regexes}.rs` (2,276); `X/pair_checks.rs` (1,330 of 1,378) with `counts.rs` (49)
and `witness.rs` (83); `X/pair_skip.rs` (527 of 610); `CS/pairs.rs` (483) and `CS/pairs/atoms.rs` (121);
`X/first.rs` (50); `X/reader_run.rs` (42) and `X/reader_run/pairs.rs` (29). Every module is public from the crate
(`pub mod pair_summary;`  `@2a207464c:crates/ryo/crates/executor/src/lib.rs:18`, likewise `first`, `pair_checks`,
`pair_skip`, `reader_run`, `check_session`).

**Public items: 271** by `^pub (struct|enum|fn|trait|type|const)` and `^    pub (const )?fn ` over those 19 files
(pair_summary.rs 62, pair_checks.rs 55, pair_skip.rs 32, pair_summary/exact.rs 22, cost.rs 21, regexes.rs 12,
CS/pairs.rs 11, atoms.rs 10, patterns.rs 9, compact.rs 8, first.rs 7, precheck.rs 5, project.rs 5, witness.rs 4,
counts.rs 2, paths.rs 2, reader_run.rs 2, groups.rs 1, reader_run/pairs.rs 1). Public fields are not counted.

**Two screens, one after the other, per file.**
1. *The content screen* (`pair_skip`): from each program's plan, clauses of kinds, fields and identifier words that
   must occur in the file; tested against the kinds present in the tree, its fields and its distinct words, before
   any fact is filled. It retires a program outright only when the program reads no provider fact:
   ```
   pub type Clause = Rc<BTreeSet<String>>;  @2a207464c:crates/ryo/crates/executor/src/pair_skip.rs:7
   pub struct BoundSummary {  @2a207464c:crates/ryo/crates/executor/src/pair_skip.rs:308
       pub fn bind(  @2a207464c:crates/ryo/crates/executor/src/pair_skip.rs:316
   pub struct FileSummary {  @2a207464c:crates/ryo/crates/executor/src/pair_skip.rs:423
       pub fn excludes(  @2a207464c:crates/ryo/crates/executor/src/pair_skip.rs:446
   pub fn file_requirements(  @2a207464c:crates/ryo/crates/executor/src/reader_run.rs:12
   ```
2. *The witness screen* (`pair_summary`, `pair_checks`, `CS/pairs`): an abstract run of every program's plan steps
   over witness domains (`Domain`, 28 variants) and needs (`Need`, 12 variants), compiled once into one program per
   set of programs, then evaluated per file over the tree and the facts it asks for; for some outputs it proves the
   exact nodes (a "certificate") and the rule is answered without evaluation:
   ```
   pub enum Domain {  @2a207464c:crates/ryo/crates/executor/src/pair_summary.rs:41
   pub enum Need {  @2a207464c:crates/ryo/crates/executor/src/pair_summary.rs:72
   pub struct Program {  @2a207464c:crates/ryo/crates/executor/src/pair_summary.rs:109
       pub scratch: std::cell::RefCell<super::pair_checks::Scratch>,  @2a207464c:crates/ryo/crates/executor/src/pair_summary.rs:110
   pub struct Builder<'a> {  @2a207464c:crates/ryo/crates/executor/src/pair_summary.rs:240
       pub fn step<'a>(  @2a207464c:crates/ryo/crates/executor/src/pair_summary.rs:1335
   pub fn prepare_plans(  @2a207464c:crates/ryo/crates/executor/src/pair_summary.rs:1701
   pub struct Certificate {  @2a207464c:crates/ryo/crates/executor/src/pair_summary/exact.rs:38
   pub enum LocationColumn {  @2a207464c:crates/ryo/crates/executor/src/pair_summary/exact.rs:46
       pub fn selected_columns(  @2a207464c:crates/ryo/crates/executor/src/pair_checks.rs:1183
   pub struct Checks {  @2a207464c:crates/ryo/crates/executor/src/check_session/pairs.rs:13
       pub fn from_roots<L: FileSource>(  @2a207464c:crates/ryo/crates/executor/src/check_session/pairs.rs:35
       pub fn exclude(  @2a207464c:crates/ryo/crates/executor/src/check_session/pairs.rs:168
       pub fn asked_first(&self) -> Vec<l::Relation> {  @2a207464c:crates/ryo/crates/executor/src/check_session/pairs.rs:353
       pub fn exact_locations(  @2a207464c:crates/ryo/crates/executor/src/check_session/pairs.rs:371
   pub struct Retained {  @2a207464c:crates/ryo/crates/executor/src/check_session/pairs.rs:427
       pub fn merged<L: FileSource>(  @2a207464c:crates/ryo/crates/executor/src/check_session/pairs.rs:437
   ```
   The order of a screen's tests is a fixed price per test family, applied when the program is compiled:
   ```
       pub fn price(self) -> u8 {  @2a207464c:crates/ryo/crates/executor/src/pair_summary/cost.rs:39
   pub fn arrange(p: &mut Program, omit: u16) {  @2a207464c:crates/ryo/crates/executor/src/pair_summary/cost.rs:207
   ```

**What the language tells it** (from the driver's language handle, turned into the screen's own two types):
```
pub struct Vocab {  @2a207464c:crates/ryo/crates/executor/src/first.rs:7
pub struct Language {  @2a207464c:crates/ryo/crates/executor/src/first.rs:22
pub fn reader_declaration<L: FileSource>(parser: Option<&L>) -> first::Language {  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:72
pub struct SourceWords {  @2a207464c:crates/ryo/crates/language/src/inbound/use_cases/parse/named.rs:19
```
Two relation names are compared as text inside the screen:
`pub const SYNTAX: [&str; 2] = ["semantic-syntax", "syntax-tree"];`  `@2a207464c:crates/ryo/crates/executor/src/first.rs:4`
(and `CHILD` at `:5`); they are catalog names, not a language's.

**Who calls it.** Only the driver `visit`, in `X/check_session.rs`, at lines 907 (`merged`), 962
(`file_requirements`), 966 (`BoundSummary::bind`), 1175 (`FileSummary::excludes`), 1211 (`asked_first`), 1286
(`Checks::exclude`), 1290 (`exact_roots`), 1302 and 1305 (`exact_locations` into `publish_locations`):
```
    let (scope, screen, positions) = state.screens.merged(parser, programs, program_of, &needed, cut_of);  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:907
                        reader_run::file_requirements(&program.typed, &language)  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:962
                    let requirements = Rc::new(BoundRequirements::bind(&clauses, &mut prerequisite_clauses, |word| run.sym(word)));  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:966
                    && contents.excludes(b.test, &b.requirements, |out| {  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:1175
                if excluded && b.facts.is_empty() && b.named.is_empty() && !unknown_permission {  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:1187
                let asked = screen.asked_first();  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:1211
                        if let Some(locations) = screen.exact_locations(positions[binding]) {  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:1302
                                |(emit, ((rule, _, subject, keys), (columns, mask)))| finish::session::publish_locations(  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:1305
```
Outside the screening files, three more files name it: `X/finish/session.rs` and `X/finish/answer.rs` (the location
columns, to write a certified answer) and `X/lib.rs`:
```
pub fn publish_locations(  @2a207464c:crates/ryo/crates/executor/src/finish/session.rs:546
```
No file of the SDK, the benchmark crate or the lint images names a screening module (`git grep` at TIP).

**What it reaches back into** (why it cannot be lifted out as it is): the driver's `Program` (`plan.emits`,
`typed.steps`), `Partition` (`inputs`, `domains`, `reach`), the per-binding `Bound` (`facts`, `absent`,
`projections`, written by the screen at `CS/pairs.rs:336-346`), `project::Layout` (`relations`,
`witness_columns`), `Run` (`texts`, `ids`, `absent_word`, `kind_sym`, `field_sym`, `empty`,
`supplied_permission`), `host_columns::fill`, `finish::NULL`, and the packet's `has_rows`, `any_symbol`, `columns`.
`Checks::exclude` takes 23 parameters besides `self` (the plan's "25" is two too many). A cross-file program's
partial absence is written straight into the driver's binding:
```
                let (home, absent) = b.absent.get_or_insert_with(|| (id.0, Vec::new()));  @2a207464c:crates/ryo/crates/executor/src/check_session/pairs.rs:336
```
and read by the cross-file phase (`CS/frontier.rs:93`).

**How often.** Prepared once per worker shard and per set of admitted programs (`Retained::merged`, kept in
`ReaderState`; `dispatch` gives every shard a fresh `ReaderState`, so ten workers prepare it ten times); a program's
file requirements once per program and reader, kept on the program behind a `Mutex`:
```
    pub requirements: std::sync::Mutex<Vec<(Option<usize>, std::sync::Arc<Requirements>)>>,  @2a207464c:crates/ryo/crates/executor/src/check_session.rs:65
```
Per file: one content screen per (program, file) pair, memoised per bound clause; one witness screen run per file
over all its pairs.

**Proof on the tip.** 7 `#[test]`, all unit tests inside the files:
```
    fn a_missing_shape_leaves_word_projection_unasked() {  @2a207464c:crates/ryo/crates/executor/src/pair_skip.rs:533
    fn incomplete_alternatives_cannot_prove_absence() {  @2a207464c:crates/ryo/crates/executor/src/pair_skip.rs:556
    fn programs_share_a_clause_and_reset_its_answer_for_a_new_version() {  @2a207464c:crates/ryo/crates/executor/src/pair_skip.rs:580
    fn projection_keeps_codes_outside_the_dense_prefix() {  @2a207464c:crates/ryo/crates/executor/src/pair_skip.rs:597
    fn a_partial_read_leaves_no_word_or_row_in_the_next_file() {  @2a207464c:crates/ryo/crates/executor/src/pair_checks.rs:1336
    fn absent_symbol_matches_the_empty_literal_and_keeps_its_identity() {  @2a207464c:crates/ryo/crates/executor/src/pair_checks.rs:1348
    fn reserved_reason_has_a_spelling_but_does_not_equal_a_literal_with_that_spelling() {  @2a207464c:crates/ryo/crates/executor/src/pair_checks.rs:1361
```
No test in `pair_summary` (4,168 lines with its sub-modules), `CS/pairs.rs`, `atoms.rs` or `first.rs`. No acceptance
scenario. The real proof is indirect: the linter parity suites (the same findings with the screen as without), run
through the lint images.

**Tastemaker.** No entry names screening. The executor crate's entries cover it: `crate-executor`
(`crates/ryo/tastemaker.toml:105`) and the two allowlists at `:31` and `:32`.

## 4. What changed between them
- **Added on the tip: the whole concept** (6,882 lines, 271 public items). It came in with the vector engine,
  which lived in `crates/engine/benches/vec` (first commits on these paths: `21fb355070`, `a03d81f133`, 23 Nov,
  "Separate the vector planner and executor from the Python host") and was moved to `crates/ryo/crates/executor/src`
  by `f2869d1f74` (26 Nov, "renames only"). `git log` on the screening paths, both homes: **120 commits** from 23 to
  27 Nov. The ones that shaped the interface:
  - `aeef47577d` (24 Nov) "Bind shared literal globs and certified pair decisions on the typed plan" and
    `cabb17e1ad` "Use one pair evaluator for bound roots and certify their exact locations": the certificate path.
  - `dca7507469` (24 Nov) "Reuse file prerequisite clauses across programs and read the existing kind bitmap".
  - `909735c97a` (25 Nov) "Remove the empty-answer verifier from pair screening" (perf row 11).
  - `d6de8581df` (25 Nov) "Prepare one merged file screen per admitted program set and visit its roots once".
  - `3be7fb5185` (25 Nov) "Use the numbered field lists and share one distinct-word scan between both screens".
  - `4b6cf1c93b` (25 Nov) "Fill a relation the first time a witness or a step loads it for the file" (row 12), after
    `1cfebbdfe3` "Fill a file's relations in two steps: the screen's before it runs, the rest for the survivors" was
    tried and reverted (`d0b8ea76b6`).
  - `9b9ccf7e3b` (25 Nov) "Screen the project-layout and executable-permission witnesses on the file's own rows":
    the reach into `Layout` and `host_columns`.
  - `b5804c7789` (27 Nov) removed unread counters and debug audits from these files.
- **Removed:** main's per-operator presence contract went with `c46607e439` (27 Nov, "the old operator contract
  ... go"); `NodeMove` with `861c7de10e` (26 Nov, "the old executor goes"); `gate_rules` with `ff4533b99a` (26 Nov,
  "the old engine arms are deleted"). The tip does the applicability gate in the driver
  (`X/check_session.rs:1161-1171`) and per-step emptiness in `X/finish/{emptyprop, known}.rs`.
- **Changed:** nothing carried over; the tip's screen is new code beside the deleted precursors, not a change of them.
- **Lost that should not have been:** `PresenceMask` / `InputMask` / `RowFailure`: a designed per-operator contract
  ("Which inputs must have a row in a file for an operation to answer any row there") deleted because nothing read
  it, while the tip re-derived the same knowledge a third time in `Builder::step` (`pair_summary.rs:1335-1622`).
  Decision 1 gives it a home.

## 5. The drafted public interface        (explains every line of api.txt that is changed or new)
`api.txt` beside this sheet: **29 public items** (2 traits, 10 trait methods, 2 associated types, 7 structs,
3 enums, 5 inherent functions) plus 10 variant lines and 3 impl lines. Main: 0 items of the concept (3 precursor
items in `execution_domain`, none read). Tip: 271.

**Corrections to the claim it starts from** (plan Appendix A 3.8, `PLAN.md:1261-1275`):
- `screen(&self, screens, file: FileFacts<'_>, ...)` cannot work: the tip's screen needs more than `FileFacts`
  (`pub struct FileFacts<'f> {`  `@2a207464c:crates/ryo/crates/language/src/domain/facts/types.rs:35`), namely
  which reads were partial, the host's own rows (executable permission), a project layout's rows, the completeness
  of the numbered projections, a "has rows" probe that does not fill, and the file's distinct words as run symbols.
  The draft replaces `FileFacts` with a port the caller implements, `ScreenFilePort`, crossed per relation, not per
  cell. `FileFacts` also carries `&mut dyn FactSource`, a `dyn` the owner's rule excludes from the hot path.
- `Verdict { Skip, Run, Answered(Located) }` lacks the cross-file case: for a program whose output crosses files the
  tip records which carried inputs this file cannot contribute (`CS/pairs.rs:335-346`). Added: `Verdict::Absent`.
- `Answered(Located)` as a payload allocates per pair; the tip borrows the mask from the screen's state. The draft
  keeps the variant bare and reads the nodes from `ScreenScratch::located`.
- `prepare(&self, programs, vocabulary: &Vocabulary)`: the tip needs the language's kinds, fields, source words and
  relation availability, and the run's symbols (to bind clause words once). Drafted as
  `prepare(&Programs, &ScreenLanguage<'_>, &FrozenSymbols)`.
- Added `Screens::asked_first`: the relations to fill in one provider call before the screen runs (perf row 24,
  one provider call per file), computed once at prepare instead of per file.

**The calls.**
```
pub trait ScreenUseCase: Send + Sync {                                                                     @new
    fn prepare(&self, programs: &Programs, language: &ScreenLanguage<'_>, symbols: &FrozenSymbols) -> Screens;
    fn screen<S: TablePort, F: ScreenFilePort<S>>(&self, screens: &Screens, programs: &[SelectedRuleOrdinal],
                                                  file: &F, scratch: &mut ScreenScratch, verdicts: &mut [Verdict]);
}
```
- `prepare` is, in one call and once per language when rules load: `vocabulary` + `reader_declaration` (the
  language's words), `Checks::from_roots` -> `prepare_plans` (the witness program over every program's plan steps,
  shared by step identity), `cost::arrange` (test order by fixed price), `file_requirements` +
  `BoundSummary::bind` (the content clauses, words bound to run symbols through `FrozenSymbols` instead of
  `run.sym` per worker) and `Atoms::bind`. It reads each program's plan and cut from `Programs` (a need on the
  programs sheet: `Programs` must lend a program's plan and its cut). Infallible, as `prepare_plans` is today.
- `screen` is, per file: `FileSummary::excludes` for every pair, then `Checks::exclude`, then `exact_roots` /
  `exact_locations`. `programs` names the file's pairs (the tip's `active`, `(binding, slot, excluded)`), and
  `verdicts[i]` answers `programs[i]`. The caller allocates `verdicts` once per worker.

**What must be handed in: the file, per file.**
```
pub trait ScreenFilePort<S: TablePort> {                                                                   @new
    type Tree: TreeRead;
    type Symbols: SymbolsUseCase;
    fn tree(&self) -> &Self::Tree;                               // was `tree: &l::NodeTable`
    fn source(&self) -> &[u8];                                   // was `bytes: &[u8]` (SourceContains)
    fn path(&self) -> &str;                                      // was `path: &str` (path globs, captured paths)
    fn words(&self) -> Option<&[Symbol]>;                        // was `text: Option<&[u32]>` + `FileSymbols`
    fn symbols(&self) -> &Self::Symbols;                         // was `run.texts`, `run.ids`, `unknown`
    fn complete(&self) -> Completeness;                          // was the `Demand` at check_session.rs:1179-1183
    fn rows_known(&self, relation: Relation) -> Option<bool>;    // was `packet.has_rows` (row guards)
    fn facts(&self, relation: Relation) -> Facts<S::Reader<'_>>; // was the `read` closure + `packet.any_symbol`
}
```
Execute implements it. `facts` fills a relation the first time the screen asks (row 12) and answers the host's
rows and a project layout's rows itself, so the screen no longer knows `host_columns`, `Layout` or
`supplied_permission`. `Facts::Unknown` is a relation read in part or not answered (the tip's
`partials.iter().any(..)` checks): the screen never excludes on it. All calls are per relation or per file; the
cells are read through the store's `TableRead` (`column_into`), never by a call per cell.

**What comes out.**
```
pub enum Verdict { Skip, Run, Absent, Answered }                                                           @new
```
- `Skip`: no row possible; execute answers the pair empty (today `answers.set_empty` or `excluded = true`).
- `Run`: evaluate it.
- `Absent`: a cross-file program; `ScreenScratch::absent(rule)` says, per carried input, whether this file has none
  (today `b.absent`, read by `CS/frontier.rs:93`). Every input absent is `Skip`.
- `Answered`: the screen proved the rows; `ScreenScratch::located(rule)` gives, per emit, the location columns and the
  node set, which execute writes through today's `publish_locations`. Execute may still treat it as `Run` when the
  binding has a project layout (today's extra condition `!b.layout.is_empty()` at `X/check_session.rs:1301`).
- `LocationColumn { File, Span, Node }` is the tip's type verbatim. `NodeSet<'a>` is new: an opaque borrowed set
  of positions with `iter` and `is_empty`, replacing the tip's `&[u64]` bit mask and `pair_checks::each`, so no
  layout word crosses.

**The types.** `ScreenLanguage<'a> { kinds, fields, words, relations }` and `Offered { numbered, expands,
available, absent }` replace `first::Vocab` and `first::Language`, filled by the caller from the language contract
when rules load (today `Source::{kinds, fields, source_words, numbered_projections, expands, available, absent}`).
`Completeness { kinds, fields, words }` is the tip's `pair_skip::Demand` in the meaning the driver gives it at
`check_session.rs:1179-1183` (which numbered projections the provider certified complete). `Facts<R>` is new.

**How it is built.** `Screen`, a unit struct with `Default`, implements `ScreenUseCase`; nothing is handed in at
construction (the per-operator screening meaning comes from `domain/operators` at compile time, decision 1).
`Screens` is `Send + Sync` and built once per language per request set; `ScreenScratch` (`Default`) is one per
worker: it takes everything mutable out of the tip's `Program` (`RefCell<Scratch>`), `WitnessTables`,
`LiteralPositions`, `AtomAnswers`, `FileSymbols`, `SymbolMatches` (the per-symbol pattern answers) and
`FileSummary`.

**What stops being public.** All 271 tip items (listed by module, with examples, in api.txt `# NO LONGER PUBLIC`):
the screen's algebra (`Domain`, `Need`, `CellTest`, `Builder` and its 32 methods), its compiled form
(`pair_summary::Program`, `Certificate`, `Fact`, `Summary`), the per-file machinery (`pair_checks::*`,
`FileSummary`, `Projection`, `Clauses`), the driver-facing types (`Checks`, `Retained`, `Atoms`) and `first::*`.
On main, the three `presence` items leave `execution_domain` for `domain/operators` (decision 1), not for this crate.

| Side | Public items in the concept | After the draft |
|---|---|---|
| main `f4ae3bd802` | 0 (3 precursors in `execution_domain`, unread) | precursors to operators |
| tip `2a207464c` | 271 (19 files, 6,882 lines without tests) | all private |
| draft | **29** (+10 variants, +3 impls) | |

## 6. Proof it brings
| Proof | Source | Fate |
|---|---|---|
| the 4 content-screen tests (`a_missing_shape_leaves_word_projection_unasked`, `incomplete_alternatives_cannot_prove_absence`, `programs_share_a_clause_and_reset_its_answer_for_a_new_version`, `projection_keeps_codes_outside_the_dense_prefix`) | `X/pair_skip.rs:533-597` | **carried** as unit tests inside the crate; the last builds a `NodeTable` and is restated over `TreeWrite` |
| `a_partial_read_leaves_no_word_or_row_in_the_next_file` | `X/pair_checks.rs:1336` | **carried** (witness tables across files) |
| `reserved_reason_has_a_spelling_but_does_not_equal_a_literal_with_that_spelling` | `X/pair_checks.rs:1361` | **carried**, restated over `SymbolsUseCase::reserve` |
| `absent_symbol_matches_the_empty_literal_and_keeps_its_identity` | `X/pair_checks.rs:1348` | the symbols sheet restates its dictionary half; the witness half is **carried** here |
| main's `syntax_applicability_skips_only_gated_rules_and_names_the_reason` | `core/.../service_test/gating/test.rs:260` | not this crate's: applicability stays in execute (decision 5) |
| the screen never changes a finding: for every case of the linter suites, findings with the screen equal findings with every verdict forced to `Run` | none | **MISSING**. Today only the whole-run parity covers it, and there is no switch that turns the screen off (`VEC_PAIR_FULL` only stops omitting test families) |
| a `Verdict::Answered` case table: the certified rows equal the evaluated rows | none | **MISSING** (the certificate has no test on the tip) |
| a `ScreenFilePort` contract test: `Facts::Unknown` never yields `Skip` | none | **MISSING**; the tip's partial-read guards are untested |
| `Screens: Send + Sync`, built once, read by many workers | none | **MISSING**: a compile-time assertion plus a two-worker case |
| the witness compiler (`pair_summary.rs` and its sub-modules, 4,168 lines: `prepare_plans`, `Builder::step`, `exact::prepare`) | none | **MISSING** on both sides: no test at all today |

## 7. Perf gains it carries
Every row of `perf-gains.tsv` whose crate column names `application/screen` (10 rows), and what in the draft keeps it:

| Row | What | Measured (from the tsv) | State | What in the draft keeps it |
|---|---|---|---|---|
| 11 | empty-answer verifier removed from screening | pairs equal to the old binaries; CPU not reported | live | `Verdict` is final: there is no verify call on `ScreenUseCase`, and execute does not re-check a `Skip` after evaluation |
| 19 | pair screen across the whole session | 22.34 -> 17.04 CPU-s; pairs 557,238 -> 199,111 | live | `prepare` is called once per language per request set and returns `Screens: Send + Sync`; one value read by every worker and every file (stronger than today: the tip prepares per worker shard) |
| 21 | literal glob compiled once, answer reused per interned subject; visible to the screen | 57.226 -> 36.237 CPU-s | live | `prepare` takes `&FrozenSymbols`, so clause words and literal patterns are bound to `Symbol`s once; `ScreenFilePort::words` gives the file's words as `Symbol`s; `ScreenScratch` keeps the per-`Symbol` pattern answers (today's `SymbolMatches`). The bound predicate itself is operators' (`FunctionsItem`, Appendix A 3.4) |
| 26 | exact keyed probes; frozen emptiness bit; entry tests cached | probes +4.00% dagster | lost | nothing to keep; the screen keeps deciding from what the producer reports at run time: `rows_known` and `facts` |
| 28 | pair decisions for TypeScript and Rust | hono 19,716 -> 4,196 pairs; 1.176 -> 1.021 | live | `ScreenLanguage` is data from the language contract; no language-specific path; `first::SYNTAX`/`CHILD` are catalog names read through `ryo_relations` |
| 29 | first-half tests ordered by cost | Django 9.93 -> 8.91 | live | the order is fixed inside `Screens` at `prepare` (`cost::arrange`, a fixed price per family); `screen` never looks a cost up |
| 35 | step summaries decide pairs | Django 11.60 -> 9.10; preparation 1.0-1.4 -> 0.045-0.05 CPU-s | live | `prepare` reads each program's plan steps through `Programs` (shared by step identity, one witness program per set); with decision 1, each step's screening meaning comes from the one operator definition, not a second description |
| 38 | empty-run propagation | Django 16.28 -> 14.34 | live | **execute's, not this crate's** (`X/finish/emptyprop.rs`, `known.rs`); `Verdict::Run` puts no constraint on it |
| 46 | engine in `crates/engine`: needs, skip terms | Django 36.0 -> 19.9 | lost | B133's scan contract promised "every program whose anchor occurs in the file runs"; that promise is this crate's `screen` |
| 49 | requirement cache per prepared program and reader | requirement programs 5,110 -> 552 | live | the content clauses are computed inside `prepare`, once per program and language, and live in `Screens`; no `Mutex` on the program |

Also carried, though the tsv assigns it elsewhere: row 12 (fill on first use) needs `ScreenFilePort::facts` to fill
on the first ask, and row 24 (one provider call per file) needs `Screens::asked_first`. Gains to make room for: row
13 (parked, many workers sharing frozen state) needs `Screens: Send + Sync` and the screen's words bound to
`FrozenSymbols`, both drafted.

## 8. Benchmarks that cover it
- **The lint images** (`vec`, `vec_ts`, `vec_rust`, `tm`; `crates/ryo/crates/executor/benches/lint_image`) run
  the screen end to end through the SDK and name nothing here. The "switched off" figures (+30% CPU on fastapi and
  dagster, +14% on the check; screening 9 to 13% of CPU) are recorded in `discovery-inputs/review-performance.md:24`;
  how the screen was switched off for them is not checked.
- **Counters.** The driver's `Clock` (`screen`, `bound_proofs`, `bulk_screen_files`, `exact_inputs`, `skipped`,
  `certified`, `literal_positions`, `requirement_programs`) is printed as one `CHECK_SESSION` line
  (`CS/replay.rs:278`) and read by scripts in the evidence repository (`build/redo-lowering/scripts/pay-screen.sh`,
  `pair-stages.sh`, `pay-probes.sh` and others found by `grep -l CHECK_SESSION`). After the draft execute keeps the
  clock: it times the `screen` call and counts verdicts by kind, so the printed names can stay.
- **Diagnostics** behind the `pair-diagnostics` feature (`crates/ryo/crates/executor/Cargo.toml:50`) and the
  environment variables `VEC_PAIR_METER`, `VEC_PAIR_TRACE`, `VEC_PAIR_OMIT`, `VEC_PAIR_FULL`, `VEC_PAY_PREP`
  (`pair_summary.rs:1711`, `:1772`, `:1877`; `cost.rs:244`): they move with the code as a feature of this crate and
  are read inside it; nothing public is needed for them.
- **Parity and pair counts** in the landing gate (step 4, `build/ports/landing/landing-gate.sh:18-19`, timing
  pairs) and the parity counts in the check-in table measure the screen's effect but name no type.
- No micro-benchmark of the screen exists on either side; none is proposed (the images are the measure).

## 9. Checks
IMPORTS: table, symbols, relations, operators, plan, programs, language
CROSSED: per file
- Directions: application may import domain and outbound. `table`, `symbols`, `relations`, `operators`, `plan`,
  `programs` are domain; `language` is outbound. It imports neither `execute` nor `project` (screen < execute <
  project). In the public signatures only `table`, `symbols`, `relations`, `programs` and `language` appear;
  `operators` and `plan` are used inside (each operator's screening meaning; the plan's step types).
- Layout words: none in a signature. The tip's `&[u64]` masks (`exact_locations`, `selected`), `Rc<Vec<u64>>`
  (`Bits`), `FactColumn` columns and the `NodeTable` type are absent; nodes cross as `NodeSet` and `Position`, cells
  through the store's reader.
- Crossing: `prepare` per language per request set; `screen` per file (all pairs of the file in one call); the port
  per relation per file. The tree is read through `TreeRead`: set calls (`ids_of_kind`, `ids_of_field`) dominate,
  but the witness walks call `parent_of` / `children_of` per node, as every tree kernel does; the type is fixed at
  compile time (`F: ScreenFilePort<S>`, `S: TablePort`), so no call is virtual.
- `TreeRead` as drafted on the table sheet lacks four reads the screen makes today: `kinds_present` (one set of the
  kinds in the file, `pair_skip.rs:390`), `decoded_of`, `embedded_segments`, and `starts`/`ends` (`first.rs:49`;
  `span_of` per node can replace this one). A need on the table sheet.

## 10. Decisions for the owner              (each: the example from the code; why; what we get; my recommendation)
1. **Where each operator's screening meaning lives.** Example: `Builder::step` (`X/pair_summary.rs:1335-1622`)
   switches over every plan step a third time (after `Exec::step` and `Exec::schema`); main had the coarse form as a
   designed contract, `PresenceMask` (`AllOf`, `AnyOf`, `Always` per operator, `execution_domain/.../service.rs:219`),
   read by nobody and deleted at `c46607e439`. Why: one definition per operator; the operators crate (domain) cannot
   import the screen (application), so the description must be an operators type the screen interprets. What we
   get: a new operator is screened by declaring its meaning in its own folder. Options: (a) the screen keeps its own
   switch; (b) operators exports main's `PresenceMask` only (loses the column and text tracking that rows 21 and 35
   rely on); (c) operators exports a `Screening` description per operator, seeded by main's `PresenceMask`,
   `AbsenceMask` and `RowFailure`, and the screen keeps its witness algebra (`Domain`, `Need`) private.
   **Recommend (c).**
2. **Whether the screen may answer a rule.** Example: `exact_locations` (`CS/pairs.rs:371`) feeds
   `publish_locations` (`X/check_session.rs:1305`), so a rule is answered from the tree without evaluation. Why: with
   it, "Owns" reads "can produce a row, and which"; without it the screen is a pure filter. What we get by keeping
   it: the certified pairs are never evaluated (the rows trace counts 387 of 7,379 pairs certified on one run,
   `discovery-inputs/rows-trace.md:90`); the cost is a second way to make rows, with no test today. **Recommend:
   keep `Verdict::Answered`, the screen gives the nodes and execute writes the rows; add the missing case table
   (section 6) before the build.**
3. **Prepared once and shared by every worker.** Example: `Retained` lives in each shard's `ReaderState`
   (`X/check_session.rs:846`) and `Program` holds a `RefCell` (`pair_summary.rs:110`), so it is neither `Send` nor
   shared; clause words are bound by `run.sym` per worker (`X/check_session.rs:966`). Why: parked row 13 (many
   workers) failed on per-worker state; row 35 measured preparation at 0.045 to 0.05 CPU-s per set, paid again per
   shard. What we get: one `Screens` for the request, words bound to `FrozenSymbols` (symbols sheet, decision 2,
   option B). **Recommend yes.**
4. **One call per file, with fills through the port.** Example: today two screens run with the fill between them
   (`FileSummary::excludes` at `:1175`, the fill at `:1223`, `Checks::exclude` at `:1286`), and a two-step fill was
   tried and reverted (`1cfebbdfe3`, `d0b8ea76b6`). Why: two calls would make execute own the order of the screen's
   own steps. What we get: execute fills `Screens::asked_first` in one provider call, then calls `screen` once; any
   other relation is filled when the screen first asks for it through `facts`. **Recommend one call.**
5. **Rule applicability stays out.** Example: the tip skips accepted-syntax rules on a rejected file in the driver
   (`X/check_session.rs:1161-1171`) and reports them as skipped with a reason; main did it in `core`
   (`gate_rules`, `gating/service.rs:90`). Why: it is a property of the rule and the file's parse, reported to the
   user ("skipped: syntax rejected"), not a proof about rows. What we get: `Verdict` stays about rows; execute keeps
   the gate and its report. **Recommend: execute owns applicability.**

## 11. Not checked
- Quotes: all 61 `@rev:path:line` quotes in this sheet were re-checked by script against `git show "REV:PATH" |
  sed -n 'Np'` (exact line text once the code block's own indentation is removed): 61 match, 0 differ. Prose references written `path:line`
  without `@` (for example `CS/frontier.rs:93`, `tastemaker.toml:105`, `review-performance.md:24`) were checked by
  hand one at a time, except the evidence-repository script names in section 8, which were found by `grep -l` only.
- The bodies of `pair_summary/exact.rs` (1,061 lines), `pair_checks.rs` lines 400-1160 (the witness evaluator) and
  `Builder::step` were not read line by line. So whether every `Need` and `Domain` variant can be driven by a
  per-operator `Screening` description (decision 1, option c) is a claim, not a finding.
- The perf rows are quoted from `perf-gains.tsv`; no measurement was taken. Whether sharing one `Screens` across
  workers keeps the tip's speed (the per-worker `SymbolMatches` cache becomes per worker over shared symbols) is
  not measured.
- Whether `Programs` (programs sheet, not written yet) will lend a program's plan and cut, and the exact name of a
  program's identity there (`SelectedRuleOrdinal` here, from Appendix A 3.7), are assumptions; `ryo_programs::*`
  lines in api.txt are placeholders for that sheet's names. Likewise `ryo_language::SourceWords` for the language
  sheet and `FunctionsItem` for the operators sheet.
- The binding vs program distinction: the tip keys a screen by (program, target steps, cut domains) per binding
  (`CS/pairs.rs:448-453`); hosted relations make two bindings of one program. Whether `SelectedRuleOrdinal` is
  per binding or per program is not settled.
- `reader_run/packet.rs` (689 lines, 4 tests) was placed with execute (it is the file's fact packet; the screen uses
  only `has_rows`, `any_symbol`, `columns`); not read in full.
- Lines with tests of the main precursors were not counted; whether anything on main reads `PresenceMask` through a
  path `git grep "\bpresence\b"` cannot see (a macro) is not checked.
- How the "+30% switched off" figure was produced, and whether a build switch exists for it, is not checked.
- No `tm check` was run on a skeleton; whether `Option<&[Symbol]>` and the bare `bool` fields of `Completeness` and
  `Offered` pass the nominal-type rules is not checked.
- The four missing tree reads (section 9) were found by `grep -o 'node_table::[a-z_]+'` over the screening files;
  calls through other paths were not looked for.
