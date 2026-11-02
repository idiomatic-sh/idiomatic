# Extension ownership contracts

The engine should own scheduling, sharing, invalidation, and lifetime so a language provider or RyoQL primitive can plug in facts or typed row semantics. This table records **current ownership**, not the desired performance or type-safety level. `☑` means the generic engine owns the responsibility today; `☐` means a language, primitive, compiler extension, or outbound adapter still owns part of that responsibility; `—` means it does not apply. Producing correct facts or row semantics alone does not make an implementor the owner of stored data or scheduling. A checked cell is not a performance claim.

| Contract | Scheduling | Sharing / cache | Invalidation | Data ownership | Resource lifetime | Residual implementor ownership or central gap |
| --- | :---: | :---: | :---: | :---: | :---: | --- |
| Language parse and project | ☑ | — | ☑ | ☐ | ☐ | Provider owns parser state and its call-local scratch; engine requests and releases resident facts. Provider fact semantics are separate. |
| Typed relation catalog | ☑ | — | — | ☑ | ☑ | Engine owns typed resident columns; provider populates their values. Semantic row correctness remains a provider contract. |
| Complete and partial facts | ☑ | ☑ | ☑ | ☑ | ☑ | Engine owns reuse refusal and resident generations; provider must truthfully report recovery and partials. |
| Provider walk vs execution demand | ☑ | — | — | ☑ | — | Engine owns both demands. Raw slices still permit confusing them; a typed wrapper is pending, without changing ownership. |
| Source bytes and spans | ☑ | — | ☑ | ☑ | ☑ | Engine holds the file buffer; provider supplies token coordinates and lexical meaning, not a retained cache. |
| Source and derived symbols | — | — | ☑ | ☑ | ☑ | Engine owns one file's symbol table and query mutation capability; provider supplies source name semantics. |
| Query grammar and typed plan | ☑ | — | ☑ | ☑ | ☑ | Compiler is an engine component; new syntax still requires compile/typecheck error mapping. |
| Primitive schema and evaluator | ☑ | ☑ | ☑ | ☑ | ☑ | All 24 kinds use request-prepared contracts and the mandatory application executor; primitives receive borrowed semantic capabilities. Behavioral, generation/lifetime and compile-negative capability controls pass; internal optimized algorithms remain behind the same owner. |
| Operator inputs and absence | ☑ | — | — | ☑ | — | Prepared contracts declare dependencies and absence requirements. Actual production-route tests cover positive partial facts and refusal of partial absence; provider truthfulness and relation semantics remain separate obligations. |
| Ordinary plan and frontier | ☑ | ☑ | ☑ | ☑ | ☑ | Engine owns ordering, rule-local tables, frontier handover, and opt-in sharing; complete DAG coverage across all operators is still open. |
| Request-wide file graph | ☑ | ☑ | ☑ | ☑ | ☑ | Every kind enters the common application owner. DAG retention remains selectable. The measured relation worker took 222.59 s cold versus the required-live worker's 244.70 s; it remains above the older 214.56 s pre-contract observation. |
| Stored selections and indexes | ☑ | ☑ | ☑ | ☑ | ☑ | Engine owns predicate/index state, bounds, fallback, and release; provider only supplies complete resident columns. |
| Graph adjacency and traversal | ☑ | ☑ | ☑ | ☑ | ☑ | Engine owns typed per-file graph views and memo. Alternate storage remains unmeasured. |
| Persistent lint snapshot | ☑ | ☑ | ☑ | ☐ | ☐ | Engine chooses digests/reuse; outbound adapter owns framing, snapshot file handles, and atomic byte replacement behind the port. |
| Outbound reports and jobs | ☑ | — | ☑ | ☑ | ☐ | Engine owns reports and visit order; scheduler adapter still owns executor resources behind its port. |

The open ownership cells identify code that a new implementor must still coordinate. A provider supplies facts and completeness evidence; a primitive supplies typed row semantics. The engine should validate inputs and outputs, schedule their dependencies, share safe work, invalidate it, and release each file generation. Incomplete performance or type-safety evidence is named in the last column and does not by itself turn an engine-owned cell into an implementor-owned cell.

## Executable evidence and open gates

- **Catalog schema identity gap:** retained rule/frontier identity previously omitted relation column names, kinds and order. The source fix adds one request-wide digest of a versioned, length-framed catalog descriptor to stored-rule identity. Synthetic changed-name/kind/order/width and framing tests passed in the 440-test core library gate. The [old/new/warm retained-cache control](../../languages/python/benchmarks/linters/ruff/results/all24-contract-snapshot-2026-11-02/summary.json) executed PT010 **1, 1, 0** times with byte-identical reports, proving actual cross-version invalidation and new-version reuse. No ownership or lifecycle checkbox is credited by this source change.

- [Primitive registry and conformance](../../crates/core/src/domain/operator/contract_test.rs) exercises all 24 registered kinds through the same production evaluator in collected and streamed modes. [Production-route probes](../../crates/core/src/application/file_dag/contract_route_test.rs) additionally verify actual entry/output for all kinds, binding/generation isolation, and partial-absence refusal. The core library gate passed 440 tests on 2026-11-02; this is behavioral evidence, not a performance result. The two [capability integration controls](../../crates/core/tests/operator_capabilities.rs) pass: legitimate production-boundary implementations compile; forbidden scheduler/cache/escaping-borrow accesses and removal of each real registry arm fail for the intended reason.
- [Source readiness](../../crates/core/src/domain/table_model/service_test.rs) distinguishes stored/native complete rows from partial or recovered input. [QuerySymbols tests](../../crates/language/src/domain/symbols/service_test.rs) protect source versus derived symbol provenance.
- [Request graph tests](../../crates/core/src/domain/file_program/service_test.rs) cover identity, successful cut reuse, failure, last-consumer release, and demanded ancestor reads. The [scoped graph receipt](../../languages/python/benchmarks/linters/ruff/results/file-program-graph-2026-11-02/summary.json) proves one 12-rule Join reuse but a 1.63× full-Django cold slowdown on the first overlay. The demand-mask follow-up skips 44 ancestor steps in the one-file trace but its complete cold run remains slower at 435.77 s versus the retained 248.62 s graph-off run (1.75×); neither single observation establishes stable component attribution.
- The separate [application DAG paired receipt](../../languages/python/benchmarks/linters/ruff/results/file-dag-execution-2026-11-02/summary.json) used one sealed worker and fresh caches: 211.50 s graph-on versus 255.31 s graph-off on frozen Django, with byte-equal complete reports. It is one scoped observation, not a default-on or general performance claim.
- The historical [mandatory-contract snapshot](../../languages/python/benchmarks/linters/ruff/results/all24-contract-snapshot-2026-11-02/README.md) passed focused behavior tests but its candidate hit the 600 s Django cap after the matched baseline completed in 214.56 s. Candidate report parity and completed-run memory were unproven in that snapshot. A bounded diagnostic localized repeated 91,053-cell DAG node evaluation during `PLC0206` on `django/contrib/admin/options.py`; the later required-live follow-up addresses that repeated execution.
- The [required-live follow-up](../../languages/python/benchmarks/linters/ruff/results/required-live-2026-11-02/README.md) passes 444 core, three capability and 205 Python tests. The repeated node runs three times on the same file, and cold, warm and retained-edit Django reports match their structured goldens. Frozen Django cold wall time is 244.70 s versus the retained 214.56 s baseline, with 3.23% higher peak RSS. This complete run still fails performance acceptance.
- The [pipeline Tastemaker spec](../../../../tastemaker/specs/pipeline/0.1.0/spec.toml) currently checks structure and test registration. A string containing “bounded” is not behavioral proof of bounded state. Product lifecycle tests with repeated unchanged and changed runs, measured retained state, and a deliberately unbounded negative control are required before a bounded-lifetime checkbox can be credited to that spec.

The [Ryo design](../README.md) is the architectural source. The tested boundary registers all **24 operator kinds** and all **four representations across 61 catalog relations**. Update this table only when a passing executable contract changes ownership. Keep source-bound fixture, lifecycle, and performance receipts separate; do not mark an opt-in strategy production-ready from one-file parity alone.

## Mandatory operator boundary: accepted implementation decision

The implementation replaces the pilot with one mandatory contract for all 24 kinds. Its required-live gate passed 444 core, three capability and 205 Python tests; the later relation gate passed 463 core, seven capability, 33 SDK, 206 Python and 107 benchmark tests. The measured relation worker completed frozen Django with byte-identical cold, warm and retained-edit reports. Cold wall time was 222.59 s versus 244.70 s for required-live, while the older pre-contract observation was 214.56 s. Performance acceptance remains open.

The existing `operator`/`operating` boundary owns the following interface shape; concrete supporting types reuse existing row, schema, trust, scratch and index concepts:

```rust
trait OperatorsItem: Sync {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[InputSchema<'_>],
        bindings: &[ParameterBinding],
    ) -> Result<OperatorContract, OperatingError>;

    fn evaluate(
        &self,
        operation: &Operation,
        inputs: PreparedInputs<'_>,
        lease: &mut PrimitiveLease<'_>,
        output: &mut dyn RowSink,
    ) -> Result<(), OperatingError>;
}

fn registered(kind: OperationKind) -> &'static dyn OperatorsItem;
```

- `OperatorContract` declares ordered dependencies, output schema, input access/order and lazy gates, absence requirements, column/symbol provenance, typed resource needs, effects and semantic identity. Prepare it once per request/operation; do not allocate or interpret metadata per row. Preparation must preserve the observable timing/order of deferred execution errors.
- `PreparedInputs` represents the existing finite execution shapes: an input row with its input position, complete input views, or an engine-supplied request source. The engine selects the shape and supplies any required immutable side/index views. Reuse existing domain algorithms; do not introduce an event bus, generic execution language or second plugin framework.
- Whole-input requirements inherent in grouping, ordered matching and graph algorithms are declared semantics. Choosing to retain, materialize, index, stream, reuse or evict eligible data is engine policy. Primitives cannot call upstream `Feed::hold/stream`, choose cache admission, build retained indexes or schedule dependencies.
- `PrimitiveLease` supplies borrowed views and narrowly scoped text/symbol capabilities plus invocation scratch. The engine initializes, owns, bounds and releases that scratch. It is not an escape hatch for primitive-owned caches: no raw `FileIndexes`, cache mutation, retained `Arc`, generation keys, arbitrary persistent state or handles escaping the invocation. Stateful row semantics may mutate only their assigned workspace.
- Source/derived symbol operations remain separate capabilities. Source bytes, symbol arenas, coverage and input identities are generation-bound. Scratch may contain algorithm work state; reusable indexes, prepared matchers and cross-invocation memo are acquired and owned centrally.
- Registration is an exhaustive `match OperationKind`, without wildcard, `Option`, or legacy fallback. The existing enum-generated `OperationKind::ALL` also enumerates exhaustive conformance fixtures; adding a kind must fail compilation until registration and its fixture exist.

`domain/operator`, `operating`, `operation_schema`, `trust` and `file_program` retain contracts, semantic algorithms and pure planning. `application/file_dag` becomes the common execution/state owner for every kind and scope. Existing pipeline/lowered/column algorithms are engine-selected implementations behind this boundary, with no independent residual scheduler. Outbound supplies injected external facts/effects; inbound remains thin. Domain and outbound import no other architectural layer; application imports domain/outbound.

The engine controls request/file/invocation lifetimes, input demand, ordered terminals/frontiers, index acquisition/builds, shared identity, invalidation, retention limits and release on success/error/cancellation. Typed identity includes every semantic field and ordered child identity; generation-dependent results additionally bind parameters, host/source/input generations, completeness and symbol provenance. Hash equality alone never establishes identity.

A closed declaration may constrain sharing for genuine observable effects or unproven effect replay, while keeping execution inside the same engine. Missing registration, unsupported implementation and a per-kind opt-out are not semantic reasons. Preserve lazy branches, ordered bags, mode-specific failures and emitted prefixes; emitter failure stops further demand immediately. Partial positive facts remain usable; absence-dependent results require the declared completeness evidence.

All classes migrate in the same coherent batch:

| Kinds | Required distinctions |
| --- | --- |
| Select, Project, Extend | Comparison resources, ordered projection/distinct behavior, parameters and column provenance. |
| Union | First-occurrence distinct rows, left-before-right order and engine-owned seen workspace. |
| Join | Key/filter/view/reach semantics, duplicate multiplicity, lazy demand and indexed side views. |
| Intersection, Difference, Antijoin | Membership/key views and existing multiplicity; right-side completeness for Difference/Antijoin. |
| GroupCount, GroupDigest | Complete contributing input, first-seen groups, distinct/digest semantics and group workspace. |
| Capture, Replace, Words | Text/matcher views, derived symbols, zero/multiple outputs and failures. |
| Rank | Parameter-set order and text equality. |
| SpanBetween, SpanBounds, SourceMatches | Physical source identity, checked spans, capture/line behavior and source-backed provenance. |
| OrderedMatch | Both-input completeness, text-equivalent keys, duplicate-order failures and consumption order. |
| Axis | Three ordered dependencies, direction/filter/view semantics, adjacency and nearest-answer resources. |
| Closure, Cycles | Graph partitions/keys, traversal, cycle and dedup semantics. |
| OrderedDfs | Root/edge completeness, partitions and ordered edge identity. |
| Rows, Hosted | Registered request sources, parameter/host identity and request/after-file scope; Hosted cannot remain a `WrongOperation` placeholder. |

Acceptance extends existing contract tests and the ownership matrix:

1. Execute every kind through the production boundary in streaming and collected modes. Check exact ordered bags, empty/malformed inputs, partial facts/absence, provenance, execution errors and emitter prefixes; fixture dispatch must be exhaustive.
2. Exercise mixed graphs, sources and frontiers. Count eligible shared work once; prove constrained work still uses the same scheduler. Exercise source/binding/coverage/arena/graph changes and release after success, error and cancellation.
3. Include negative controls for missing registration, forbidden cache/scheduler access, wrong shared identity, incomplete-result reuse, changed order/multiplicity, ignored emitter failure and retained per-file state. Tests must detect the deliberate violation rather than pass through fallback.
4. Preserve complete Django report parity and record measured runtime/retention separately. Ownership completion is not proof of a subsecond target or default-on performance benefit.

Write acceptance first, replace the boundary and all 24 implementations in one worktree, then build/gate the complete batch once with serialized builds and the required peer review. No incremental six-kind pilot lands. A measured DAG scheduling/retention strategy may remain switchable; the all-kind contract and common execution owner are unconditional. Update matrix cells only after their executable ownership proof passes.

## Relation representation: implemented boundary and measured checkpoint

This boundary describes how rows are represented, not how a language produces semantic facts. It covers **Stored, Selected, Tree and Computed together**; no representation retains an optional legacy execution path. The interface below records the accepted design; the measured implementation is linked at the end of this section.

Today schema/count/cell/row/materialization dispatch lives in `domain/table_model/service.rs`; a second representation enum and validation/access dispatch lives in `domain/operator/contract/views.rs`. Admission is repeated by `application/file_dag/execution.rs`, `invocation.rs` and `selected.rs`; index algorithms have stored/selected/computed entry points; `complete_for_reuse`, stored-selection/index services and graph-cache services independently decide eligibility. Native tree access has additional shape/acceptance assumptions. These are representation semantics and engine policy mixed across call sites, not language-provider semantics.

Place the mandatory interface beside implementing domain items, following `OperatorsItem`: `domain/table_model/collections/representations/interface.rs`, with stored/selected/tree/computed item modules and one exhaustive registry. Supporting domain types remain in the existing table/coverage/symbol concepts. Application owns admission, resource selection and generation state.

```rust
trait RelationItem: Sync {
    // Once per immutable logical view, not per consumer or row. Schema errors
    // remain declared/deferred when the operator's existing demand gate requires it.
    fn prepare<'file>(
        &self,
        input: RepresentationInput<'file>,
    ) -> Result<RelationContract<'file>, RepresentationError>;

    // Mandatory finite bulk access preserves column, selection, native-tree
    // and row-major algorithms; the engine chooses how to use the view.
    fn bulk<'read>(
        &self,
        view: &'read RepresentationLease<'_>,
    ) -> BulkAccess<'read>;

    fn cell(
        &self,
        view: &RepresentationLease<'_>,
        row: RowIndex,
        column: ColumnIndex,
    ) -> Result<CellRead, RepresentationError>;

    fn row<'read>(
        &self,
        view: &'read RepresentationLease<'_>,
        row: RowIndex,
        decode: &'read mut [Value], // fixed-width storage supplied by the engine
    ) -> Result<RowRead<'read>, RepresentationError>;
}

fn registered(kind: RepresentationKind) -> &'static dyn RelationItem;
// RepresentationKind::ALL and its exhaustive match cover all four variants.
// BulkAccess = StoredColumns | SelectedColumns | NativeTree | ComputedCells;
// each variant borrows its existing columns/positions/tree or cells+kinds+count.
// There is no scalar-only/unsupported bulk fallback.
// CellRead = Value(Value) | MissingPhysical.
// RowRead = Borrowed(&[Value]) | Decoded(&[Value]) | MissingPhysical.
```

`RepresentationInput` is the finite borrowed descriptor for all four variants. `RelationContract` declares schema, logical cardinality (including zero-width bags), physical access shape, source/projection mapping, symbol provenance and the representation's missing-physical-row semantics. It does not choose whether to materialize, index, cache, retain or skip work. `RepresentationLease` supplies immutable generation-bound storage only: no `Feed`, `FileIndexes`, mutable memo, retained `Arc`, provider handle or generation mutation. `cell`/`row` perform only representation decoding; they cannot acquire an index or schedule another input. Metadata and reusable decode capacity are prepared outside row loops.

`bulk` is required for every item and exposes existing immutable physical structure, including selected row IDs/projection and native tree form. It contains no retained ownership or cache policy. Index, sparse-join, column and tree algorithms consume these typed views directly; scalar callbacks are available for semantics that need them, never a mandatory replacement for bulk execution.

The application file owner admits a descriptor at the operator contract's existing demand/error boundary and returns an opaque borrowed `AdmittedRelation<'file>`. Its constructor and proof cache are private to the engine. Primitive input views are read-only borrows of that admitted handle. The owner separately acquires requested indexes and native batch views; existing scalar/column/tree algorithms remain available behind this same owner. Dropping the file owner releases admissions/indexes before resident facts. Computed views bind their immutable producer result and symbol provenance, rather than pretending a file ID proves identity.

Admission distinguishes an engine producer's construction proof from an external/unproved descriptor. Validated resident columns, bounded engine-produced selections, admitted native trees and successful typed computed publication can mint proofs without rescanning every cell. External/test descriptors undergo the appropriate structural checks once at demand; a proof is never inferred solely from a representation tag or an asserted provider flag. The existing source/derived-symbol and partial-coverage checks remain independent.

Admission keys bind the current owner generation, immutable resident/producer population, exact schema, and exact ordered projection. For Selected, use the retained shared row-position population and ordered columns/kinds, not a newly minted `StoredSelection` wrapper address; retaining the population prevents pointer reuse while a proof is live. Exact identity checks follow any hash lookup. No global cache or per-row identity hashing is introduced. Successful admission is reused across consumers of that view, while physical completeness and semantic coverage remain separate: partial positive facts can be read, but absence-dependent computation and successful-result sharing still require the operator's declared evidence.

| Representation | Item semantics | Engine admission and preserved behavior |
| --- | --- | --- |
| Stored | Decode resident typed columns; preserve physical row and duplicate order. | Validate immutable schema/population once; retain legacy ragged readable-row fallback at the original demand boundary without granting a complete-population proof. |
| Selected | Map ordered/repeated logical row IDs and ordered/repeated projected columns onto the source. | Check projection kinds/columns before normalization; admit valid positions once per generation/projection. Missing physical rows retain existing skip/fallback and predicate-error timing; malformed metadata never gains a trusted proof. |
| Tree | Decode Elements/Edges, sentinel/root offsets and existing source-symbol/span values. | Validate actual tree storage, form and accepted/recovered coverage once; native empty gates, traversal order and source provenance remain unchanged. `validate = true` is not sufficient proof. |
| Computed | Borrow row-major cells without copying; honor explicit row count, even at width zero. | Refuse unreadable declared rows/overflow instead of silently shortening output; publish immutable results only after the existing success/prefix boundary and bind producer generation. |

Migration is one coherent all-four batch: move duplicate schema/access algebra into representation items; replace raw/sanitized parallel dispatch with admitted read views; route scalar, selected-index, native-tree and column backends through the same admissions; then remove superseded validators only after their behavior is exercised through the new boundary. Representation-specific index construction remains a pure domain algorithm; application decides acquisition, bounds, sharing and release. No independently scheduled fallback may bypass admission.

Acceptance extends existing executable contracts, not string checks: table-model borrowed-computed/tree/compact-materialization tests; `complete_source_contract_covers_native_and_stored_without_promoting_partial_or_mutable_rows`; selected duplicate/projection and malformed-population tests; strict computed-count/selected-kind production negatives; graph exact-binding/projection/generation tests; stored-index drop/reacquisition and budget tests. Add an exhaustive four-variant production-route fixture with independent expected cells/order/errors, a counted many-consumer admission test, same-file-ID/new-generation invalidation, reordered/duplicate projection and shared-position/different-projection controls. Preserve ragged fallback, empty-before-error and emitted-prefix controls. A legitimate production trait implementation must compile before negatives assert missing required methods/registry arms, forbidden scheduling/cache acquisition and escaping generation borrows. Tests must exercise native and column routes, not only catalog entries.

The intended avoided work is repeated representation admission and decoding-policy discovery across consumers, including Selected's position-by-projection scans. Index construction, traversal and semantic fact production are separate costs. The measured Django result below has exact report parity and mixed phase timings; it does not isolate which cost changed.


### Relation and layer migration checkpoint

The relation batch implements the mandatory representation trait for Stored, Selected, Tree, and Computed together. Application owns private admission/publication state and resident population epochs; only the actual selector producer can mint a trusted selected result. Stable partial populations may reuse structural proofs without gaining absence/completeness authority. DAG delivery retains representation handles and prepared physical recipes instead of requiring every node result to become a Table. Provider `Stable` promises cover `keep`; descriptors are invalidated before release/refill, and unstable providers do not retain borrowed alias results.

Runtime Feed, PrimitiveExecutor, DagExecution, pipeline/Stepper, and rule/cross-file orchestration move to application. Domain retains representation semantics, pure plan/index/traversal algorithms, immutable contract data, and the borrowed Reading port. Engine-driven behavioral tests move with orchestration; domain tests do not receive an application import exception.

The [source-bound relation receipt](../../languages/python/benchmarks/linters/ruff/results/relation-contract-2026-11-02/README.md) records 463 passing core tests, seven passing capability controls, and passing SDK, Python, and benchmark library suites. Its frozen Django cold, warm, and retained-edit reports are byte-identical to the required-live reports. Cold wall time was 222.59 s versus 244.70 s for the required-live worker, with 1.84% lower peak RSS in one observation; warm was 3.29% slower and retained edit 4.04% faster. The later [exact-head 413-rule CI replay](../../languages/python/benchmarks/linters/ruff/results/ci-24274-full-inventory-2026-11-02/README.md) reports 383 PASS and 30 FAIL. Stable performance acceptance and full rule parity remain open.

The next 12-class derived-view acquisition contract is source work in an isolated tree. Its coherent core gate, full Django measurement, and remote checks are pending. Its draft release policy retains a view while any dependent population is pinned; it does not claim exact per-consumer last-use release. No checkbox above or measured relation result is credited to that untested batch.
