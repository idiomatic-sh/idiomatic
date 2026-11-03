# Extension ownership contracts

## 1. Ownership matrix

`☑` means the engine is the sole authority for that cell on the named **actual route**, with source and test evidence. `☐` means split or unproved ownership; `—` means the axis does not apply. A checked cell is not a speed claim. The numbered rows cover 34 audited contract families; letters keep narrow proven subroutes separate from broader unproved work. The crate column identifies the current semantic home and any proposed isolation; it does not transfer the engine's scheduling or storage authority. The [common-owner tests and review](../../languages/python/benchmarks/linters/ruff/results/common-owner-graph-2026-11-03/README.md) cover the scoped selection, derived-view, and graph rows; fanout and remaining terminal paths stay open.

| Contract / route | Capability crate (actual → proposed) | Scheduling | Caching | Invalidation | Data ownership | Lifetime | Remaining implementor responsibility |
| --- | --- | :---: | :---: | :---: | :---: | :---: | --- |
| 1. Typed relation catalog (65 relations) | `ryo-language` | ☑ | — | — | ☑ | ☑ | Providers supply values through the catalog; retain its typed identity. |
| 2. Provider answers and absence | `ryo-language` + providers | ☐ | — | ☐ | ☐ | ☐ | Enumerate every reader and typed absence path, including supported but unavailable facts. |
| 3. Optional capability declarations | `ryo-language` + providers | ☐ | — | ☐ | — | — | Make declaration and refusal explicit; audit default trait methods. |
| 4. Readiness, completion and file model | `ryo-language` + `ryo-core` | ☐ | ☐ | ☐ | ☐ | ☐ | Engine owns preparation order; providers declare required model and completion. |
| 5. Parse route and explicit assignment | `ryo-language` + providers | ☑ | — | ☑ | ☐ | ☐ | Preserve recognized language, read outcome, parser state and scratch boundaries. |
| 6a. Typed complete and partial fact storage | `ryo-language` + `ryo-core` | ☑ | ☑ | ☑ | ☑ | ☑ | Existing proof covers typed storage and provider-certified partial facts. |
| 6b. Trust, rule gating and persisted incompleteness | `ryo-core` | ☐ | ☐ | ☐ | ☐ | ☐ | Audit Unknown propagation through every consuming rule and persisted result. |
| 7. Project and manifest relations | `ryo-language` + providers | ☐ | ☐ | ☐ | ☐ | ☐ | Separate manifest semantics from engine once-per-run reads and consulted-file digests. |
| 8. Engine-given relations | `ryo-language` + `ryo-core` | ☐ | ☐ | ☐ | ☐ | ☐ | Engine owns filesystem file, line count and executable facts and corpus walk. |
| 9. Host-declared relations | `ryo-values` + `ryo-core` | ☐ | ☐ | ☐ | ☐ | ☐ | Preserve host schema and rows while engine controls request-source admission. |
| 10a. Query grammar and typed plan | `ryo-query_language` + `ryo-values` | ☑ | — | ☑ | ☑ | ☑ | Preserve typed plan meaning and compile errors. |
| 10b. Compile-result cache | `ryo-core` | ☐ | ☐ | ☐ | ☐ | ☐ | Prove cache identity and retention independently of grammar ownership. |
| 11. Relation lookup, kinds and features | `ryo-language` + `ryo-core` | ☐ | — | ☐ | — | — | Refuse undeclared relations, kinds and features before reads. |
| 12. Primitive operator semantics (26 kinds) | `ryo-core` → proposed `ryo-operators` | ☐ | ☐ | ☐ | ☐ | ☐ | Move every row meaning through one port; row 13 retains narrower common-input proof while timing, leases and publication stay in engine. |
| 13. Operator inputs and absence | `ryo-values` + `ryo-core` | ☑ | — | — | ☑ | — | Preserve declared absence, bindings and lazy error order. |
| 14a. Scoped derived equality, predicate and distinct views | `ryo-core` → proposed `ryo-derived` semantics | ☐ | ☑ | ☑ | ☑ | ☑ | Existing common-owner proof covers Equality, PredicateMatches and DistinctRepresentatives only. |
| 14b. Remaining derived kinds and traversal | `ryo-core` → proposed `ryo-derived` semantics | ☐ | ☐ | ☐ | ☐ | ☐ | Audit CandidatePostings, Adjacency, Membership, PartitionOrder, ClosureAdjacency, CycleAdjacency, OrderedKeys, OrderedDfs and NativeLookup; all 12 kinds are now named. |
| 15. Candidate fanout | `ryo-core` | ☐ | ☐ | ☐ | ☐ | ☐ | Move ordered postings, policy decline, retained capacity and release into one owner. |
| 16. Stored selections and selectors | `ryo-core` | ☑ | ☑ | ☑ | ☑ | ☑ | Extend the proven owner route to all candidate consumers. |
| 17. Graph population and memo | `ryo-core` → proposed `ryo-derived` semantics | ☐ | ☑ | ☑ | ☑ | ☑ | Finish traversal demand; keep backing, memo budget and release in engine. |
| 18. Physical representations and publication | `ryo-core` | ☐ | ☐ | ☐ | ☐ | ☐ | Consolidate RelationOwner publish, admit and release for every representation. |
| 19. Predicates, comparisons and matching | `ryo-core` + matching adapter | ☐ | ☐ | ☐ | ☐ | ☐ | Separate comparison meaning from prepared matcher lifetime and request reuse. |
| 20a. Provider walk versus execution demand | `ryo-language` + `ryo-core` | ☑ | — | — | ☑ | — | Existing proof covers engine demand rows and provider-certified deferred-fact completeness. |
| 20b. Native and shared tree walk | `ryo-core` | ☐ | ☐ | ☐ | ☐ | ☐ | Prove every residual native route and shared-walk representation under the same evaluator contract. |
| 21a. Common ordinary frontier bag | `ryo-core` | ☐ | ☐ | ☐ | ☑ | ☐ | Existing proof covers the ordinary frontier representation only. |
| 21b. Five per-file routes and terminal output | `ryo-core` | ☐ | ☐ | ☐ | ☐ | ☐ | Consolidate `evaluate_file`, `with_graph`, `with_dag`, `with_executor` and `with_legacy_oracle`; prove Stepper, ordered bags and last release. |
| 22. Cross-file execution | `ryo-core` | ☐ | ☐ | ☐ | ☐ | ☐ | Prove frontier handover and materialization under one application owner. |
| 23a. Source bytes and spans | `ryo-language` + `ryo-core` | ☑ | — | ☑ | ☑ | ☑ | Engine owns stored spans; providers supply token coordinates. |
| 23b. Source and derived symbols | `ryo-language` + `ryo-core` | — | — | ☑ | ☑ | ☑ | Engine owns symbol storage; providers supply source-name meaning. |
| 24. Keyed semantic rendering | `ryo-language` + `ryo-core` | ☐ | ☐ | ☐ | ☐ | ☐ | Current SIM source has typed requests; close answer admission, per-rule incompleteness and lifetime. |
| 25. Source identity and read outcomes | `ryo-values` + `ryo-core` | ☐ | ☐ | ☐ | ☐ | ☐ | Unify generation, digest, re-read and stored snapshot decisions. |
| 26. Persistent lint store | `ryo-core` + SDK adapter | ☑ | ☑ | ☑ | ☐ | ☐ | Adapter still owns framing and file handles; preserve partial and unread records. |
| 27. Typed facts spill | `ryo-core` + facts adapter | ☐ | ☐ | ☐ | ☐ | ☐ | Engine owns spill admission and release; adapter owns external handles. |
| 28. Memory budget and admission | `ryo-core` + memory adapter | ☐ | ☐ | ☐ | ☐ | ☐ | Prove one budget and refusal authority across all resource families. |
| 29. Reporting and rows out | `ryo-core` + rows adapter | ☐ | — | ☐ | ☐ | ☐ | Preserve heading, row and summary order; prove output-handle release separately. |
| 30. Jobs and scheduler | `ryo-core` + scheduler adapter | ☐ | — | ☐ | — | ☐ | Prove worker boundaries and executor lifetime independently of reporting. |
| 31. Core inbound use cases | `ryo-core` | ☐ | — | — | — | ☐ | Audit all seven use cases and injected ports, including CLI composition. |
| 32. SDK and CLI composition | `ryo-sdk` + `ryo-cli` | ☐ | — | — | — | ☐ | Compose each port once at the edge; do not expose engine-private implementation. |
| 33. Shared benchmark harness | `ryo-benchmark` | — | — | — | — | — | Verify final 413-rule inventory and workload with source-bound receipts. |
| 34. Provider internals | ten provider crates | — | — | — | — | — | Parser sessions and source meaning stay local; audit each declared relation route. |

The counts come from the [65-relation catalog](../../crates/language/src/domain/relations/service.rs), [26 operation kinds and 25 public query operators](../../crates/values/src/domain/plans/types.rs), and [12 derived kinds](../../crates/core/src/domain/derived_views/types.rs). The [relation owner](../../crates/core/src/application/file_dag/relations.rs) and [five per-file entry routes](../../crates/core/src/application/file_dag/pipeline/service.rs) anchor the engine side. These inventories identify work to prove; they do not certify every route.

### Capability crate inventory (pending independent route audit)

This maps the matrix to the seven existing workspace crates, all ten language-provider members, and two proposed capability crates. Every ownership proof remains unchecked until the named routes and consumers are audited; a crate's existence or a compiling port does not close a cell above. `—` in Migration means no crate move is proposed, not an ownership proof. Dependencies shown are the intended direction, with proposed crates explicitly marked.

| Crate / status | Semantic responsibility and public contract | Allowed dependency direction | Engine owned schedule / cache / invalidation / data / lifetime | Migration | Ownership proof / evidence |
| --- | --- | --- | --- | :---: | :---: |
| `ryo-language` / existing | Typed base relation catalog, parse/project, partial facts, source names, syntax semantics | No internal Ryo crate dependency; no `ryo-core` | `ryo-core` owns demand and stored fact generations; provider owns parser state | — | ☐ Catalog and all reader routes need independent enumeration. |
| `ryo-values` / existing | Query plan, inputs, bindings and typed value meanings | `ryo-language`; no `ryo-core` | `ryo-core` owns prepared-plan admission and execution | — | ☐ Plan and input routes remain in the matrix. |
| `ryo-query_language` / existing | RyoQL compile/typecheck and plan construction | `ryo-language`, `ryo-values`; no `ryo-core` | `ryo-core` owns compile result retention and request execution | — | ☐ Grammar and plan boundary evidence above. |
| `ryo-operators` / proposed, isolated extraction | Portable primitive meanings through one typed inbound use case | `ryo-language`, `ryo-values`; no engine implementation imports | `ryo-core` retains primitive scheduling, scratch, publication and leases | ☐ | ☐ First Project/Append/Span extraction is not family closure. |
| `ryo-derived` / proposed | Pure derived relation/index/graph/traversal algorithms and typed requests | `ryo-language`, `ryo-values` only; no `ryo-operators` or engine owner | `ryo-core` retains admission, physical backings, memo, invalidation and last release | ☐ | ☐ Relations and acquisition paths require independent audit before this split is approved. |
| `ryo-core` / existing engine | Use cases, ordinary/native evaluation, terminal/frontier, selections, derived owner, fanout and persistence | `ryo-language`, `ryo-values`, `ryo-query_language`; proposed `ryo-operators` and `ryo-derived`; never provider internals | Sole intended authority for all five axes; the matrix marks actual proven routes only | ☐ | ☐ Terminal, native, traversal demand and fanout remain open. |
| `ryo-sdk` / existing edge | Public entrypoints and composition of injected ports | `ryo-core`, `ryo-language`, `ryo-values`, `ryo-query_language`; proposed `ryo-operators` only to construct the injected use case; no engine-private backends | Starts/ends scopes; engine owns execution resources | — | ☐ Adapter handles and query/lint entrypoints need route audit. |
| `ryo-cli` / existing edge | Commands and composition of injected use cases | `ryo-core`, `ryo-language`, `ryo-values`, `ryo-query_language`, ten optional providers; proposed `ryo-operators` only at composition; SDK only in dev dependencies | Starts jobs; engine owns execution resources | — | ☐ Job lifetime and command routes remain open. |
| `ryo-css` / existing provider | CSS typed facts and parser adapter | `ryo-sdk` only as an internal Ryo dependency; no engine internals | Engine owns storage/demand; provider owns parser session | — | ☐ Audit its reader/catalog routes. |
| `ryo-go` / existing provider | Go typed facts and parser adapter | `ryo-sdk` only as an internal Ryo dependency; no engine internals | Engine owns storage/demand; provider owns parser session | — | ☐ Audit its reader/catalog routes. |
| `ryo-html` / existing provider | HTML typed facts and parser adapter | `ryo-sdk` only as an internal Ryo dependency; no engine internals | Engine owns storage/demand; provider owns parser session | — | ☐ Audit its reader/catalog routes. |
| `ryo-javascript` / existing provider | JavaScript typed facts and parser adapter | `ryo-sdk` only as an internal Ryo dependency; no engine internals | Engine owns storage/demand; provider owns parser session | — | ☐ Audit its reader/catalog routes. |
| `ryo-json` / existing provider | JSON typed facts and parser adapter | `ryo-sdk` only as an internal Ryo dependency; no engine internals | Engine owns storage/demand; provider owns parser session | — | ☐ Audit its reader/catalog routes. |
| `ryo-markdown` / existing provider | Markdown typed facts and parser adapter | `ryo-sdk` only as an internal Ryo dependency; no engine internals | Engine owns storage/demand; provider owns parser session | — | ☐ Audit its reader/catalog routes. |
| `ryo-python` / existing provider | Python parser, typed base facts, recovery, syntax rendering | `ryo-sdk` only as an internal Ryo dependency; no engine internals | Engine owns storage/demand; provider supplies fact meaning and parser session | — | ☐ Complete/partial and keyed rendering ownership differ. |
| `ryo-rust` / existing provider | Rust typed facts and parser adapter | `ryo-sdk` only as an internal Ryo dependency; no engine internals | Engine owns storage/demand; provider owns parser session | — | ☐ Audit its reader/catalog routes. |
| `ryo-toml` / existing provider | TOML typed facts and parser adapter | `ryo-sdk` only as an internal Ryo dependency; no engine internals | Engine owns storage/demand; provider owns parser session | — | ☐ Audit its reader/catalog routes. |
| `ryo-typescript` / existing provider | TypeScript typed facts and parser adapter | `ryo-sdk` only as an internal Ryo dependency; no engine internals | Engine owns storage/demand; provider owns parser session | — | ☐ Audit its reader/catalog routes. |
| `ryo-benchmark` / existing shared harness | Inventory, receipt, comparison and performance protocol | `ryo-sdk`; no rule-specific evaluator ownership | Does not own engine execution cache or facts | — | ☐ Full 413-rule and final workload gates remain open. |

### Proposed remaining boundaries

These signatures describe the authority to consolidate; they are **proposed**, not implemented or ownership proof. Domain types state meaning. Application operations own scheduling, admission, cache identity and last release. Outbound ports supply external facts and effects. Engine handles, budgets, indexes and scheduler choices stay internal and never become language provider contracts.

| Family | Semantic contract | Proposed application operation and open route |
| --- | --- | --- |
| Terminal and frontier rows | Existing `Input` names a relation or step; output remains an ordered bag with explicit absence, partiality and error order. | An internal owner-issued `InputHandle` may identify a retained publication or a deferred step. `consume(handle, sink)` scans resident rows or streams the step without first materializing it; `resident(handle)` is available only when already retained or explicitly collected. `keep` is the explicit stream-to-bag boundary; `hand_over` materializes cross-file frontier inputs. Reconcile [`run`, `deliver`, `keep` and `hand_over`](../../crates/core/src/application/file_dag/pipeline/service.rs) without changing lazy errors, cancellation, emitted prefixes or last-consumer release. |
| Native evaluation | Typed tree rows, rule demand and completeness have the same meaning as ordinary operator input; there is one evaluator semantic contract. | Internal `evaluate_native(demand, tree, sink)` remains a strategy under that contract, not a second public evaluator API. The [shared walk and residual mask](../../crates/core/src/application/file_analysis/service.rs) currently produce `SharedClasses` verdicts; moving their results through the common consumption path is a target. The provider certifies recovery/partials; the engine computes native-tree completeness. |
| Derived views and traversal | Existing `ViewRequest`, graph demand and admitted input proof define the result; aliases retain one physical backing. | `acquire(request, proof) -> Result<Option<DerivedHandle>>` and `reach(graph, demand) -> Result<Option<RootAnswer>>` are application-owned admission and lifetime operations. A root absent from the graph, a stopped traversal, or an unshared result yields no completed root answer; traversal scratch can still feed the current ordinary route. Graph, index and memo storage layouts stay private. |
| Candidate fanout | A request specifies relation, column and literals; admitted population identity belongs to the engine proof. The answer is an ordered bag of positions, including a proven-empty bag. | `acquire_candidates(request, proof) -> Result<CandidateDecision>` distinguishes a leased answer, policy decline to ordinary selection, incomplete facts and construction error. Move [local choice, pointer-based identity, post-build decline and retained capacity](../../crates/core/src/application/file_dag/derived/selection/fanout.rs) under the common owner without changing bag order. |
| Keyed semantic answers | Public base `LanguagePort::parse` exposes `RoleWriting`, typed absence and `ReadInPart` partials. Current public SIM401 source (introduced at 4f300176, present at 5b26737) defines `RenderRequest`, `RenderOutcome`, `RenderError` and outbound `SemanticSession`; its typed `Unsupported` differs from file-level and parse errors of the same name. | `render_batch(file_generation, consuming_rule, requests) -> Result<Vec<RenderOutcome>>` returns answers in request order; generation is application cache identity, not provider input. Incomplete answers mark only consuming rules Unknown before publication, and persisted partial/unread records remain truthful. Prototype batch-local dedup and file-wide persistence bypass do not close answer admission/lifetime. |
| External resources | Separate outbound ports include corpus, digest, facts, language, matching, memory, query language, store, rows and scheduler. | The application opens and ends each file/request scope and retains only declared leases. Persisted completeness must remain truthful, including supported partial facts; Unknown never becomes a Known empty result. Adapters own external handles; the domain sees no file descriptor, thread/executor handle, cache or transaction type. |

Pure operator and derived-view semantics currently live inside `ryo-core`, so crate isolation is still open. Proposed cohesive capability crates are `ryo-operators` (primitive semantics) and `ryo-derived` (derived and traversal algorithms), each depending on `ryo-language`/`ryo-values`, never on `ryo-core`; neither capability crate imports the other in the proposed first split. Before any move, replace the core-local `InputSchema`, `ParameterBinding`, `OperatingError`, `PreparedInputs`, `PrimitiveLease`, `RowSink` and derived storage/scratch dependencies in their semantic interfaces with portable borrowed values, semantic errors and one output boundary. `ryo-core` keeps scheduling, admission, caches, leases and transport. Existing `ryo-language` owns provider facts and semantic contracts, `ryo-values` owns plan meaning, `ryo-query_language` owns DSL compilation, and SDK, providers and the shared benchmark keep their current outward dependency direction. Tastemaker should enforce the resulting dependency allowlist; this proposal adds no crates or trait scaffolding yet.

## 2. Rubric

Each row is **execution route × resource family/representation**. Enumerate actual registry, acquire/select, publication/Selected constructors, cache accesses and terminal drivers; map every resolved site exactly once. Unmapped sites block closure.

Tastemaker enforces imports, layout, required assets and structural access. Runtime tests establish behavior, completeness, lifetime and performance that structural rules cannot observe.

| Question | Ownership axes to prove | Required evidence |
| --- | --- | --- |
| Q1 Who may prepare, admit, build, publish and execute; in which phase? | Scheduling, sharing, invalidation, data, lifetime | Actual callers; Tastemaker structural access; runtime admission and failure controls. |
| Q2 At what frequency does work repeat: request, file/population, rule, node, row? | Scheduling, sharing | Cross-rule consumers, build counts and actual reached workload counts. |
| Q3 Which cardinalities, join order, bindings, indexes, scans, materializations and recursive deltas drive work? | Scheduling, data | Input/fanout distributions; preserved native/bulk paths; counted route costs. |
| Q4 Who owns required versus optional bytes, aliases, metadata and release? | Sharing, data, lifetime | Single backing charge; zero-budget sharing; last-consumer, refusal and error release. |
| Q5 Are diagnostics, coverage, ordered bags, partial absence, lazy errors and emitted prefixes preserved? | Scheduling, invalidation, lifetime | Competing-error, empty-input, partial/recovery and downstream-refusal controls. |
| Q6 What identifies and invalidates a population/view/result? | Sharing, invalidation | Generations, bindings, ordered projections/positions, same-buffer refill and dependency controls. |
| Q7 What is shared versus worker-local, and what workload/mode was measured? | Scheduling, data, lifetime | Determinism, aggregate peak, language/corpus/rules/threads and separate cold/warm/edit receipts. |

Cells are **proven: test/receipt**, **falsified: file:line**, or **unknown**; any falsified authority leaves its ownership checkbox unchecked. Traits/registry coverage alone prove neither sole ownership nor speed. Q2–Q4 workload-cost claims require attributable counted diagnostics or frozen-workload receipts; synthetic tests prove only their constructed mechanism. Shared untagged counters cannot identify a route. Exercise all 24 operators through actual DAG/root and residual/terminal routes; compile positives must use real capability types before negatives.

## 3. Prioritization

**Nightmare severity:** 5 = observed wrong results/OOM/unusable target; 4 = measured large time/memory waste; 3 = measured moderate repeated waste; 2 = localized small impact; 1 = minor; ? = unmeasured. Unchecked ownership does not imply severity 5.

| Current route / resource | Severity | Evidence and next fix |
| --- | --- | --- |
| Admission/publication, derived acquisition and terminal ownership | ?; correctness first | Known bypasses remain; predicate workspace application ownership is unchecked pending production-route enumeration. |
| Corrected owner checkpoint, frozen Django/Rust | 3 observed; cause unknown | [Matched receipts](../../languages/python/benchmarks/linters/ruff/results/owner-checkpoint-measurement-2026-11-03/README.md): Django cold +12.33%; Rust spike CPU +16.67% above guard. Graph/fanout ownership remains unchecked. |
| Graph/domain owner candidate, same frozen workload | 3 observed; cause unknown | [Matched receipts](../../languages/python/benchmarks/linters/ruff/results/common-owner-graph-2026-11-03/README.md): versus the corrected checkpoint, Django cold +0.39%, warm +13.08%, retained edit −11.22%; Rust spike CPU +1.87%. Exact reports, one pair, no general gain; fanout remains unchecked. |
| Prepared comparison candidate, frozen 310-rule workload | 3 observed; cause under study | [Corrected matched pair](../../languages/python/benchmarks/linters/ruff/results/prepared-comparisons-2026-11-03/README.md): versus 859c6af8, Django cold −0.40% (effectively flat), retained edit −11.9% in one observation; Rust warm +16.7%. The contaminated first baseline is excluded. [Source-bound CPU and bounded heap attribution](../../languages/python/benchmarks/linters/ruff/results/owner-cost-attribution-2026-11-03/README.md) ranks lifecycle, setup and backing cost; it is not a whole-corpus heap or exact CPU-caller account. |
| Repeated file/rule demand and materialization | ? | [Whole-corpus routes](../../languages/python/benchmarks/linters/ruff/results/owner-checkpoint-measurement-2026-11-03/routes-summary.json): rules/frontiers dominates instrumented phase time; PredicateMatches 1.172M, Equality 759k, DistinctRepresentatives 633k builds remain cost hypotheses. [Memo diagnostics](../../languages/python/benchmarks/linters/ruff/results/predicate-memo-diagnostic-2026-11-03/README.md) count 1.0015B text-equality bypasses; the options.py reset probe measured 0.329 ms, not whole-corpus reset cost. Compare reached scans/build identities next. |
| Narrow native/projection experiments | ? | Local options.py reached only 4 native segments / 11 projections; parked, no production or whole-workload gain credit. |
| Historical required-live recomputation | 5, guarded | [Recorded failure and repair](../../languages/python/benchmarks/linters/ruff/results/required-live-2026-11-02/README.md); preserve zero-optional-budget regression controls. |

Correctness/enforcement blockers come first. Otherwise sort **lexicographically** by observed severity, reached coverage, evidence confidence and dependencies; weigh effort and semantic risk. Confidence: frozen-workload receipt > attributable counted diagnostic > static hypothesis. Expected gain stays unknown without supporting evidence. Seek stepwise **20–50%+ whole-workload gains** by eliminating passes/repeated work; this is an aspiration, not a forecast or sum of microbenchmarks.

## 4. How to work

- [ ] Identify the semantic interface, engine-owned policy/state/lifetime, every implementation and actual route before changing code.
- [ ] Run a short isolated POC: hypothesis, smallest representative workload, baseline plus counts; document temporary contract debt, with no checked ownership credit.
- [ ] Consolidate into one production owner; remove replaced paths and preserve native access and lazy/error semantics. Use Tastemaker for structural boundaries and runtime tests for behavior and lifetime.
- [ ] Investigate first regressions using preserved baseline/failure receipts and route coverage; iterate the systemic migration instead of automatically abandoning it or raising caps.
- [ ] Invest in the shared benchmark harness: inventory, enumeration, execution, normalization, comparison, reporting and performance belong there; linters supply config/query/cases and necessary protocol adapters only, with no hardcoded rule exceptions.
- [ ] Repeat independent Fable review and fixes until blockers close; then run Rust/Python correctness and frozen cold/warm/edit performance gates before acceptance.

Reuse [measured demand/admission results](../../languages/python/benchmarks/linters/ruff/results/demand-admission-2026-11-03/README.md). The historical [18-workload/five-process spike](https://github.com/idiomatic-sh/idiomatic/blob/85d0a1ad7e/.agents/research/ryo-pvwd-performance-20261001.md) retained two failed timing budgets; [perf-4.2](https://github.com/idiomatic-sh/idiomatic/blob/85d0a1ad7e/crates/ryo/docs/archive/planning/perf-4.2.md) distinguishes measured M from assumed A. Its ≈515 ms is a model, not an achieved result. The [independent rubric review](evidence/fable-review-3-20261103.json) passed; implementation ownership closure remains open.

## 5. Inspiration

| Primary source | Transferable question; applicability limit |
| --- | --- |
| [CodeQL](https://codeql.github.com/docs/writing-codeql-queries/troubleshooting-query-performance/) | Binding, join cardinality and recursion; relational cost, not a diagnostic-equivalence oracle. |
| [Soufflé](https://souffle-lang.github.io/vldb19.html) | Automatic index selection; set/fixpoint semantics differ from ordered bags and partial facts. |
| [Ruff](https://github.com/astral-sh/ruff/blob/9d353a6f13a58ce0c7ae24a53859d6560a48e7cd/crates/ruff_linter/src/checkers/ast/mod.rs#L1-L23) | Shared semantic traversal with deferred phases; individual checks can still walk. |
| [Oxc/Oxlint](https://github.com/oxc-project/oxc/blob/406bc50d1d714cfd905f94e58d37fee4757dcfea/crates/oxc_linter/src/lib.rs#L161-L270) | Node-kind dispatch and reused rule buckets; compare optimized/reference diagnostics. |
| [TSRS](https://github.com/oxc-project/tsrs/blob/5cdb8d89204d9ef8bb4856e1f3b9fa030f224892/docs/PORTING.md#L111-L155) | Frozen shared data versus checker-local state; fast-mode semantic differences are not work elimination. |
| [Biome](https://github.com/biomejs/biome/blob/b620cbceed195542cb3c051f265e1f9e043a3749/crates/biome_analyze/src/registry.rs#L20-L155) | Service/phase eligibility and query dispatch; no universal single-pass requirement. |
| [Clippy](https://doc.rust-lang.org/stable/clippy/development/lint_passes.html) / [rustc queries](https://rustc-dev-guide.rust-lang.org/query.html) | Phase-correct compiler queries and tracked memoized dependencies; compiler work is reused, not standalone lint cost. |
| [Go analysis](https://pkg.go.dev/golang.org/x/tools/go/analysis) | Requires/ResultOf/Facts and driver-controlled reads; package scope differs from file rows. |
| [Credo](https://hexdocs.pm/credo/1.7.19/Credo.Check.html) / [Dialyzer](https://www.erlang.org/doc/apps/dialyzer/dialyzer_chapter.html) | Per-source checks versus persistent analysis summaries; distinct tools and contracts. |
