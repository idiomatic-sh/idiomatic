# Extension ownership contracts

## 1. Ownership matrix

`☑` means the engine is the sole authority for that cell on the named **actual route**, with source and test evidence. `☐` means split or unproved ownership; `—` means the axis does not apply. A checked cell is not a speed claim. The [common-owner tests and review](../../languages/python/benchmarks/linters/ruff/results/common-owner-graph-2026-11-03/README.md) cover the scoped selection, derived-view, and graph rows; fanout and remaining terminal paths stay open.

| Contract / route | Scheduling | Caching | Invalidation | Data ownership | Lifetime | Remaining implementor responsibility |
| --- | :---: | :---: | :---: | :---: | :---: | --- |
| Language parse and project | ☑ | — | ☑ | ☐ | ☐ | Provider parser state and scratch stay local. |
| Typed relation catalog | ☑ | — | — | ☑ | ☑ | Provider supplies values through typed relations. |
| Complete and partial facts | ☑ | ☑ | ☑ | ☑ | ☑ | Engine owns typed storage; provider still certifies recovery and partials. |
| Provider walk versus execution demand | ☑ | — | — | ☑ | — | Engine owns demand rows; provider still certifies deferred-fact completeness. |
| Source bytes and spans | ☑ | — | ☑ | ☑ | ☑ | Engine owns stored spans; provider still supplies token coordinates. |
| Source and derived symbols | — | — | ☑ | ☑ | ☑ | Engine owns symbol storage; provider still supplies source-name meaning. |
| Query grammar and typed plan | ☑ | — | ☑ | ☑ | ☑ | Map compile errors without changing query semantics. |
| Primitive schema and evaluator | ☐ | ☐ | ☑ | ☑ | ☐ | Close remaining native and fanout bypass routes. |
| Operator inputs and absence | ☑ | — | — | ☑ | — | Preserve declared absence and lazy error order. |
| Ordinary plan, frontier, and terminal output | ☐ | ☐ | ☐ | ☑ | ☐ | Finish one scheduler and output lifetime across residual paths. |
| Request-wide file graph | ☐ | ☐ | ☐ | ☑ | ☐ | Consolidate remaining request and Stepper work/release. |
| Stored selections and selector views | ☑ | ☑ | ☑ | ☑ | ☑ | Extend the proven owner path to CandidatePostings fanout. |
| Derived predicate/equality/distinct views | ☐ | ☑ | ☑ | ☑ | ☑ | Remove remaining caller-local scheduling choices. |
| Graph adjacency, domain, and memo retention | ☐ | ☑ | ☑ | ☑ | ☑ | Finish traversal-demand scheduling; measure cost separately. |
| Keyed semantic rendering demand | ☐ | ☐ | ☐ | ☐ | ☐ | Add engine-owned keyed requests, per-rule incompleteness and lifetime; providers supply syntax rendering only. |
| CandidatePostings fanout | ☐ | ☐ | ☐ | ☐ | ☐ | Move admission, materialization, reuse, and release into the common owner. |
| Persistent lint snapshot | ☑ | ☑ | ☑ | ☐ | ☐ | Adapter still owns framing and file handles. |
| Outbound reports and jobs | ☑ | — | ☑ | ☑ | ☐ | Scheduler adapter still owns executor resources. |

### Proposed remaining boundaries

These signatures describe the authority to consolidate; they are **proposed**, not implemented or ownership proof. Domain types state meaning. Application operations own scheduling, admission, cache identity and last release. Outbound ports supply external facts and effects. Engine handles, budgets, indexes and scheduler choices stay internal and never become language provider contracts.

| Family | Semantic contract | Proposed application operation and open route |
| --- | --- | --- |
| Terminal and frontier rows | Existing `Input` names a relation or step; output remains an ordered bag with explicit absence, partiality and error order. | `publish(input) -> PublishedInput`, then `resolve(&publication) -> Rows<'owner>` and `scan(rows, sink)`; the application retains the publication through its last consumer. Publication may defer work: preserve lazy errors, streaming cancellation and emitted prefixes. Reconcile [`run`, `deliver`, `keep` and `hand_over`](../../crates/core/src/application/file_dag/pipeline/service.rs), including selected, residual, exported terminal and cross-file frontier paths. Delivery does not create a new ownership handle after rows have escaped. |
| Native evaluation | Typed tree rows, rule demand and completeness have the same meaning as ordinary operator input; there is one evaluator semantic contract. | The internal `evaluate_native(demand, tree, sink)` strategy publishes through that same result path. The [shared walk and residual mask](../../crates/core/src/application/file_analysis/service.rs) remain application scheduling choices, not a second public evaluator API; the provider supplies the typed tree and partial-fact proof. |
| Derived views and traversal | Existing `ViewRequest`, graph demand and admitted input proof define the result; aliases retain one physical backing. | `acquire(request, proof) -> Result<Option<DerivedHandle>>` and `reach(graph, demand) -> Result<RootAnswer>` are application-owned admission and lifetime operations. The graph, index and memo storage layouts stay private. |
| Candidate fanout | A candidate request specifies relation, column, literal set and ordered row positions. Optional decline means ordinary selection can run; unknown facts and execution errors remain distinct. | `acquire_candidates(request, proof) -> Result<CandidateDecision>` returns a leased answer or explicit decline. Move [local fanout choice, retained capacity and reuse](../../crates/core/src/application/file_dag/derived/selection/fanout.rs) under the common owner without changing bag order. |
| Keyed semantic answers | Existing `RenderRequest`, `RenderOutcome` and `RenderError` define syntax roles, Known, NotApplicable and Unsupported. | `render_batch(file_generation, consuming_rule, requests) -> Result<Vec<RenderOutcome>>` admits and shares answers by generation/context. Unsupported marks that consuming rule incomplete before publication and prevents a complete persisted snapshot. The outbound `SemanticSession` renders syntax only; current batch-local dedup does not close answer lifetime. |
| External resources | Corpus, facts, language, store, rows and scheduler remain separate outbound ports with typed results. | The application opens and ends each file/request scope and retains only declared leases. Persisted completeness must remain truthful, including supported partial facts; Unknown never becomes a Known empty result. Adapters own external handles; the domain sees no file descriptor, worker, cache or transaction type. |

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
