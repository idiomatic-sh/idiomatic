# Engine contracts: working rubric

## 1. Rubric

Each row is **execution route × resource family/representation**. Enumerate actual registry, acquire/select, publication/Selected constructors, cache accesses and terminal drivers; map every resolved site exactly once. Unmapped sites block closure.

| Question | Ownership axes to prove | Required evidence |
| --- | --- | --- |
| Q1 Who may prepare, admit, build, publish and execute; in which phase? | Scheduling, sharing, invalidation, data, lifetime | Actual callers; private capabilities; application and domain bypass negatives. |
| Q2 At what frequency does work repeat: request, file/population, rule, node, row? | Scheduling, sharing | Cross-rule consumers, build counts and actual reached workload counts. |
| Q3 Which cardinalities, join order, bindings, indexes, scans, materializations and recursive deltas drive work? | Scheduling, data | Input/fanout distributions; preserved native/bulk paths; counted route costs. |
| Q4 Who owns required versus optional bytes, aliases, metadata and release? | Sharing, data, lifetime | Single backing charge; zero-budget sharing; last-consumer, refusal and error release. |
| Q5 Are diagnostics, coverage, ordered bags, partial absence, lazy errors and emitted prefixes preserved? | Scheduling, invalidation, lifetime | Competing-error, empty-input, partial/recovery and downstream-refusal controls. |
| Q6 What identifies and invalidates a population/view/result? | Sharing, invalidation | Generations, bindings, ordered projections/positions, same-buffer refill and dependency controls. |
| Q7 What is shared versus worker-local, and what workload/mode was measured? | Scheduling, data, lifetime | Determinism, aggregate peak, language/corpus/rules/threads and separate cold/warm/edit receipts. |

Cells are **proven: test/receipt**, **falsified: file:line**, or **unknown**; any falsified authority leaves its ownership checkbox unchecked. Traits/registry coverage alone prove neither sole ownership nor speed. Q2–Q4 workload-cost claims require attributable counted diagnostics or frozen-workload receipts; synthetic tests prove only their constructed mechanism. Shared untagged counters cannot identify a route. Exercise all 24 operators through actual DAG/root and residual/terminal routes; compile positives must use real capability types before negatives.

## 2. Prioritization

**Nightmare severity:** 5 = observed wrong results/OOM/unusable target; 4 = measured large time/memory waste; 3 = measured moderate repeated waste; 2 = localized small impact; 1 = minor; ? = unmeasured. Unchecked ownership does not imply severity 5.

| Current route / resource | Severity | Evidence and next fix |
| --- | --- | --- |
| Admission/publication, derived acquisition and terminal ownership | ?; correctness first | Known bypasses remain; inventory every producer/consumer, remove mutable-cache escape and competing owners. |
| Repeated file/rule demand and materialization | ? | [One-file route counts](../../languages/python/benchmarks/linters/ruff/results/resource-route-diagnostic-2026-11-03/README.md): 47,620 graph requests, 24,555 traversals, 2,088 builds; consumer identity and cost remain mostly unknown. |
| Narrow native/projection experiments | ? | Local options.py reached only 4 native segments / 11 projections; parked, no production or whole-workload gain credit. |
| Historical required-live recomputation | 5, guarded | [Recorded failure and repair](../../languages/python/benchmarks/linters/ruff/results/required-live-2026-11-02/README.md); preserve zero-optional-budget regression controls. |

Correctness/enforcement blockers come first. Otherwise sort **lexicographically** by observed severity, reached coverage, evidence confidence and dependencies; weigh effort and semantic risk. Confidence: frozen-workload receipt > attributable counted diagnostic > static hypothesis. Expected gain stays unknown without supporting evidence. Seek stepwise **20–50%+ whole-workload gains** by eliminating passes/repeated work; this is an aspiration, not a forecast or sum of microbenchmarks.

## 3. How to work

- [ ] Identify the semantic interface, engine-owned policy/state/lifetime, every implementation and actual route before changing code.
- [ ] Run a short isolated POC: hypothesis, smallest representative workload, baseline plus counts; document temporary contract debt, with no checked ownership credit.
- [ ] Consolidate into one production owner; remove replaced paths, preserve native access and lazy/error semantics, and add falsifying capability/behavior tests.
- [ ] Investigate first regressions using preserved baseline/failure receipts and route coverage; iterate the systemic migration instead of automatically abandoning it or raising caps.
- [ ] Invest in the shared benchmark harness: inventory, enumeration, execution, normalization, comparison, reporting and performance belong there; linters supply config/query/cases and necessary protocol adapters only, with no hardcoded rule exceptions.
- [ ] Repeat independent Fable review and fixes until blockers close; then run Rust/Python correctness and frozen cold/warm/edit performance gates before acceptance.

Reuse [measured demand/admission results](../../languages/python/benchmarks/linters/ruff/results/demand-admission-2026-11-03/README.md). The historical [18-workload/five-process spike](https://github.com/idiomatic-sh/idiomatic/blob/85d0a1ad7e/.agents/research/ryo-pvwd-performance-20261001.md) retained two failed timing budgets; [perf-4.2](https://github.com/idiomatic-sh/idiomatic/blob/85d0a1ad7e/crates/ryo/docs/archive/planning/perf-4.2.md) distinguishes measured M from assumed A. Its ≈515 ms is a model, not an achieved result. The [independent rubric review](evidence/fable-review-3-20261103.json) passed; implementation ownership closure remains open.

## 4. Inspiration

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
