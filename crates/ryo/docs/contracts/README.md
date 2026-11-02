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
| Primitive schema and evaluator | ☑ | ☐ | ☐ | ☑ | ☐ | Some primitives still allocate and manage operator-local indexes/scratch directly. Registration covers six of 24 kinds; global policy is not uniformly declared. |
| Operator inputs and absence | ☐ | — | — | ☐ | — | Implementors still add per-kind dependency, trust, feature, and provenance branches in several engine services. |
| Ordinary plan and frontier | ☑ | ☑ | ☑ | ☑ | ☑ | Engine owns ordering, rule-local tables, frontier handover, and opt-in sharing; complete DAG coverage across all operators is still open. |
| Request-wide file graph | ☑ | ☑ | ☑ | ☑ | ☑ | The opt-in application DAG owns supported node scheduling and result lifetime; other operators retain ordered residual execution. It remains disabled by default. |
| Stored selections and indexes | ☑ | ☑ | ☑ | ☑ | ☑ | Engine owns predicate/index state, bounds, fallback, and release; provider only supplies complete resident columns. |
| Graph adjacency and traversal | ☑ | ☑ | ☑ | ☑ | ☑ | Engine owns typed per-file graph views and memo. Alternate storage remains unmeasured. |
| Persistent lint snapshot | ☑ | ☑ | ☑ | ☐ | ☐ | Engine chooses digests/reuse; outbound adapter owns framing, snapshot file handles, and atomic byte replacement behind the port. |
| Outbound reports and jobs | ☑ | — | ☑ | ☑ | ☐ | Engine owns reports and visit order; scheduler adapter still owns executor resources behind its port. |

The open ownership cells identify code that a new implementor must still coordinate. A provider supplies facts and completeness evidence; a primitive supplies typed row semantics. The engine should validate inputs and outputs, schedule their dependencies, share safe work, invalidate it, and release each file generation. Incomplete performance or type-safety evidence is named in the last column and does not by itself turn an engine-owned cell into an implementor-owned cell.

## Executable evidence and open gates

- [Primitive registry and conformance](../../crates/core/src/domain/operator/contract_test.rs) exercises six registered kinds, including ordered bags, errors, and counted scaling. The other 18 kinds still need the same behavioral contract; a catalog match alone is insufficient.
- [Source readiness](../../crates/core/src/domain/table_model/service_test.rs) distinguishes stored/native complete rows from partial or recovered input. [QuerySymbols tests](../../crates/language/src/domain/symbols/service_test.rs) protect source versus derived symbol provenance.
- [Request graph tests](../../crates/core/src/domain/file_program/service_test.rs) cover identity, successful cut reuse, failure, last-consumer release, and demanded ancestor reads. The [scoped graph receipt](../../languages/python/benchmarks/linters/ruff/results/file-program-graph-2026-11-02/summary.json) proves one 12-rule Join reuse but a 1.63× full-Django cold slowdown on the first overlay. The demand-mask follow-up skips 44 ancestor steps in the one-file trace but its complete cold run remains slower at 435.77 s versus the retained 248.62 s graph-off run (1.75×); neither single observation establishes stable component attribution.
- The separate [application DAG paired receipt](../../languages/python/benchmarks/linters/ruff/results/file-dag-execution-2026-11-02/summary.json) used one sealed worker and fresh caches: 211.50 s graph-on versus 255.31 s graph-off on frozen Django, with byte-equal complete reports. It is one scoped observation, not a default-on or general performance claim.
- The [pipeline Tastemaker spec](../../../../tastemaker/specs/pipeline/0.1.0/spec.toml) currently checks structure and test registration. A string containing “bounded” is not behavioral proof of bounded state. Product lifecycle tests with repeated unchanged and changed runs, measured retained state, and a deliberately unbounded negative control are required before a bounded-lifetime checkbox can be credited to that spec.

The [Ryo design](../README.md) is the architectural source. Update this table only when a passing executable contract changes ownership. Keep source-bound fixture, lifecycle, and performance receipts separate; do not mark an opt-in strategy production-ready from one-file parity alone.

## Mandatory operator boundary: accepted implementation decision

The next implementation batch replaces the pilot with one mandatory contract for all 24 kinds. This is a target decision; the current-ownership matrix above remains unchanged until executable evidence passes. No new plan ledger is needed.

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
