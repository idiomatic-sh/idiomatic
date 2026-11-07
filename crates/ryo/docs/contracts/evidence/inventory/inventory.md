# Concept inventory of the ryo workspace

Produced from the source by RyoQL (`ryo 0.1.2`) at commit 80f473c67d. The queries and the commands that run them are in the [README](README.md).

## Summary

The workspace holds 401 concept directories in 12 crates. 39 public type or trait names are defined in more than one crate, and 7 public function names in more than one concept of a layer. The 15 names defined in the most crates (all are in two or three):

1. `ByteOffset (benchmark, sdk, values)`
2. `BoundObservationInput (sdk, values)`
3. `Cell (sdk, values)`
4. `ColumnName (sdk, values)`
5. `Comparison (benchmark, benchmark_contracts)`
6. `ContextManifest (benchmark, benchmark_contracts)`
7. `Example (sdk, values)`
8. `FileObservation (benchmark, benchmark_contracts)`
9. `Finding (benchmark, derived)`
10. `Function (relations, values)`
11. `Holds (relations, sdk)`
12. `HostColumn (sdk, values)`
13. `HostColumnName (sdk, values)`
14. `HostKind (sdk, values)`
15. `LanguageBuild (core, sdk)`

Most pairs are sdk against values: both define `Cell`, `ColumnName`, `HostKind`, `Example` and `Unread`. The benchmark against benchmark_contracts pairs (`Comparison`, `ContextManifest`, `FileObservation`) are defined in both.

The duplicate that the kernels proposal would have missed is a function, and shows in the any-visibility table of [duplicates](duplicates.md): `count_groups` and `digest_groups` are declared in both operators (domain/reduction) and execution (domain/table_groups), as are `select`, `matches`, `values`, `cell`, `row`, `finish` and `same`.

No crate imports execution's domain, application or outbound layers. Only core imports execution, and only its inbound layer (use_cases/execution 127 imports, use_cases/metadata 85, use_cases/publication 3), see [execution imports](execution-internals.md). Execution itself imports operators 64 times.

No concept directory is hollow: none has zero production lines. 36 hold production lines but write no item with a visibility other than private, and are listed in [hollow](hollow.md): the adapters directories of core outbound language and query_language, relations domain/kinds, and 33 directories under execution application/file_dag (the issuance, runtime/service, kernel_session, facade_runtime, admission and preparation leaves).

## Sections

- [concepts](concepts/): one file per crate: crate, layer, concept, production lines.
- [items](items/): one file per crate: the public items each concept defines.
- [imports](imports/): one file per crate: what each concept imports and what imports it.
- [duplicates](duplicates.md): names defined in more than one crate or concept.
- [hollow](hollow.md): concepts with no production lines or no declared item.
- [execution imports](execution-internals.md): crates that import execution.
- Implementations per kind: the last section below.

<!-- implementations:begin -->
## Implementations per kind

Kinds come from RyoQL rows; the two macro-declared lists are read by grep. An implementation is a concept directory declaring a free function whose name carries the kind's name (a name match, not a call graph). A kind is behind a port when its implementing files hold `impl <Trait> for` for a workspace trait with two or more impls.

### Operator kinds (OperationKind, grep: macro list)

26 kinds, 10 with more than one implementation.

| kind | implementations | locations | port |
|---|---|---|---|
| Project | 5 | execution:application/file_dag/program/facade_runtime; execution:domain/frontiers; execution:domain/operation_schema; execution:domain/trust; operators:domain/project | direct |
| Rows | 5 | execution:application/file_dag; execution:application/file_dag/program/facade_runtime; execution:application/file_dag/program/file_session; execution:domain/contributor_rows; operators:domain/table_rows | RowSink |
| Replace | 3 | execution:application/file_dag/derived/owner/graphs; execution:domain/describing; operators:domain/replace | direct |
| Select | 3 | execution:domain/completion; execution:domain/operation_schema; operators:domain/select | direct |
| Axis | 2 | execution:application/file_dag; execution:domain/operation_schema | direct |
| Capture | 2 | execution:application/file_dag/route_counts; execution:domain/operator/contract | RowDriver, TextRowsEvaluation |
| Join | 2 | execution:domain/operation_schema; operators:domain/equality_join | direct |
| SourceMatches | 2 | execution:domain/describing; execution:domain/operation_schema | direct |
| SpanBounds | 2 | execution:domain/describing; execution:domain/operation_schema | direct |
| Union | 2 | execution:domain/completion; execution:domain/operation_schema | direct |
| Closure | 1 | operators:domain/equality_join | direct |
| Cycles | 1 | operators:domain/cycles | direct |
| FitsRendered | 1 | execution:domain/describing | direct |
| Hosted | 1 | operators:domain/evaluation | direct |
| OrderedDfs | 1 | execution:domain/describing | direct |
| OrderedMatch | 1 | execution:domain/describing | direct |
| Rank | 1 | operators:domain/rank | direct |
| RenderSyntax | 1 | execution:domain/describing | direct |
| SpanBetween | 1 | execution:domain/describing | direct |
| Antijoin | 0 | - | direct |
| Difference | 0 | - | direct |
| Extend | 0 | - | direct |
| GroupCount | 0 | - | direct |
| GroupDigest | 0 | - | direct |
| Intersection | 0 | - | direct |
| Words | 0 | - | direct |

### Derived kinds (DerivedKind::ALL, grep: macro list)

9 kinds, 0 with more than one implementation.

| kind | implementations | locations | port |
|---|---|---|---|
| Adjacency | 1 | execution:application/file_dag/derived/owner/acquisition/graph/construction | direct |
| Equality | 1 | derived:domain/footprint | direct |
| Membership | 1 | execution:application/file_dag/derived/owner | direct |
| OrderedDfs | 1 | execution:domain/describing | direct |
| ClosureAdjacency | 0 | - | direct |
| CycleAdjacency | 0 | - | direct |
| NativeLookup | 0 | - | direct |
| OrderedKeys | 0 | - | direct |
| PartitionOrder | 0 | - | direct |

### Relation members (relations catalog, RyoQL: member directories)

69 kinds, 1 with more than one implementation.

| kind | implementations | locations | port |
|---|---|---|---|
| Function | 2 | core:inbound/use_cases/lint; execution:application/file_dag/program/facade_runtime | RowSink |
| Scope | 1 | execution:application/file_dag/route_counts | direct |
| SemanticChild | 1 | language:domain/answers | direct |
| SemanticSyntax | 1 | language:domain/answers | direct |

### Walk and build routes (WalkRoute, WalkBarrier, Route, BuildRoute; RyoQL enum members)

11 kinds, 2 with more than one implementation.

| kind | implementations | locations | port |
|---|---|---|---|
| Continuation | 3 | execution:application/file_dag/invocation; execution:application/file_dag/program/runtime/service/invocation; execution:domain/operation_schema | Emit |
| Observation | 3 | execution:application/file_dag; execution:application/file_dag/program/facade_runtime; execution:domain/source_partitions | direct |
| Barrier | 1 | execution:domain/shared_walk | direct |
| OrderedExistence | 1 | execution:domain/executable_program | direct |
| Representation | 1 | execution:application/file_dag/route_counts | direct |
| DependentInput | 0 | - | direct |
| DerivedOrOrdered | 0 | - | direct |
| Generic | 0 | - | direct |
| RequestOwnedSource | 0 | - | direct |
| RowSemantics | 0 | - | direct |
| Shared | 0 | - | direct |

### Frontier decoders, finishers, renderers and text gates (functions by role word)

Free functions whose name starts with decode, finish, render, fits, gate, escape or carry, in execution, query_language and core. They are listed by name; whether each is behind a port is not decided by a name match.

| function | concept | file |
|---|---|---|
| call_fits_rendered | query_language:domain/rendering | service.rs |
| call_render_syntax | query_language:domain/rendering | service.rs |
| carry | execution:domain/finish | service.rs |
| decode_input | execution:domain/frontiers | service.rs |
| escape | execution:domain/finish | service.rs |
| escape_cross_publication | execution:domain/finish | publication.rs |
| escape_frame | execution:domain/finish | service.rs |
| finish | execution:domain/shared_walk | service.rs |
| finish | execution:outbound/digest/adapters/blake3 | adapter.rs |
| finish_file | core:domain/recording | service.rs |
| fits_rendered | execution:domain/describing | constants.rs |
| render_finding | core:domain/render | service.rs |
| render_placed | core:domain/render | service.rs |
| render_rows | core:domain/render | service.rs |
| render_syntax | execution:domain/describing | constants.rs |
| rendered_limit | core:domain/render | service.rs |
| rendered_of | core:domain/keeping | service.rs |

### Kinds with more than one implementation and no port

- Operator kinds: `Axis` (2 implementations)
- Operator kinds: `Join` (2 implementations)
- Operator kinds: `Project` (5 implementations)
- Operator kinds: `Replace` (3 implementations)
- Operator kinds: `Select` (3 implementations)
- Operator kinds: `SourceMatches` (2 implementations)
- Operator kinds: `SpanBounds` (2 implementations)
- Operator kinds: `Union` (2 implementations)
- Walk and build routes: `Observation` (3 implementations)

Ports found (workspace traits with two or more impls): `AxisEvaluation` (2), `CaptureOutput` (2), `ClosureEvaluation` (2), `ComparisonsItem` (7), `CorpusPort` (2), `CyclesEvaluation` (2), `DfsEvaluation` (2), `Emit` (4), `Encode` (18), `ExecutionCompilation` (2), `ExecutionPort` (3), `FactsFinish` (4), `FactsPort` (2), `FactsWorker` (2), `JoinEvaluation` (2), `LintStorePort` (3), `MatchingPort` (2), `OrderedMatchEvaluation` (2), `PrepareUseCase` (2), `ProgressObserver` (2), `ProjectRootExecution` (3), `ReadItem` (3), `ReadStore` (3), `RenderedOutput` (2), `RepresentationsItem` (4), `RowCells` (3), `RowDriver` (8), `RowSink` (2), `SchedulerPort` (2), `Sealed` (70), `SetRowsEvaluation` (2), `Syntax` (4), `TableRowOutput` (2), `TextRowsEvaluation` (2), `ValueTexts` (3), `Visiting` (3), `WordOutput` (2).

<!-- implementations:end -->
