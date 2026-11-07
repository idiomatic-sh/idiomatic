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

26 kinds, 8 with more than one implementation.

| kind | implementations | locations | port |
|---|---|---|---|
| Project | 7 | execution:application/file_dag/frontiers; execution:application/file_dag/program/facade_runtime; execution:domain/frontiers; execution:domain/native_positions; execution:domain/operation_schema; execution:domain/trust; operators:domain/project | RowSink |
| Join | 6 | execution:domain/lowering; execution:domain/native_positions; execution:domain/operation_schema; execution:domain/operator/contract; execution:domain/tree_join; operators:domain/equality_join | PositionJoinDriver |
| Select | 6 | execution:application/file_dag; execution:application/file_dag/derived/owner/selection; execution:domain/native_positions; execution:domain/operation_schema; execution:domain/selections; operators:domain/select | direct |
| Rows | 5 | execution:application/file_dag; execution:application/file_dag/program/facade_runtime; execution:domain/completion; execution:domain/native_positions; execution:domain/operator/collections/operators | OperatorsItem, RowSink, SetPositionsDriver |
| Axis | 4 | execution:application/file_dag; execution:domain/lowering; execution:domain/native_positions; execution:domain/operation_schema | direct |
| Union | 3 | execution:application/file_dag/program/facade_runtime; execution:domain/native_positions; execution:domain/operation_schema | RowSink, SetPositionsDriver |
| Capture | 2 | execution:application/file_dag; execution:domain/executable_program | direct |
| Replace | 2 | execution:application/file_dag/derived/owner/graphs; operators:domain/replace | direct |
| Antijoin | 1 | execution:domain/native_positions | SetPositionsDriver |
| Closure | 1 | execution:application/file_dag | direct |
| Cycles | 1 | execution:application/file_dag | direct |
| Rank | 1 | operators:domain/rank | direct |
| SourceMatches | 1 | execution:domain/operation_schema | direct |
| SpanBounds | 1 | execution:domain/operation_schema | direct |
| Difference | 0 | - | direct |
| Extend | 0 | - | direct |
| FitsRendered | 0 | - | direct |
| GroupCount | 0 | - | direct |
| GroupDigest | 0 | - | direct |
| Hosted | 0 | - | direct |
| Intersection | 0 | - | direct |
| OrderedDfs | 0 | - | direct |
| OrderedMatch | 0 | - | direct |
| RenderSyntax | 0 | - | direct |
| SpanBetween | 0 | - | direct |
| Words | 0 | - | direct |

### Derived kinds (DerivedKind::ALL, grep: macro list)

13 kinds, 1 with more than one implementation.

| kind | implementations | locations | port |
|---|---|---|---|
| Adjacency | 2 | execution:application/file_dag; execution:application/file_dag/derived/owner/acquisition/graph/construction | direct |
| Membership | 1 | execution:application/file_dag/derived/owner | direct |
| PartitionOrder | 1 | execution:application/file_dag | direct |
| CandidatePostings | 0 | - | direct |
| ClosureAdjacency | 0 | - | direct |
| CycleAdjacency | 0 | - | direct |
| DistinctRepresentatives | 0 | - | direct |
| Equality | 0 | - | direct |
| NativeLookup | 0 | - | direct |
| OrderedDfs | 0 | - | direct |
| OrderedKeys | 0 | - | direct |
| PredicateMatches | 0 | - | direct |
| RawKeyOrder | 0 | - | direct |

### Relation members (relations catalog, RyoQL: member directories)

69 kinds, 1 with more than one implementation.

| kind | implementations | locations | port |
|---|---|---|---|
| Function | 2 | core:inbound/use_cases/lint; execution:application/file_dag/program/facade_runtime | RowSink |
| Scope | 1 | execution:application/file_dag | direct |
| SemanticChild | 1 | language:domain/answers | direct |
| SemanticSyntax | 1 | language:domain/answers | direct |

### Walk and build routes (WalkRoute, WalkBarrier, Route, BuildRoute; RyoQL enum members)

22 kinds, 5 with more than one implementation.

| kind | implementations | locations | port |
|---|---|---|---|
| Column | 4 | execution:domain/executable_program; execution:domain/operator/collections/operators; execution:domain/positions; execution:domain/tree_join | OperatorsItem |
| Observation | 3 | execution:application/file_dag; execution:application/file_dag/program/facade_runtime; execution:domain/source_partitions | direct |
| Selection | 3 | execution:application/file_dag/derived/owner; execution:application/file_dag/derived/owner/acquisition/graph; execution:domain/executable_program | direct |
| Continuation | 2 | execution:application/file_dag/program/runtime/service/invocation; execution:domain/operation_schema | Emit |
| Shared | 2 | execution:application/file_dag/derived/ledger; execution:application/file_dag/derived/owner/acquisition/selection | direct |
| Barrier | 1 | execution:domain/shared_walk | direct |
| NativePositions | 1 | execution:domain/executable_program | direct |
| OrderedExistence | 1 | execution:domain/executable_program | direct |
| Representation | 1 | execution:application/file_dag | direct |
| ClosurePartition | 0 | - | direct |
| DependentInput | 0 | - | direct |
| DerivedOrOrdered | 0 | - | direct |
| Generic | 0 | - | direct |
| GraphAdjacency | 0 | - | direct |
| GraphDomain | 0 | - | direct |
| OwnerAcquire | 0 | - | direct |
| RequestOwnedSource | 0 | - | direct |
| RowSemantics | 0 | - | direct |
| SelectorCandidate | 0 | - | direct |
| SelectorDistinct | 0 | - | direct |
| SelectorPostings | 0 | - | direct |
| SelectorPredicate | 0 | - | direct |

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
| finish | core:outbound/digest/adapters/blake3 | adapter.rs |
| finish | execution:domain/shared_walk | service.rs |
| finish_file | core:domain/recording | service.rs |
| fits | execution:domain/operator/collections/operators | item.rs |
| render_finding | core:domain/render | service.rs |
| render_placed | core:domain/render | service.rs |
| render_rows | core:domain/render | service.rs |
| rendered_limit | core:domain/render | service.rs |
| rendered_of | core:domain/keeping | service.rs |

### Kinds with more than one implementation and no port

- Operator kinds: `Axis` (4 implementations)
- Operator kinds: `Capture` (2 implementations)
- Operator kinds: `Replace` (2 implementations)
- Operator kinds: `Select` (6 implementations)
- Derived kinds: `Adjacency` (2 implementations)
- Walk and build routes: `Observation` (3 implementations)
- Walk and build routes: `Selection` (3 implementations)
- Walk and build routes: `Shared` (2 implementations)

Ports found (workspace traits with two or more impls): `ComparisonsItem` (7), `CorpusPort` (2), `DerivedUseCase` (2), `Emit` (5), `ExecutionPort` (3), `FactsFinish` (4), `FactsPort` (2), `FactsWorker` (2), `JoinDriver` (3), `LintStorePort` (3), `OperatorsItem` (26), `OperatorsUseCase` (2), `PositionJoinDriver` (3), `PredicateProgram` (2), `ProjectRootExecution` (3), `ReadItem` (3), `ReadStore` (3), `RepresentationsItem` (6), `RowCells` (3), `RowSink` (2), `SchedulerPort` (2), `Sealed` (70), `SetPositionsDriver` (2), `StrategiesItem` (2), `Syntax` (4), `ValueTexts` (3), `Visiting` (3).

<!-- implementations:end -->
