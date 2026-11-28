# domain/operators
STATUS: drafted (28 Nov 2026; bead ryo-fmx3.1.13.23.4.6; not reviewed, not agreed)

Revisions: MAIN = `f4ae3bd802` (release 0.2.0, live). TIP = `2a207464c` (perf branch, build 139). B133 = `f0219b3b3`.
Every quote is one line of the repository at that revision, then two spaces and `@rev:path:line`. Read through
`git -C /workspaces/idiomatic-public-benchmark show|grep|ls-tree|log|diff` only. Lines naming a `ryo_table` type are
UNPROVEN until `domain/table/spike.md` records a pass.

## 1. Owns
What each query operator means: its name, its typed arguments, its output columns, what it needs of its inputs, and
the kernel that computes it, together with the expressions and functions a `where` or `extend` holds.
(One thing: "what an operator call means". The functions are inside it because a function means nothing outside the
expression of an operator call; the owner chose "functions in operators", 28 Nov.)

## 2. On main (f4ae3bd802)
**Where.** The crate `crates/ryo/crates/operators` (package `idiomatic-sh-ryo-operators`, library `ryo_operators`):
461 files, 39,729 Rust lines with tests, 18,599 outside `*_test*`, `tests/`, `fixtures` and `cases.rs` files (by
file name), 166 `#[test]`. It depends on `language`, `matching`, `derived`, `relations`, `values` (its `Cargo.toml`).
The operators it means are values' typed plan enum, one dispatch to a kernel lowering:
```
pub enum Operation {  @f4ae3bd802:crates/ryo/crates/values/src/domain/plans/types.rs:65
        Operation::Select {  @f4ae3bd802:crates/ryo/crates/operators/src/domain/physical_plan/service/lowering/service.rs:53
pub enum QueryOperator {  @f4ae3bd802:crates/ryo/crates/values/src/domain/query_operators/types.rs:24
```
`Operation` has 27 variants, `QueryOperator` 26, `OperationKind` 27 (the 27 are the 26 plus `Hosted`). There is no
text dispatch on main: `lowering/service.rs` (323 lines) matches `Operation` once into 18 `Lowering` kinds.

**The contract, word for word.** Two use cases, unchanged at B133 (`git diff f4ae3bd802 f0219b3b3` on the four
contract files is empty):
```
pub trait PlanUseCase: Sync {  @f4ae3bd802:crates/ryo/crates/operators/src/inbound/use_cases/plan/interface.rs:39
    fn plan(&self, program: &LogicalProgram<'_>) -> Result<PhysicalPlan, PortableOperatorFault>;  @f4ae3bd802:crates/ryo/crates/operators/src/inbound/use_cases/plan/interface.rs:45
pub trait RunUseCase: Sync {  @f4ae3bd802:crates/ryo/crates/operators/src/inbound/use_cases/run/interface.rs:32
    fn run(  @f4ae3bd802:crates/ryo/crates/operators/src/inbound/use_cases/run/interface.rs:39
        plan: &PhysicalPlan,  @f4ae3bd802:crates/ryo/crates/operators/src/inbound/use_cases/run/interface.rs:41
        node: PlanNodeId,  @f4ae3bd802:crates/ryo/crates/operators/src/inbound/use_cases/run/interface.rs:42
        buffers: &mut FileBuffers,  @f4ae3bd802:crates/ryo/crates/operators/src/inbound/use_cases/run/interface.rs:43
        inputs: &dyn PlanInputs,  @f4ae3bd802:crates/ryo/crates/operators/src/inbound/use_cases/run/interface.rs:44
        resources: &mut dyn Resources,  @f4ae3bd802:crates/ryo/crates/operators/src/inbound/use_cases/run/interface.rs:45
    ) -> Result<(), PortableOperatorFault>;  @f4ae3bd802:crates/ryo/crates/operators/src/inbound/use_cases/run/interface.rs:46
```
The doc of `run`: "Makes one node's rows for the open file, running only the pipelines it needs; its rows are then in
`buffers`." (`run/interface.rs:7`). So main's `run` is already a pull: the caller names a node, `run` makes only what
that node reads. What the caller hands in:
```
pub trait PlanInputs {  @f4ae3bd802:crates/ryo/crates/operators/src/domain/physical_plan/types.rs:128
    fn facts(&self, relation: Relation) -> FactRows<'_>;  @f4ae3bd802:crates/ryo/crates/operators/src/domain/physical_plan/types.rs:130
    fn roots(&self, root: PlanNodeId) -> Candidates<'_>;  @f4ae3bd802:crates/ryo/crates/operators/src/domain/physical_plan/types.rs:132
    fn frontier(&self, place: FrontierPlace) -> HeldRows<'_>;  @f4ae3bd802:crates/ryo/crates/operators/src/domain/physical_plan/types.rs:134
    fn observation(&self, place: BoundObservationPlace) -> HeldRows<'_>;  @f4ae3bd802:crates/ryo/crates/operators/src/domain/physical_plan/types.rs:136
    fn hosted(&self, relation: HostedNumber) -> HeldRows<'_>;  @f4ae3bd802:crates/ryo/crates/operators/src/domain/physical_plan/types.rs:138
pub trait Resources {  @f4ae3bd802:crates/ryo/crates/operators/src/domain/physical_plan/types.rs:142
    fn texts(&self) -> Texts<'_>;  @f4ae3bd802:crates/ryo/crates/operators/src/domain/physical_plan/types.rs:144
    fn intern(&mut self, text: CellText<'_>) -> Value;  @f4ae3bd802:crates/ryo/crates/operators/src/domain/physical_plan/types.rs:146
    fn mint(&mut self, text: CellText<'_>) -> Value;  @f4ae3bd802:crates/ryo/crates/operators/src/domain/physical_plan/types.rs:148
    fn source(&self) -> OpenFileBytes<'_>;  @f4ae3bd802:crates/ryo/crates/operators/src/domain/physical_plan/types.rs:150
    fn tree(&self) -> &Elements;  @f4ae3bd802:crates/ryo/crates/operators/src/domain/physical_plan/types.rs:152
pub struct PlanNodeId(pub u32);  @f4ae3bd802:crates/ryo/crates/operators/src/domain/physical_plan/types.rs:45
    pub operation: &'p Operation,  @f4ae3bd802:crates/ryo/crates/operators/src/domain/physical_plan/types.rs:97
    Materialized(&'b Table),  @f4ae3bd802:crates/ryo/crates/operators/src/domain/physical_plan/types.rs:110
pub struct LogicalProgram<'p> {  @f4ae3bd802:crates/ryo/crates/operators/src/domain/physical_plan/types/logical_program/value.rs:18
pub struct PhysicalPlan {  @f4ae3bd802:crates/ryo/crates/operators/src/domain/physical_plan/types/physical_plan/value.rs:16
pub struct FileBuffers {  @f4ae3bd802:crates/ryo/crates/operators/src/domain/physical_plan/types/file_buffers/value.rs:31
    pub fn rows(&self, node: PlanNodeId) -> NodeRows<'_> {  @f4ae3bd802:crates/ryo/crates/operators/src/domain/physical_plan/types/file_buffers/value.rs:155
    pub fn retain(&mut self, node: PlanNodeId, mut keep: impl FnMut(&[Value]) -> PredicateOutcome) {  @f4ae3bd802:crates/ryo/crates/operators/src/domain/physical_plan/types/file_buffers/value.rs:196
```
`Resources::render` is at `types.rs:158-161` (multi-line). The layout words in this contract: `Value` (the 16-byte
tagged cell), `Table` (row-major `Vec<Value>`), `ColumnsView` inside `FactRows::Lent`, `&[Value]` in `retain`,
`Elements` (language's tree). `PhysicalPlan` on main has two private fields (`nodes`, `indexes`), B133 six.

**The fault type.** 20 variants; two name other crates:
```
pub enum PortableOperatorFault {  @f4ae3bd802:crates/ryo/crates/operators/src/domain/contract/types.rs:26
    Derived(ryo_derived::domain::fault::types::DerivedFault),  @f4ae3bd802:crates/ryo/crates/operators/src/domain/contract/types.rs:69
    Pattern(ryo_matching::inbound::use_cases::matching::errors::MatchingError),  @f4ae3bd802:crates/ryo/crates/operators/src/domain/contract/types.rs:76
```

**The kernel collection.** 21 items in `domain/physical_plan/collections/kernels`: components, directed_walk,
distinct, extend, filter, group, hash_index, lookup, ordered_match, ordered_walk, probe, probe_right, project,
queued_walk, reach, render, scan_bound, scan_facts, scan_node, scan_roots, text_map. Four plan-time rewrites
(existence_probe, keyed_select, ordered_existence, tree_probe). Seven comparison families (affix, equality,
float_text, glob, membership, numeric, ordering). B133 adds kernels partition and pointer_chase and rewrites
band_existence, base_probe, facts_right, group_keys, partitioned_select, pointer_chase, seeded_closure (23 and 11).

**Traits.** 12 `pub trait` (CaptureOutput, ComparisonsItem, `sealed::Sealed`, PatternSemantics, PlanInputs, Resources,
TextRowDriver, TableRowOutput, WordOutput, PlanUseCase, RunUseCase, MatchingPort) and the 4 `pub(crate)` ones:
```
pub(crate) trait KernelsItem {  @f4ae3bd802:crates/ryo/crates/operators/src/domain/physical_plan/collections/kernels/interface.rs:6
pub(crate) trait RewritesItem {  @f4ae3bd802:crates/ryo/crates/operators/src/domain/physical_plan/collections/rewrites/interface.rs:7
pub(crate) trait KeptRoom {  @f4ae3bd802:crates/ryo/crates/operators/src/domain/kept_room/types.rs:14
pub(crate) trait Poolable: Default {  @f4ae3bd802:crates/ryo/crates/operators/src/domain/kept_room/types.rs:79
```
`KernelsItem`'s module is declared `#[cfg(test)]` (`kernels.rs:10-11`): on main the kernel collection's trait exists
only in test builds; the dispatch is the `Lowering` match. Matching came in through a sealed port and an outbound port:
```
pub trait PatternSemantics: sealed::Sealed {  @f4ae3bd802:crates/ryo/crates/operators/src/domain/patterns/types.rs:40
pub trait MatchingPort: Send + Sync {  @f4ae3bd802:crates/ryo/crates/operators/src/outbound/matching/interface.rs:9
pub trait ComparisonsItem {  @f4ae3bd802:crates/ryo/crates/operators/src/domain/comparing/collections/comparisons/interface.rs:9
```
Functions on main are a domain module of this crate (6 text makers; the 8 flag makers are `Test` variants):
```
pub fn apply(function: Function, arguments: &[MadeText], made: &mut MadeText) {  @f4ae3bd802:crates/ryo/crates/operators/src/domain/functions/service.rs:10
pub enum Function {  @f4ae3bd802:crates/ryo/crates/values/src/domain/plan_parts/types.rs:231
pub enum Test {  @f4ae3bd802:crates/ryo/crates/values/src/domain/plan_parts/types.rs:152
```

**Public items.** 168 lines declaring a `pub` item outside test, fixture and case files (grep; reachability not
computed); per module in api.txt's `# NO LONGER PUBLIC`.

**Implementors and constructors.** `PlanUseCaseImpl`, `RunUseCaseImpl` in this crate:
```
    pub const fn new() -> Self {  @f4ae3bd802:crates/ryo/crates/operators/src/inbound/use_cases/plan/use_case.rs:14
    pub fn new(matching: Arc<dyn MatchingPort>) -> Self {  @f4ae3bd802:crates/ryo/crates/operators/src/inbound/use_cases/run/use_case.rs:28
```
**Callers.** One port over the two calls, in `execution_runtime`, and its one adapter (`OperatorsUseCaseAdapter`):
```
pub trait OperatorsPort: Send + Sync + std::fmt::Debug {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/outbound/operators/interface.rs:19
    let plan = operators.plan(build.program()).map_err(portable_fault)?;  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/file_pass/service.rs:205
        .run(physical(&pass.program), node, buffers, inputs, resources)  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/file_pass/service/nodes/service.rs:292
impl PlanInputs for PassInputs<'_, '_, '_> {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/file_pass/service.rs:261
impl Resources for PassResources<'_> {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/file_pass/service.rs:296
```
plus the project pass (`project_pass/service/inputs/service.rs:31, 67`). Files naming `ryo_operators` outside the
crate: execution_runtime 35, execution_domain 32, core 11, execution 9, sdk 3, cli 4 (Cargo.toml, `src`, 2 tests),
tastemaker.toml and one spec. Composition at the edges: `cli/src/resources/query/run/composition.rs:83`, core fixtures.

**Proof.** 166 `#[test]`; 126 files holding `CaseName(` rows, 541 such lines (by `grep -c`): `inbound/use_cases/run`
27 files / 171 rows, physical-plan internals 48 / 136 (collections 25/60, types 20/62, service_test 3/14), comparison
families 7 / 70, the rest one file per domain module. Two acceptance features, two scenarios each:
```
    Scenario: a program whose nodes read only earlier nodes is planned  @f4ae3bd802:crates/ryo/crates/operators/src/inbound/use_cases/plan/acceptance.feature:6
    Scenario: a node that reads a later node is refused  @f4ae3bd802:crates/ryo/crates/operators/src/inbound/use_cases/plan/acceptance.feature:12
    Scenario: a select of a relation answers the rows that hold  @f4ae3bd802:crates/ryo/crates/operators/src/inbound/use_cases/run/acceptance.feature:6
    Scenario: a relation the facts refuse is never empty rows  @f4ae3bd802:crates/ryo/crates/operators/src/inbound/use_cases/run/acceptance.feature:12
        "allocation::operators::allocations_do_not_grow_with_rows_or_the_graph",  @f4ae3bd802:crates/ryo/crates/operators/tests/allocation.rs:19
```
8 `compile_fail` doctest lines (kernels and lowering unreachable; an implementor must supply `plan` and `run`). Benches
`kernels` and `graphs` (section 8). B133: 173 `#[test]`, 132 files / 578 `CaseName(` lines.

**Tastemaker entries on it** (main `crates/ryo/tastemaker.toml`): lines 34-35 (internal-package and sibling
allowlists: relations, values, matching, language, derived), 44 (`no-default-trait-methods` on the contract files), 72
(`operator-meaning-once` over `physical_plan/service.rs` and `lowering/service.rs`), 75 (`driver-implementers`: "a
second, engine-side execution path cannot return"), 89 (`allocation-test` template on `tests/allocation.rs`), 116
`crate-operators`, 117 `nu-hex-operators`.

## 3. On the perf branch (2a207464c)
**Where.** No operators crate (`git ls-tree` at the tip: absent). The concept is inside the proof-of-concept crate
`executor` (`crates/ryo/crates/executor`), with no contract: `finish.rs` (4,388 lines) and 41 files of
`executor/src/finish/`, plus `pair_summary.rs` (1,892) for screening; argument and expression types in
`values/src/domain/finish_plans`; function and kernel cores in `values/src/domain/{functions, rank, words, cycles}`;
descriptions in `execution_domain/src/domain/{describing, operator}`; regex and globs in `matching`.

**The interface actually called.** Not a trait: the run's `Exec` methods, a switch on the step's name as text:
```
pub struct Exec<'p> {  @2a207464c:crates/ryo/crates/executor/src/finish.rs:1220
    pub fn eval(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Rc<Table>> {  @2a207464c:crates/ryo/crates/executor/src/finish.rs:1876
    pub fn step(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Table> {  @2a207464c:crates/ryo/crates/executor/src/finish.rs:2041
            other => Err(format!("operator {other} not in the step set").into()),  @2a207464c:crates/ryo/crates/executor/src/finish.rs:2433
        let t = exec.eval(frame, run, facts, slots[*node])?;  @2a207464c:crates/ryo/crates/executor/src/finish/session.rs:430
```
`eval(node)` is the same shape as main's `run(node)`: the driver asks for each emitted node; `eval` → `res` →
`make_` pulls producers through `self.eval(..)` inside kernels, memoised per file (`File::memo`), and answers early:
`known_empty` (`finish.rs:1735`), the frame's gate ("a step over an empty input answers its shared empty table before
any set-up", `:1739`), views never made whole (`Res::View`), keyed pulls (`Exec::probe`, `:1785`), a closure never
made whole (`Exec::lazy`, `:1957`). The map is: `Exec` = prepared per-request state + per-worker caches; `File` =
per-file buffers; `Run` = the dictionary, regexes, globs, paths; `Provider` = the inputs (fill, render).

**The name lists, five of them, and where they disagree.**
```
pub enum QueryOperator {  @2a207464c:crates/ryo/crates/values/src/domain/query_operators/types.rs:24
pub const OPERATORS: [QueryOperator; 34] = [  @2a207464c:crates/ryo/crates/values/src/domain/query_operators/constants.rs:14
operation_kinds! {  @2a207464c:crates/ryo/crates/values/src/domain/query_operators/types.rs:120
pub enum FinishStep {  @2a207464c:crates/ryo/crates/values/src/domain/finish_plans/types.rs:160
    Unlisted(UnlistedArguments),  @2a207464c:crates/ryo/crates/values/src/domain/finish_plans/types.rs:214
pub fn visit_slots<'a>(step: &'a FinishStep, mut visit: impl FnMut(&'a str, usize)) -> &'a str {  @2a207464c:crates/ryo/crates/executor/src/finish/plan_load.rs:96
pub fn parameters(step: &FinishStep) -> Args {  @2a207464c:crates/ryo/crates/executor/src/finish/plan_load.rs:275
pub enum Op {  @2a207464c:crates/ryo/crates/executor/src/finish/frame.rs:9
```
- `QueryOperator` 34. `OperationKind` 31: lacks GroupMin, GroupMax, Order, Prefixes, SubtreeClass; has GroupBound and
  Hosted, which `QueryOperator` lacks. `FinishStep` 27 = 25 operators + `Base` + `Unlisted`: Join, Intersection,
  Difference, GroupDigest, Words, Rank, Cycles, Order, Worklist have no variant (9), and all but Join reach the
  executor as `Unlisted` text. The executor's text names (`Exec::step`, `Exec::schema`) are the 34 minus Join plus
  Base. `frame::Op` names 9 (`Select, Union, Antijoin, Extend, Base, Rows, Closure, Follow, GroupCount`, `Other`),
  matched from text at `frame.rs:44-52`.
- **The tip has typed steps and un-types them.** `PNode::op()` turns the typed `FinishStep` back into a `&str`
  through `visit_slots` (`plan_load.rs:215`), and `args()` turns its typed arguments into a text-keyed
  `BTreeMap<String, Param>` (`parameters`, memoised per node, `plan_load.rs:271`), which kernels read as
  `a["keys"]`, `a["from"]`.
- The same 34 names again: `Exec::schema` (output columns) and `pair_summary::Builder::step` (screening; it matches
  `FinishStep` variants, typed, and the `Unlisted` "order" by text at `:1600`):
```
    pub fn schema(&mut self, run: &mut Run, node: usize) -> R<Rc<Schema>> {  @2a207464c:crates/ryo/crates/executor/src/finish.rs:1348
            other => return Err(format!("operator {other} not in the step set").into()),  @2a207464c:crates/ryo/crates/executor/src/finish.rs:1580
    pub fn step<'a>(  @2a207464c:crates/ryo/crates/executor/src/pair_summary.rs:1335
```
- **Three second paths behind environment switches** ("one path" is not held): union and antijoin are dispatched in
  `body` before `step`, and select has an old path:
```
            frame::Op::Union if !sw!("VEC_OLD_UNION") => return union_kernel::whole(self, f, run, p, node),  @2a207464c:crates/ryo/crates/executor/src/finish.rs:1981
            frame::Op::Antijoin if !sw!("VEC_OLD_ANTIJOIN") => return antijoin_kernel::whole(self, f, run, p, node),  @2a207464c:crates/ryo/crates/executor/src/finish.rs:1982
        if sw!("VEC_OLD_SELECT") {  @2a207464c:crates/ryo/crates/executor/src/finish/select_kernel.rs:444
```

**The 34 names of `Exec::step`** (X = `executor/src/finish`; ".cols lines" = lines matching `\.cols\b` in the arm or
kernel file; helpers = the `Table` methods used; MAIN = the `Operation` arm in main's `lowering/service.rs` and the
`Lowering` it makes, or "none"):

| # | Name | Kernel at the tip | .cols lines / helpers | MAIN counterpart |
|---|---|---|---|---|
| 1 | base | `finish.rs:2047` → `Exec::base` 2468-2490, `fill_len::fill_or_empty` | 0 / `copy`, `push_row` | none as an operator: `PlanInput::Relation` → scan_facts / scan_node |
| 2 | select | `:2048` (and `body` → `select_entry`, empty_sel.rs:174) → `select_k` X/select_kernel.rs:443 (1,174 lines) with band, keyed, view_*, cold_*; old `Exec::select` 2559-3006 | 16 in select_kernel.rs | `:53` Filter (+ rewrites keyed_select, existence_probe, tree_probe) |
| 3 | union | `:2049-2059` inline (only under `VEC_OLD_UNION`); live: X/union_kernel.rs:97 `whole` | 1 / `col`, `copy`; kernel 3 | `:93` set → Distinct |
| 4 | intersection | `:2060-2064` → X/set_ops.rs:6 `select_rows` | 0 / —; file 3 | `:94` set → Probe |
| 5 | difference | same as 4 | same | `:95` set → Probe |
| 6 | group-digest | `:2065-2070` → X/set_ops.rs:37 `digest` | 0 / — | `:207` Group |
| 7 | antijoin | `:2071` → `Exec::antijoin` 2491-2544 (only under `VEC_OLD_ANTIJOIN`); live: X/antijoin_kernel.rs:70 | 6; kernel 9 | `:96` set → Probe |
| 8 | extend | `:2072-2089` inline | 0 / `copy`, `push_const_col` | `:65` Extend |
| 9 | project | `:2090-2107` inline | 1 / `col`, `push_row` | `:59` Project |
| 10 | axis | `:2108` → `Exec::axis` 3007, `axis_over` 3075-3204, X/walk.rs | 2; walk.rs 4 | `:220` Axis |
| 11 | group-count | `:2109-2145` inline; `Exec::counted` 2003 (band) | 2 / `col`, `push_row` | `:99` Group |
| 12 | group-min | `:2146-2180` inline | 4 / `col`, `push_row`, `row` | none |
| 13 | group-max | same as 12 | same | none |
| 14 | cycles | `:2181-2186` → X/populations.rs:9 `cyclic` | 0 / `col`; file 4 | `:303` Components |
| 15 | rank | `:2187-2192` → X/populations.rs:36 `ranked` | 0 / — | `:109` TextMap(Rank) |
| 16 | closure | `:2193-2206` over `Exec::lazy` 1957 | 0 / `push_row` | `:211` Reach |
| 17 | follow | `:2207-2251` inline | 4 / `col`, `push_row` | none |
| 18 | ordered-match | `:2252` → `Exec::ordered` 3205-3315 | 6 | `:161` OrderedMatch |
| 19 | as-of | same as 18 (`as_of = true`) | 6 | none |
| 20 | span-between | `:2253-2267` inline | 2 / `col`, `copy` | `:125` TextMap |
| 21 | span-bounds | `:2268-2277` inline | 2 / `col`, `copy` | `:158` TextMap |
| 22 | words | `:2278-2284` → X/words.rs:14 `write` | 0 / `col`; file 1 | `:122` TextMap(Words) |
| 23 | source-matches | `:2285` → `Exec::source_matches` 3316-3398 | 2 | `:128` TextMap |
| 24 | replace | `:2286-2303` inline, `run.re` (the `regex` crate) | 2 / `col`, `copy` | `:112` TextMap |
| 25 | capture | `:2304-2320` → X/capture.rs:155 `write` | 0 / `col`; file 3 | `:106` TextMap(Capture) |
| 26 | group-ordinal | `:2321-2351` inline | 3 / `col`, `push_row`, `row` | none |
| 27 | render-syntax | `:2352-2366` inline, `p.render` once per row | 1 / `col`, `push_row`, `row` | `:179` Render (batched) |
| 28 | subtree-class | `:2367` → X/tree_class.rs:17 | file 5 | none |
| 29 | prefixes | `:2368` → X/prefixes.rs:12 | file 3 | none |
| 30 | order | `:2369` → X/ordering.rs:130 | file 5 | none |
| 31 | fits-rendered | `:2370-2398` inline | 2 / `col`, `push_row`, `row` | `:146` TextMap |
| 32 | rows | `:2399-2417` inline | 2 / `push_row` | `:311` ScanBound |
| 33 | worklist | `:2418-2425` → X/worklist.rs:94 `trace` | 0; file 8 | `:272` Worklist |
| 34 | ordered-dfs | `:2426-2432` → X/ordered_dfs.rs:33 `walked` | 0; file 9 | `:253` OrderedWalk |

Main has two arms with no tip name: `Join` (`:69`, `:183`; the tip lowers joins into select sources) and `Hosted`
(`:318`, ScanBound; the tip reads hosts through `base`, `Supplier::Host`). Eight tip operators have no main
counterpart (group-min, group-max, follow, as-of, group-ordinal, subtree-class, prefixes, order).

**Expressions and functions.** The compiled expression, its evaluator per row, and a per-row text dispatch of 14
functions; an unknown name answers `false` and is refused only after the emit:
```
pub enum E {  @2a207464c:crates/ryo/crates/executor/src/finish.rs:1126
pub fn eval(run: &mut Run, f: &File<'_>, srcs: &[Src], cur: &[u32], e: &E) -> V {  @2a207464c:crates/ryo/crates/executor/src/finish.rs:4106
pub fn call(run: &mut Run, name: &str, vs: &[V]) -> V {  @2a207464c:crates/ryo/crates/executor/src/finish.rs:4180
            run.missing.insert(name.to_owned());  @2a207464c:crates/ryo/crates/executor/src/finish.rs:4273
            return Err(format!("unimplemented functions {:?}", run.missing).into());  @2a207464c:crates/ryo/crates/executor/src/finish/session.rs:432
pub struct FunctionName(pub Box<str>);  @2a207464c:crates/ryo/crates/values/src/domain/finish_plans/expressions/types.rs:110
pub enum PlanExpression {  @2a207464c:crates/ryo/crates/values/src/domain/finish_plans/expressions/types.rs:19
```
The 14: matches, starts_with, ends_with, pascal, camel, snake, kebab, lower, concat, glob_any, natural_lt,
integer_successor, float_text_gte, numeric_gte (`finish.rs:4183-4275`); the case makers call main's old
`functions::apply`, moved to `values` (`ryo_values::domain::functions::service::apply`, `:4191`). The hot forms are
compiled once and cached per interned word (perf rows 21, 22):
```
pub enum TextTest {  @2a207464c:crates/ryo/crates/executor/src/finish/symbol_predicate.rs:93
    pub fn answer(&self, id: u32, test: impl FnOnce() -> bool) -> bool {  @2a207464c:crates/ryo/crates/executor/src/finish/symbol_predicate.rs:43
    pub fn re(&mut self, p: &str) -> Result<Rc<Bound<Rc<regex::bytes::Regex>>>, crate::check_session::refusal::Refusal> {  @2a207464c:crates/ryo/crates/executor/src/finish.rs:386
```
`symbol_predicate.rs` (212 lines): `Bound<T>` keeps a per-symbol answer bitmap in pages of 512 symbols;
`PreparedGlob` wraps `CompiledTextGlobs`; `SymbolSet` binds a word list once. `glob_pattern.rs` (41 lines) turns a
glob into a regex (used by select and by `pair_summary`).

**Which of matching's 11 methods the operators need** (`MatchingUseCase`, `matching/.../interface.rs:10`; callers at
the tip outside `matching`, tests excluded):

| Method | Callers at the tip | Fate in the draft |
|---|---|---|
| `compile_regex`, `matches_regex` | ryoql's kind check only (`kind_check/service.rs:117, 128`); the executor uses the `regex` crate directly (`finish.rs:386`, `pair_summary/regexes.rs`) | regex compile and search: private to this crate (Function::Matches, replace, source-matches, capture); the kind check calls `CompareUseCase::holds` |
| `visit_regex_matches`, `visit_regex_captures` | none at the tip (on main: operators' `MatchingPort` for source-matches and capture) | private to source-matches and capture |
| `compile_text_globs`, `matches_text_globs`, `matches_text_glob` | executor `plan_select.rs:148`, `symbol_predicate.rs:163`, `finish.rs:4236`, `pair_checks.rs:498`, `pair_summary/regexes.rs:28, 83`; ryoql `kind_check/service.rs:122` | the glob comparison: private to `Function::GlobAny`; screening and the kind check reach it through this crate |
| `compile_globs`, `parse_glob`, `match_globs` | execution's globset adapter only (`adapter.rs:38, 44, 52`) | path globs: to `outbound/corpus` (not this sheet) |
```
pub trait MatchingUseCase {  @2a207464c:crates/ryo/crates/matching/src/inbound/use_cases/matching/interface.rs:10
    let regex = (test == Test::Matches).then(|| matching.compile_regex(RegexSource(word)));  @2a207464c:crates/ryo/crates/query_language/src/domain/kind_check/service.rs:117
            let matched = MatchingUseCaseImpl::new().matches_text_glob(GlobPattern(&String::from_utf8_lossy(&g)), SearchText(&subject));  @2a207464c:crates/ryo/crates/executor/src/finish.rs:4236
```
The executor builds `MatchingUseCaseImpl::new()` inside kernels (per call), not through an injected port.

**Descriptions.** `execution_domain/src/domain/operator/constants.rs:17` `describe(operator) -> OperatorDescription`,
behind:
```
pub trait MetadataUseCase {  @2a207464c:crates/ryo/crates/execution/src/inbound/use_cases/metadata/interface.rs:11
```
(2 methods at the tip; on main the same trait had 23 methods, `operators` and `describe` at `:132, :134`.)

**Callers.** `Exec::step` is called only by `Exec::body` (`finish.rs:1998`); `Exec::eval` by `finish/session.rs`,
`session/demand.rs`, and `Exec::new` in `check_session.rs:673, 990`, `replay/batch.rs:172, 783`,
`replay/closed_workers.rs:273`, `frontier.rs:294`. Nothing outside the executor names a kernel; the SDK and the
benches reach the session.

**Proof.** No test is about an operator as an operator. Executor: 49 `#[test]` in 21 files; the closest are
`finish.rs:4377` (a language not covering embedded content reads the own tree) and `filter_order.rs:70`. Values:
`functions/service_test.rs` (11 case rows, `:17-77`), `rank/service_test.rs` (3), `words/service_test.rs` (3);
`execution`'s metadata test (`use_case_test.rs:95`, counts the 34). Matching: 5 `#[test]`. The real proof at the tip
is end to end: Ruff parity 412/413 on three repositories, cases 12,770/13,167, Oxlint 2,644/2,687, Clippy
1,015/1,015 (PLAN section 3, check-in figures).

**Tastemaker entries.** `operator-meaning-once` (tip `tastemaker.toml:69`) no longer includes any path the executor
has: the text switch is unwatched. `crate-executor` (`:105`). No `crate-operators`, `nu-hex-operators`,
`allocation-test` or `driver-implementers`.

## 4. What changed between them
| Change | Commit / measurement | Lost that should not have been? |
|---|---|---|
| A second engine grew outside the contract: `finish.rs` and kernels, first as a bench (`engine/benches/vec`, then `languages/python/benchmarks/vec`), in `executor` since `f2869d1f74` (26 Nov) | perf rows 40-48; Django 36.0 → 19.9 (row 46) | no: the speed is real; its place is the fault |
| Kernel cores (cycles, functions, rank, words) moved from operators to `values` so the executor could share them | `60bbd5537c` (26 Nov) | the functions left their concept |
| The old interpreter and the operators engine deleted | `fc94354b0c` (26 Nov) | yes: the contract |
| `physical_plan`, `distinct_rows`, `scan_rows` deleted "with their only callers" | `ad4af428fb` (27 Nov) | yes |
| The operators and derived crates deleted ("nothing outside them used them"); the metadata use case made the catalog | `c46607e439` (27 Nov) | yes: the two calls, the 166 tests, 126 case-table files, 2 features, the allocation test, the 2 benches, 6 Tastemaker entries |
| Eight operators added (group-min, group-max, follow, as-of, group-ordinal, subtree-class, prefixes, order) | Ruff/Oxlint parity work | not lost: they have kernels at the tip and no definition anywhere |
| Nine operators lost their typed arguments (pass as `Unlisted`) | — | yes: main typed seven of them |
| Functions became a per-row text dispatch; regex compiled by the `regex` crate directly | perf rows 21, 22 bound the hot forms once | the typed `Function`/`Test` |
| Matching's contract kept, honoured for globs only; operators' `MatchingPort` gone | `c46607e439` | the port is not missed (owner: "we do not colocate by mechanism") |

The owner's sentence that settles it: "the new paths were proofs of concept that were meant to sit behind the
contracts" (27 Nov).

## 5. The drafted public interface
The listing is `api.txt` beside this sheet: 251 listing lines (82 `[main]`, 62 `[perf]`, 76 `[changed]`, 31 `[new]`)
and the `# NO LONGER PUBLIC` section. Counts: main 168 `pub` item lines; tip about 450 in the executor plus 189 in
values (method in api.txt); draft 251 listing lines, of which 36 are traits, use-case methods, structs or enums a
caller names and the rest are variants, fields and argument types. This part explains every changed or new line.

### 5a. The claim it starts from, checked
Appendix A §3.4 drafts the two calls generic over `S: TablePort`, `PlanInputs<S>` and `Resources` with a
`symbols()` accessor, an `OperatorsItem` collection and `Demand`. Checked against the code: (1) main's contract equals
B133's (the appendix's `[133]` tags are `[main]` here); (2) main's `run` is already a pull by node, so the draft
keeps it rather than a push of inputs (trap 10.5 #4 holds only against `run(step, inputs, out)`, which nobody
proposes now); (3) `OperatorsItem` cannot carry `screening(arguments) -> Screening` as the appendix says, because the
tip's screening `Builder` is language-aware (`Builder { vocab, language, .. }`, `pair_summary.rs:240-256`); only the
operator's part (gate and column lineage) moves here; (4) `tree()` belongs in `PlanInputs` (appendix agrees), and
`Resources::source`, `render` stay.

### 5b. The two calls
```
pub fn ryo_operators::PlanUseCase::plan(&self, &LogicalProgram<'_>, &mut impl SymbolsUseCase) -> Result<PhysicalPlan, OperatorFault>  @new
pub fn ryo_operators::RunUseCase::run<S: TablePort>(&self, &PhysicalPlan, PlanNodeId, &mut FileBuffers<S>, &impl PlanInputs, &mut impl Resources) -> Result<(), OperatorFault>  @new
```
- `plan` gains the dictionary because the gains that matter most here are "bound once when rules load": literal
  globs compiled once (row 21), symbol tests bound once (22), equijoins bound at plan load (30), filters compiled at
  plan load (32), per-node data flat at plan load (41). On the tip these happen lazily on first use inside `Exec`
  (`ksel_prep`, `schemas`, `union_preps`, `antijoin_preps`, all `Rc`), which is why a prepared program cannot be
  shared by workers (426 `Rc` sites; ten-worker prep 1.637 s against 0.351 s). In the draft `plan` does all of it,
  once per request, and `PhysicalPlan` is `Send + Sync`. The answer caches of bound tests (`Bound::answer` pages)
  are mutable, so they live in `FileBuffers` (per worker), keyed by the plan's test number.
- `run` keeps main's five arguments and main's meaning: make `node`'s rows for the open file, pulling only what it
  reads. Laziness and early exit stay inside: a producer is made when a kernel first reads it (the tip's `self.eval`
  inside kernels), keyed reads stay (`probe`, `select_keyed`), a closure stays unmade, a gate that is empty answers
  before any set-up, a view is read in place. Nothing is pushed, so the 263,799 selects that answer nothing still stop
  at the gate. `S` is fixed at compile time; `PlanInputs` and `Resources` are `impl` (generics), never `dyn`.
- What `run` returns: nothing but a fault. The rows are in `FileBuffers` and read with `rows(node) ->
  NodeRows<'_, S>`, whose `Materialized` arm is the store's reader (`S::Reader<'b>`), not a table type. That is
  main's shape with the layout taken out.
- Crossing: `plan` once per request; `run` once per emitted node per file (the driver's `exec.eval` per emit today).
  No kernel, cell or row crosses.

### 5c. One definition per operator: the collection behind the two calls (crate-internal)
`OperatorCall` is the one list; each variant is one directory `src/domain/operators/<name>/` holding everything about
that operator, behind a crate-internal trait (the shape is fixed so two builders build the same thing):
```
pub(crate) trait OperatorsItem {                                       @new
    type Arguments;                       // the typed arguments (the OperatorCall variant's payload)
    const NAME: OperatorName;
    fn describe() -> OperatorDescription;                               // today execution_domain describing
    fn shape(arguments: &Self::Arguments, inputs: &[&Shape]) -> Result<Shape, OperatorFault>;   // today Exec::schema
    fn needs(arguments: &Self::Arguments, wanted: &Demand) -> Vec<Demand>;   // what it needs of each input
    fn gate(arguments: &Self::Arguments) -> Gate;                       // today frame.rs gate + screening pass-through
    fn summary(arguments: &Self::Arguments, inputs: &[&StepSummary]) -> StepSummary;   // today Builder::step's arm
    fn prepare(arguments: &Self::Arguments, symbols: &mut impl SymbolsUseCase) -> Result<Self::Prepared, OperatorFault>;
    type Prepared: Send + Sync;           // today select_preps / union_preps / antijoin_preps / band shapes
    fn kernel<S: TablePort>(prepared: &Self::Prepared, run: &mut KernelRun<'_, S>) -> Result<(), OperatorFault>;
}
```
and the directory holds the kernel's file(s) and its `cases.rs`. `plan` calls `shape`, `needs`, `gate`, `summary`,
`prepare` once per node; `run` calls `kernel`. One `match` on `OperatorCall` (macro-generated from the collection)
replaces `Exec::step`, `Exec::schema`, `visit_slots`, `parameters`, `frame::Op` and `Builder::step`'s arms. The
functions are a second collection, `FunctionsItem { NAME, kinds, bind (in plan), apply }`, one directory each.

### 5d. What changed in the types, line by line
- `LogicalNode.call: &'p OperatorCall` replaces main's `operation: &'p Operation` (`types.rs:97`). Main's
  `bindings: NodeBindings` goes: parameters are resolved by `plan` before the call (the tip's lowering resolves them
  into the step). `OperatorCall` takes the tip's typed arguments (37 types in `finish_plans/arguments`), gives the
  nine `Unlisted` operators typed arguments (seven drafted from main's `Operation` fields; `Order` and `SetArguments`
  new), and drops `Join` (lowered into `Select`, as the tip does). `OperatorName` replaces `QueryOperator`, generated
  with `OperatorCall`, so the names exist once.
- `PlanInput` gains `Tree(TreeInput)` and `Hosted`; `Relation` carries `RelationInput(u32)`, a number `plan` gives,
  instead of the catalog's `Relation`. `TreeInput` says which columns of the tree a tree relation reads
  (`TreeColumn`, the catalog's `NodeColumn` restated), so select reads the tree in place without asking the catalog
  (decision 5).
- `Expression` is the tip's `PlanExpression` moved; `FunctionCall.function: Function` replaces `FunctionName(Box<str>)`.
  `Function` is one closed list of 14 (main's 6 text makers + main's 8 flag-making `Test` variants = the tip's 14
  names). The per-row `call(run, name: &str, ..)` and the late "unimplemented functions" refusal disappear: an
  unknown function is a `plan` error in the `plan` crate, before any file.
- `PlanInputs`: `type Rows<'a>: TableRead` (the implementor's reader for lent facts, carried inputs and host rows),
  `type Tree: TreeRead`; `facts(relation, &Demand)` (main's `facts(relation)` told what is needed, pulled on first
  use, row 12); `tree()` moved here from `Resources`; `frontier`, `observation`, `hosted` return `HeldRows<R>`.
- `Resources`: `type Symbols: SymbolsUseCase + FixedWords` and `symbols()` replace main's `texts`, `intern`, `mint`
  (the symbols sheet's `NO LONGER PUBLIC` lists the same three); `path(CellWord)` names what main's `Texts::paths`
  and the tip's `Run::paths` hold; `render` keeps main's batch (one crossing per set; the tip calls `p.render` once
  per row, `finish.rs:2359`).
- `PhysicalPlan` gains five reads: `shape(node)` (the tip's `Exec::schema`, computed once), `gate(node)`,
  `summary(node)` (screening reads the operator's part here, so `application/screen` never matches operator names),
  `demands()` (what the program needs of each relation, so `execute` fills only that) and `node_kind -> OperatorName`.
- `FileBuffers<S>` owns the worker's store (`new(store)`); main already kept a table pool there
  (`pool: KeptPool<Table>`, `file_buffers/value.rs:45`). `retain` takes row numbers instead of a closure over
  `&[Value]`; `empty_as` takes a `Shape` instead of `&[CellKind]`.
- `OperatorFault` = main's 20 − `Derived` − `Pattern` + `InvalidPattern { pattern }` + `MissingInput` +
  `DuplicateDfsOrder` (the two `DerivedFault` cases main's variants lacked) = 21. `NoSuchColumn.index` is the table's
  `ColumnNumber`.
- The two reads beside the calls: `CatalogUseCase { operators, describe }` is the tip's `MetadataUseCase` moved to
  its owner (inbound/describe calls it); `CompareUseCase::holds(Function, SymbolBytes, SymbolBytes)` is main's
  `ComparisonsItem::holds` made the call ryoql's kind check makes for `matches` and `glob_any` over kind words
  ("the kind check uses the comparison operator's own definition", owner, 27 Nov).
- Built by an edge: `PlanUseCaseImpl::new()`, `RunUseCaseImpl::new()` (no `MatchingPort` any more),
  `CatalogUseCaseImpl::new()`, `CompareUseCaseImpl::new()`.

### 5e. What it imports, and from whom
`ryo_table`: `TablePort`, `TableRead`, `TreeRead`, `Shape`, `ColumnNumber`, `RowNumber`, `RowCount`, `CellWord`,
`Position`. `ryo_symbols`: `SymbolsUseCase`, `FixedWords`, `Symbol`, `SymbolBytes`. Nothing else in Ryo. Outside:
`regex` (or `regex-automata`, as matching used), `globset` is NOT needed (text globs are matching's own interpreter,
`domain/globbing`, 759 lines with tests at the tip, moving here), `heck` (case makers), `hashbrown`, `rustc-hash`, `num-bigint`,
`unicode-width`, `thiserror` (main's list minus the Ryo crates).

### 5f. Layout of the crate (nu-hex)
`src/inbound/use_cases/{plan, run, catalog, compare}` (interface, use_case, acceptance); `src/domain/operators/<one
directory per operator, 34>`, `src/domain/functions/<one per function, 14>`, `src/domain/{program, physical_plan,
buffers, fault, expressions}`; no `outbound` (no port: the store and the inputs come in as generics of `run`).

## 6. Proof it brings
| Proof | Source | Fate |
|---|---|---|
| the 2 plan scenarios | main `plan/acceptance.feature:6, 12` | carried as they are |
| the 2 run scenarios | main `run/acceptance.feature:6, 12` | restated: their inputs build row tables (`fixtures.rs:124 TableInputs`) |
| 171 run case rows (27 files) | main `inbound/use_cases/run/use_case_test*` | restated generic over `S: TablePort` against a test store; the rows' meaning is unchanged, their literals name `Table { kinds, cells, count }` |
| 70 comparison rows (7 files) | main `comparing/collections/comparisons/*` | carried into `functions/<name>/cases.rs`; `Datum` becomes `SymbolBytes` |
| 136 physical-plan rows (48 files) | main `physical_plan/{collections, types, service_test}` | restated where the kernel survives as an operator's kernel; the rest (lowering kinds, rewrites the tip does not have) dropped with the reason per file, never silently |
| ~164 other domain rows (one file per module) | main `domain/*/service_test.rs` | carried with their module into the operator's directory |
| the allocation test | main `tests/allocation.rs:19` | restated against `FileBuffers<S>` with a test store; it is the gate of row 37 |
| 8 `compile_fail` doctests | main plan and run interfaces | carried; they hold "a kernel is private" by the compiler |
| 11 function rows, 3 rank, 3 words | tip `values/.../{functions, rank, words}/service_test.rs` | carried into their directories |
| metadata test (counts 34) | tip `execution/.../metadata/use_case_test.rs:95` | carried as the catalog test |
| one case table per tip-only operator (8) | — | **missing**: group-min, group-max, follow, as-of, group-ordinal, subtree-class, prefixes, order have no unit test on either side |
| a test per function name dispatch | — | **missing** on the tip; made impossible by the closed `Function` |
| Ruff/Oxlint/Clippy parity and the case suites | tip benches | carried unchanged: the proof that nothing moved |

**Build 133's 15 failures** (`fork-b/workspace-test-baseline-f0219b3b3.txt`: operators 265 passed, 15 failed, exit 101):
`allocation::operators::allocations_do_not_grow_with_rows_or_the_graph`;
`domain::physical_plan::collections::rewrites::keyed_select::item_test::succeeds`;
`domain::physical_plan::service_test::catalog_test::{refuses, succeeds}`;
`inbound::use_cases::run::use_case_test::axis_layout::test::{refuses, succeeds}`;
`...::fed::counters::test::succeeds`; `...::graphs::worklist::test::refuses`; `...::relational::test::refuses`;
`...::streaming::{combined, faults, probes, stages}::test::succeeds`; `...::streaming::test::succeeds`;
`inbound::use_cases::run::use_case_test::succeeds`. So the allocation test and the streaming seam were red at B133;
main's state of the same tests is not checked (no baseline file for main exists).

## 7. Perf gains it carries
From PLAN 10.3 (perf-gains.tsv does not exist yet); "depends on" is what the draft must allow.

| Row | Gain (measured) | Code at the tip | Depends on in the interface |
|---|---|---|---|
| 16 | scalar test pushed through an inner join into the scan (23.89 → 23.27) | `query_language/.../plan_lowering/pushdown.rs` (160 lines; goes to `plan`), `select_kernel.rs` | `SelectArguments` keeps a condition per source, and `prepare` binds it on the scan |
| 21 | literal glob compiled once, answer reused per interned text (57.2 → 36.2) | `symbol_predicate.rs` (`PreparedGlob`, `Bound::answer`), `Run::prepared_globs`, `pair_summary/regexes.rs` | `plan(.., symbols)` binds the glob once; `Symbol` stable for the run; the answer cache per worker in `FileBuffers` |
| 22 | columns and indexes set up once per file version; symbol tests bound once (-5.6 s, -2.3 s) | `symbol_predicate.rs`, `cold_text.rs`, `file_version.rs`, `File::index`, `fact_ix` | lent readers (`PlanInputs::Rows`, `tree()`) shared by every program run on the file; per-file indexes in `FileBuffers`, reset by `ended()` |
| 30 | equijoin bound at plan load; joins read base views (small) | `view_pair.rs`, `view_probe.rs`, `select_kernel.rs` | `prepare` at `plan`; joins read `PlanInputs` readers in place |
| 32 | row-id lists borrow the provider's index; filters compiled at plan load (2.14 → 0.137 s) | `cold_word.rs`, `cold_pred.rs`, `cold_test.rs`, `cold_text.rs` | the table sheet's `TableRead::matching` on a lent reader; compile in `prepare` |
| 34 | typed projection kernel; fused tuple pass | — | lost; nothing to carry |
| 36 | keyed path deleted: every step made whole (Django 13.02 → 11.98) | `finish.rs` `make_`, `File::memo` | `run` makes a node whole once per file and holds it in `FileBuffers` until `release`/`ended` |
| 41 | known-empty before the bracket; per-node data flat at plan load (26.49 → 18.14) | `known.rs`, `frame.rs`, `finish.rs:1735-1751` | `Gate` computed in `plan`, checked by `run` before any set-up; flat per-node data in `PhysicalPlan` |
| 42 | union and antijoin kernels (32.68 → 24.82) | `union_kernel.rs`, `antijoin_kernel.rs` | `Prepared` per node at `plan`; scratch in `FileBuffers` |
| 43 | two select kernels (17.30 → 16.73) | `select_kernel.rs` | same |

Also carried because their code sits here: 12 (fill on first use: `facts()` is a pull), 20 (step results reused:
constant nodes kept across files in `FileBuffers`, the tip's `Exec::run_memo`), 37 (pools: the store moves into
`FileBuffers`, released on its worker), 38 (empty-run propagation: `Gate`). Made room for: 13 (parked; plan, constants,
patterns and symbol tables frozen once and shared: `PhysicalPlan: Send + Sync`, `plan` takes the dictionary before it
is frozen).

## 8. Benchmarks that cover it
| Benchmark | Names today | After the draft |
|---|---|---|
| operators `kernels`, `graphs` (main only: `operators/benches/performance/{kernels,graphs}/bench.rs`) | `ryo_values::domain::rows::types::{RowCount, Table}`, `PlanUseCaseImpl`, `RunUseCaseImpl` | restored; build inputs through a test store; call `plan` and `run` only. Today nothing at the tip benches a kernel (table sheet 8a) |
| the lint images `vec`, `vec_ts`, `vec_rust` (`executor/benches/lint_image`) and the `tm` check | the SDK only | unchanged; the measure of "not lost" (the whole bar table against the kept build-139 images) |
| per-operator CPU split (STEP-COSTS: select 64.8%, antijoin 13.0, union 7.9) | `pub use ryo_values::profile as operators;` and `profile::leave(start, kind: &str, ..)`, keyed by the text name | `FileBuffers::count_nodes` / `take_counters` (main's `NodeCounters`) keyed by `OperatorName` |
| Ruff, Clippy, Oxlint case suites and parity | the SDK | unchanged |
```
pub use ryo_values::profile as operators;  @2a207464c:crates/ryo/crates/executor/src/finish.rs:15
pub fn leave(start: ThreadTime, kind: &str, rows: usize, ran: bool) {  @2a207464c:crates/ryo/crates/values/src/profile.rs:113
```

## 9. Checks
IMPORTS: table, symbols
CROSSED: per step
- Directions: domain imports only domain; `table`, `symbols` < `operators`. Not `relations` (decision 5), not
  `language` (main imported it for `Elements`, `RenderRequest`, `RenderOutcome`: all restated or replaced), not
  `values` (dissolved: its operator types move here), not `matching` (broken apart), not `derived` (decision 4).
  `ryoql` and `plan` import this crate (ryoql for `Function`/`Expression` and `CompareUseCase`; plan to write
  `LogicalProgram`).
- No layout word in a signature: no `Table`, `Value`, `ColumnsView`, `Vec<Vec<u64>>`, `&[Value]`, `Rc<Table>`;
  readers are associated types of the table contract.
- Crossing: `plan` per request; `run` per step asked per file (per emitted node; producers are pulled inside);
  `PlanInputs::facts` per relation per file (first use); `Resources::render` per set; `symbols()` per kernel run
  (hot interning stays per row inside the kernel, as on the tip; the call is static dispatch).
- Generics, not `dyn`: `S: TablePort`, `impl PlanInputs`, `impl Resources`, `impl SymbolsUseCase`.
- Review criteria: 2 (one path: the three `VEC_OLD_*` paths and the text switch go), 3 (kernels private, sealed by
  visibility, `compile_fail` carried), 4 (seam tests by the two calls), 6 (one list of operators, one of functions),
  11 (no per-row crossing), 19 (the draft adds `CompareUseCase` and five `PhysicalPlan` reads; each has a named
  caller: ryoql, screen, execute).

## 10. Decisions for the owner
1. **`run` generic over the store, with the store inside the worker's buffers.** Example: main's
   `buffers: &mut FileBuffers, inputs: &dyn PlanInputs, resources: &mut dyn Resources` (`run/interface.rs:43-45`)
   against the tip's per-thread pool (`finish.rs:155-275`). Why: naming one table type is what blew the contract up
   when rows became columns; `dyn` on the hot path costs a call per kernel input. What we get: the same five
   arguments as main, no layout, tables released on the worker that made them. Pick: generic (`S: TablePort`) and
   `FileBuffers::new(store)`.
2. **The two calls are the contract; screening, shape and needs are reads of the plan, not more calls.** Example:
   the tip computes a node's columns in `Exec::schema` (`:1348`) and its screening in `Builder::step`
   (`pair_summary.rs:1335`), each a second switch. Why: "i thought we had a simple contract?". What we get: one
   operator definition; `screen` and `execute` read `PhysicalPlan::{summary, gate, shape, demands}` decided once at
   load. The language-aware part of screening stays in `screen`. Pick: this, plus `CatalogUseCase` (moved, 2
   methods) and `CompareUseCase` (1 method, for ryoql's kind check) as reads that execute nothing.
3. **The name of the first call beside a crate called `plan`.** Example: `PlanUseCase::plan` (`plan/interface.rs:39`)
   makes a `PhysicalPlan` out of what the `plan` crate made. Why: two "plans" for two things. What we get: no
   ambiguity in imports and logs. Pick: rename to `PrepareUseCase::prepare` (and `PhysicalPlan` to
   `PreparedProgram`); the listing keeps main's names until you say.
4. **The fault, and `derived`.** Example: `Derived(ryo_derived::domain::fault::types::DerivedFault)` and
   `Pattern(ryo_matching::..::MatchingError)` (`contract/types.rs:69, 76`); the tip refuses with
   `Refusal::Other(String)` texts ("operator {other} not in the step set", "regex {p}: {e}"). Why: a fault that names
   another crate drags it into every caller. What we get: `OperatorFault`, 21 variants, typed. `derived` (6 traits on
   main: `RowCells`, `ValueTexts`, `CanonicalTexts`, `BuildUseCase`, `PrepareDerivedUseCase`, `ViewUseCase`) serves
   ordered-match's ordering and the derived views; pick: fold into the operators that use it (ordered-match,
   closure, cycles), not restored as a crate, its two missing fault cases added here.
5. **Operators do not import `relations`.** Example: main's `PlanInputs::facts(relation: Relation)` and main deciding
   "is this the tree?" from the catalog; the tip's `Native::{Tree, Child, Location, Token, Fact}` (`finish.rs:540`).
   Why: "why do operators depend on rewlations idgi". What we get: `plan` numbers relations (`RelationInput`) and says
   which are the tree and which tree column each reads (`TreeInput`, `TreeColumn`). Cost: `TreeColumn` restates the
   catalog's `NodeColumn` (10 variants); one definition per concept says it should live once, so pick: move
   `NodeColumn` down to `table` beside `TreeRead` (it names `TreeRead`'s accessors) and import it here; if the table
   sheet declines, this sheet keeps `TreeColumn`.

## 11. Not checked
- The argument structs' fields: names only; the 37 tip types are said to move unchanged, not compared field by field
  with main's `Operation` variants. The seven `[new]` argument structs (`CyclesArguments`, `GroupDigestArguments`,
  `RankArguments`, `SetArguments`, `WordsArguments`, `WorklistArguments`, `OrderArguments`) have no drafted fields;
  `Order`'s arguments exist only as `Unlisted` text (`pair_summary.rs:1600`).
- Whether dropping `LogicalNode.bindings` loses anything: I did not trace main's `ParameterBinding` users nor confirm
  the tip resolves every parameter before the executor (it resolves `Rows` literals; others not read).
- `StepSummary` and `Gate`: drafted from reading `Builder::step`'s pass-through arms (`:1443`, `:1457`, `:1598-1599`)
  and `frame.rs`; the Select, Axis and SourceMatches arms (`:1347-1388`, `:1498-1597`) were not read in full, so
  whether lineage plus gate is all screening needs from an operator is not shown.
- Line counts at the tip per operator: only `.cols` lines and helpers per arm; kernel bodies of band, keyed, view_*,
  cold_* were not read. Join's lowering into select at the tip was not traced.
- Main's 168 and the tip's ~450 public item counts are grep counts of declaration lines, not reachable items.
- Main's suite state: no main test baseline exists; whether main's operators tests pass is not checked.
- `derived`'s views: traits counted, bodies not read; whether closure and cycles at main used them is not checked.
- `NodeCounters`' own public methods on main were not listed (fields are `pub(crate)`).
- The cost of moving the store into `FileBuffers` against the tip's thread-local pool, and of the generic `run`, is
  not measured; the table spike measures only select, antijoin and one text function.
- Whether `symbols()` per kernel run (rather than a field) costs anything: not measured.
- Tastemaker: no new entries drafted; `operator-meaning-once` would need new params naming `OperatorCall`.
- The `CaseName(` counts include each file's struct-literal lines only approximately (a file's own `CaseName` type
  definition or helper may add one); exact rows per file not taken.
