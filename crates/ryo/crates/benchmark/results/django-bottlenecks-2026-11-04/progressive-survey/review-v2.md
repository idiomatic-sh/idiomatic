# Gut-check review: ordered-existence / anti-selfjoin POC (read-only)

Reviewer: Claude Fable 5.1, high effort. 2026-11-04. No source edited. Nothing in this file is a benchmark claim.

## Verdict

**PASS for a bounded POC, with the mechanism re-stated and four required changes.** Not a BLOCK: the causal evidence for ISC004 is sound and the shape is genuinely generic (counted below, not assumed). Not a clean PASS: the plan as written over-scopes the POC (owners, leases, retention, sharing) for a rewrite that needs none of it, and the semantic proof as stated targets the wrong boundary (it keeps the antijoin's membership semantics implicit instead of leaving the antijoin untouched).

## Context

The 409-rule Django survey on db1 completes 408 rules; ISC004 is censored beyond 360 s and a 720 s retry is live. The 90 s instrumented ISC004 capture stops inside tests/admin_views/tests.py at rule-step 35, a self-join of step 34 on exact keys (id, file), with 99,202 rows on each side. The owner wants a gut check on the proposed generic rewrite before implementation starts.

## 1. Causal evidence: confirmed, and the arithmetic is stronger than the plan states

From /tmp/django-ISC004-operations-20261104/events.jsonl, node 35 on the hot file (46 progress snapshots, 1.0 s to 60.0 s elapsed within that file scope):

| quantity | value |
|---|---|
| probes done | 620 of 99,202 |
| matches emitted so far | 30,666,916 |
| rows per probe (mean bucket) | 49,516 |
| implied partitions in `preceding` for this file | 99,202 / 49,516 ≈ 2 |
| implied total pairs for the file | 99,202 × 49,516 ≈ 4.9 × 10⁹ examined, ≈ 2.45 × 10⁹ pass `l.token_end < r.token_end` |
| observed emission rate (instrumented) | ≈ 516 k pairs/s |

Two concatenated strings inside a collection in that file, each preceded by ≈49.5 k tokens, is enough to produce ≈5 billion pair visits. At the instrumented rate that one file alone is ≈2.6 hours. Even if the un-instrumented kernel is 20× faster, the file is still ≈8 minutes, consistent with the 360 s censor. The query is `exists r in partition(id,file): r.token_end > l.token_end`, i.e. "is this the last preceding token", and the engine answers it by enumerating every pair. The bottleneck is algorithmic shape, not a missing index: the index is used (index_probes = input_rows = 620), and the chain walk is the cost. Confirmed.

Caveat the plan must keep: the 516 k pairs/s is an instrumented rate. It is also suspiciously slow for an indexed equality join (a tight bucket walk is tens of millions of pairs/s). Whether the production kernel has a large constant-factor hole on top of the shape problem is **unknown** and should be measured once, cheaply (below), because if it is ≈1 M pairs/s in production, that constant factor touches every join in 300+ rules and could compete for first bet.

## 2. Is the shape generic? Yes, and the frequency is countable, not UNKNOWN

Syntactic survey of the frozen pack /tmp/ryo-current409-pack-v3-20261103/profile/rules (regex over `let X = select … from A as l join A as r on … where …;` followed by `antijoin(left=A, right=X, …)`):

| feature | count |
|---|---|
| antijoin(left=A, right=selfjoin(A)) sites | 214 in 85 rules |
| predicate is a single strict comparison `l.c <op> r.c` on the same column | 202 |
| compound lexicographic predicate (`l.finish > r.finish or l.finish = r.finish and l.begin < r.begin`) | 12 (all the `supp_later_regions` suppression helper) |
| projection uses `r.*` columns | 9 (PLE1507, PLW1501, PLW1508, PYI019, PYI041, UP008) |
| intermediate has more than one consumer | 4 (PYI012, PYI013, PYI048, UP014) |
| self-join partition keys NOT a subset of the antijoin keys | 26 (e.g. B021 `later` partitions by parent, antijoin on child) |
| compares different columns (`l.c` vs `r.d`) | 0 |

So "do not special-case ISC004" is justified. But two facts change the expected benefit:

- **Partition size, not site count, decides the gain.** `positional_ordered` / `ordered_keyword_names` (dozens of rules) partition by call: buckets of 2–10 rows. Quadratic over 5 rows is nothing. The gain exists only where a partition is large: tokens per file (ISC004), and possibly token-per-decorator sites. The recognizer firing 200 times is irrelevant; eliminated pairs per site is the number.
- **The current leaders are mostly not this shape.** Top completed isolated walls from the all413 receipts: F841 21.0, F811 19.5, PT026 18.4, PT025 17.2, PT020 16.2, PLC0206 14.8, PIE790 14.0. F841, F811, PIE790 have no self-join at all. PT020/PT025/PT026 do have the shape (`later_expression_tokens` over `expression_tokens` partitioned by decorator), but their partitions are tokens within one decorator (small) and their peak RSS is 1.6 GB versus 140 MB for ISC004. Their cost signature points at the preceding steps `at_in_decorators` and `expression_tokens`: joins whose only equality key is `file` with a range predicate, i.e. decorators_in_file × tokens_in_file materialised. That is a different shape (partitioned range join), and the pack has 221 such file-only-key range-join sites in 56 rules. ISC004's own `preceding` step is one of them (it is what produces the 99,202 rows). The plan already labels PT020/PT026 a hypothesis; this review's reading is that the existence rewrite will **not** move them.

Net: expected gain is "ISC004 goes from censored to completed, and a currently unmeasurable number of small wins elsewhere". The plan's refusal to promise 20–50% is correct. The honest framing is: this POC fixes the one censored rule and is the first consumer of a primitive (per-partition ordered view) that the range-join shape would also need. Say that, and do not quote the 814 s "wall sum of self-join rules" that a naive attribution would produce; it is not attributable.

## 3. Mechanism: re-state it as "replace the self-join subtree, leave the antijoin alone"

The plan says: replace the antijoin's right input with a "partitioned ordered extremum/existence view", and prove that "projected membership keys are sufficient" while preserving ties, multiplicity and bag order. That phrasing invites a per-row existence test, which is **wrong under the antijoin's own semantics**, and it drags the antijoin into the proof obligation unnecessarily.

The right decomposition. The subtree feeding the antijoin's right side is, in physical terms (from the trace: step 35 Join with `filter: []`, step 36 predicate step, step 37 projection):

```
Project_{l.k1..l.kn}( Select_{l.c < r.c}( Join_{p}(A, A) ) )
```

The antijoin consumes it as a **set of key tuples** (right multiplicity is irrelevant to an antijoin). Therefore the only equivalence that has to hold is set equality:

```
π_K( σ_{l.c < r.c}( A ⋈_p A ) )  ≡  π_K( σ_{x.c < max_p(c)}( A ) )      as SETS
```

where `max_p(c)` is the maximum of `c` over x's own partition under the self-join keys `p`. This holds because for any row x, `∃ r ∈ partition_p(x): x.c < r.c ⟺ x.c < max_p(c)` for a total order; the projection keeps only left columns, so the right side contributes nothing but existence. The rewritten plan is:

```
A  →  [per-partition extremum of c over keys p; flag rows with c < max]  →  Project_K  →  (right of unchanged antijoin)
```

Why this is the correct boundary:

- **The antijoin stays byte-for-byte the same operator**, so left-side bag order, multiplicity, duplicate-key groups, empty sides and absent partitions are preserved by construction, not by proof. The plan's hardest falsifiers mostly evaporate.
- **The duplicate-key trap is handled for free.** Original semantics: if two left rows share the antijoin key (id, token, file) but have different `token_end` (duplicate source-location rows would do it), and only one of them is dominated, the antijoin removes **both**, because membership is by key value. A per-row "is this row dominated, keep it otherwise" view would keep one. Under the re-stated mechanism the dominated row still produces its key tuple, the antijoin still removes the whole key group. The plan's falsifier list says "duplicate keys/duplicate projected rows"; it must spell out this exact case with this exact expected output, because it is the case where the two formulations disagree.
- **The 26 sites whose partition keys are not a subset of the antijoin keys** (B021, D419, PLR1711 partition by parent, antijoin on child, etc.) need no special treatment: dominated-ness is computed per row in its partition, keys are projected, the antijoin does the rest. Under the plan's "ordered-existence view replaces the right side" framing those 26 need a separate proof or a decline.
- **Ties.** Strict `<` against the partition max: every maximiser (tied or not) has `c == max`, is not flagged, is kept. `>` uses min. Non-strict comparators (none in the pack today) would flag every row in a partition with ≥2 rows including itself (self-pair satisfies `<=`); decline them in the POC rather than reason about it.
- **Self-pairs.** A self-join always pairs each row with itself. For strict comparison on the same column the self-pair never passes, so the extremum formulation is exact. This is also why the lazy-error argument below works.

What the rewrite is NOT: it is not a replacement for Rank (text-set semantics) nor OrderedMatch (consuming earliest-later). It is a stateless, streaming, per-partition extremum plus a comparison. It does not need a retained representation, a lease, an owner, sharing or cancellation plumbing beyond what any single step already has. See §5.

## 4. Semantic traps the plan must add or sharpen

1. **Key-group poisoning (above).** Falsifier: A = {(k, c=5), (k, c=7), (k', c=6)} in one partition, antijoin key = k. Expected: both k rows removed (k' is max). A per-row view keeps (k, 7). This single fixture separates the correct formulation from the tempting one.
2. **Lazy error / fault timing.** The plan demands "original lazy error/refusal timing" be preserved. Under the original plan every row's `c` is compared at least once (self-pair), so any malformed cell faults in both plans; but **which** fault surfaces first can differ (original walks pairs in index-chain order, the rewrite walks rows once). "First-fault latch" identity is therefore not achievable in general. Resolve it the way the plan already hints: the POC is eligible only when the operand column is a statically-typed total order (integers from source-location columns) so no fault is possible; anything else declines at planning time with no partial output. Then "same fault timing" is vacuous and should be stated as such rather than claimed.
3. **Missing cells / nulls.** Same resolution: decline unless the column is proven non-null at plan time. Do not try to reproduce three-valued comparison.
4. **Compound predicate (12 sites).** `l.finish > r.finish or l.finish = r.finish and l.begin < r.begin` is a lexicographic max over (finish, −begin). It is expressible as the same primitive with a tuple comparator, but the POC should **decline** it initially and measure whether those sites ever have large partitions (they are suppression regions per directive; probably small).
5. **`r.*` in projection (9 sites) and multi-consumer intermediates (4 sites): decline.** The plan says so; keep it.
6. **Predicate step must be the whole filter.** The recogniser must see Join(S,S) → Select whose only predicate is `Left.c <op> Right.c` with `kept = []` extras → Project of left columns only → sole consumer is an antijoin right input. Any additional predicate, any kept bindings, any other consumer: decline. (Note from the trace that the inequality is NOT pushed into the Join: `filter: []`. The recogniser must match the three-step physical shape, not a fused one.)
7. **Equality on the partition key must stay exact.** Hash-only grouping is not acceptable; use the same equality the join uses (the plan's "preserve equality collision checks").
8. **Shared step identity.** Match left and right as the **same step id**, not structurally equal plans. The trace shows `Join { left: Step(34), right: Step(34) }`; structural equality of two different steps that happen to be equivalent is a separate optimisation and must not be assumed.

## 5. Systemic enforcement and architecture: the POC is over-scoped

The plan's second and third paragraphs (mandatory owner/interface for every route/kind, required leases, indexes, sharing, cancellation, publication, "no blanket cap raise/new shadow cache", "optional retention is a policy outcome") describe the secondary holes 2 and 3 (retention/ledger ownership). None of it is needed for this rewrite:

- The extremum view is consumed exactly once, streams, and holds one `(key → extremum)` map per partition set, sized like one group-by over A. It is not a retained representation and needs no owner, lease or reclaim policy. If the design insists on making it a shared retained kind, that is the ledger-ownership refactor (hole 3) smuggled into the POC, and it should be refused for the same reason the plan refuses to implement holes 1–3 first.
- Placement that matches the stated layering with the least surface: the recogniser is a pure function in domain query planning (Join/Select/Project/AntiJoin consumer-count → `Option<PartitionExtremumPlan>`); one new pure kernel in the operators crate (partition extremum + flag), behind the existing injected port; the application picks the strategy and records the decision. No new engine owner, no env switch (the plan's own NodeResultReuse correction shows env reads are being removed, keep it that way: a typed option or nothing).
- Enforcement: the recogniser's decline list is a table; a registry-driven strategy test that enumerates the decline categories above and asserts "ordinary route" for each is enough. The "Compiler/private kernel/TM enforce sole boundary" line is fine but must not become a new rule family before the POC proves worth.

Against the memories: "briefs forbid additions", "useful, not tooling", "fix perf in the owning crate". The POC as re-stated adds one planner recognition, one kernel, one strategy table, and fixtures. Nothing else.

## 6. Acceptance plan: the oracle for ISC004 does not exist yet

- **ISC004 has never completed on the full corpus**, so "default-vs-POC exact whole report" has no default side. Options, in order: (a) the live 720 s retry (PID 2288421, ≈10.5 CPU-minutes in when checked) or its doubling successors complete and give the default report; (b) if not, the default report on the corpus minus the hot files that exceed a per-file budget, plus the hot files judged by ruff's own ISC004 output (the bake-off oracle); (c) never certify ISC004 by POC-vs-POC agreement.
- **The real semantic oracle is the other 84 rules.** All complete under default. Run default vs POC exact-bag equality (diagnostic bag, skip bag, invalid-syntax bag) over all 85 shape-bearing rules on the pinned corpus, same worker, same pack. If the recogniser fires on ~190 sites and all 84 completed rules are bag-identical, that is the proof the rewrite is sound in production; ISC004 then only needs to complete.
- **"Eliminated pairs" is measurable today without building anything.** At each recognised site the existing `equality_matches` counter at the self-join node is exactly the pair count the rewrite removes, and `input_rows` is what it costs instead. Run the instrumented capture for the handful of rules with large partitions (ISC004, PT020/PT025/PT026, PLC0206, PLW0406) and read those two counters. This ranks sites by `Σ n_p²` and tells you before coding whether anything beyond ISC004 gains. Keep the memory: instrumented wall is not gain; counters are.
- **One constant-factor measurement**, cheap: time the un-instrumented production join on a controlled self-join workload with a known pair count (e.g. ISC004 restricted to admin_views/tests.py, which has ≈4.9 × 10⁹ pairs) and divide. If production is < 5 M pairs/s, open a separate bead for the kernel's per-pair cost; do not fold it into this POC.
- Do not pool anything from the E902/RUF200 source (7a097013) with db1 numbers, and do not sum the cohort walls. The plan says this; this review repeats it because the "wall sum of self-join rules" temptation is real (it comes out to 815 of 1921 s and means nothing).

## 7. Required changes before implementation starts

1. Re-state the mechanism as in §3: replace only `Project(Select(Join(S,S)))` with `Project(Flag(Extremum_p(S)))`; the antijoin is untouched. Rewrite the falsifier list around set equality of the right side, and add the key-group poisoning fixture verbatim.
2. Eligibility for the POC: single strict comparison, same column both sides, statically typed non-null total order, projection of left columns only, single consumer which is an antijoin right input, left/right the same step id. Everything else declines at plan time. No runtime fallback exists because no runtime failure mode exists.
3. Strip owners/leases/retention/sharing/publication from the POC scope. The extremum map is per-step scratch. Record in the bead that holes 2 and 3 are explicitly not touched.
4. Before writing kernel code, run the counter survey in §6 (equality_matches and input_rows at recognised self-join nodes for the 6 named rules) and the one un-instrumented pairs/s measurement. Both are hours, not days, and the second could change the first bet.

## 8. Remaining uncertainty

- Production (un-instrumented) pairs/s of the equality join. Unknown; bounded only by "ISC004 did not finish in 360 s".
- Whether any site other than ISC004 has partitions large enough to matter. Unknown until the counter survey; the leaders' profiles suggest no.
- Whether the 720 s retry completes. If it does, its report becomes the ISC004 oracle and its wall becomes the only honest "before" number.
- PT020/PT025/PT026/PLW0406 cost attribution (1.6–2.1 GB peaks). Hypothesis: file-only-key range joins. Not confirmed by any node-level capture yet.
- The secondary holes remain measured and unaddressed: hole 1 (DISTINCT-then-Select commute with original dedup columns) is a real correctness subtlety and the plan's description of the wrong fix is accurate; hole 2's BudgetRefused is attributable to the ledger being occupied by leases outside State.entries, so eviction is not the fix; hole 3 is the ownership refactor. None should start before this POC's counter survey decides the first bet.

## Source map (read-only exploration of /tmp/ryo-derived-capability-20261103, paths under crates/ryo/crates/)

What the trace shape is, in code:

- `select … from A as l join A as r on … where l.c < r.c` lowers to three steps: `Join { filter: [], view: [], reach: None }` from `query_language/src/domain/joining/service.rs:102-112` (the unselected `token_end` columns are appended to the join output, `:151-212`, which is why the trace shows `Left 7` and `Right 7`); then `Select { comparisons, kept: [] }` from `query_language/src/domain/selecting/service.rs:171-207` (step 36, evaluated per row by `lease.holds` in `core/src/domain/operator/collections/operators/select/item.rs:41`); then `Project { columns: 0..3, distinct: Dropped }` from `selecting/service.rs:228-248` (step 37). There is no DISTINCT keyword; a plain `select` deduplicates whenever no output column is `Unique` (`resolving/service.rs:55-61`), and join outputs are all `Shared`. So `earlier` is already a per-invocation set. **The plan's "projected DISTINCT selfjoin" is accurate, and the rewrite can keep step 37 verbatim.**
- `Join.filter` can only hold right-side predicates (`values/src/domain/plans/types.rs:123`, bound over input 1 at `core/src/domain/operator/contract/service.rs:275`), and the only Select-into-Join fusion requires a tree relation on the right (`fusing/service.rs:199-270`). A cross-side inequality can never be fused into the Join today. The recogniser must match the three-step shape; it cannot assume a fused one.
- Hash index: `derived/src/domain/equality.rs:6-53` builds `next_power_of_two(rows)` buckets with chains in ascending row order; `probe`/`next_match` in `core/src/domain/operator/contract/views.rs:39-81` walk the chain comparing stored hash then every key cell with `Value ==`, counting `IndexRowsExamined` and `EqualityMatches`. Nothing orders a chain by any value column, so the self-join is Σ group² by construction. The `KeyIndex.sorted` field (`derived/src/domain/contract/types.rs:183-191`) is used only by OrderedDfs.
- Antijoin (`operators/src/domain/set_rows/service.rs:24-45`, resource `Index { input: 1, columns: right_on }` at `core/src/domain/operator/contract/resources.rs:100-103`): keeps a left row iff its projected key is not in the right key set; left duplicates kept (no `first_time`); right side fully held and indexed before probing (`LeftThenRightUnlessEmptyLeft`, `contract/service.rs:87-93`). **Right multiplicity is irrelevant, which is exactly what §3's set-equality proof needs.**
- Rank (`core/src/domain/operator/collections/operators/rank/item.rs:28-70`) appends the index of the first parameter text set containing the column text and drops non-matches. OrderedMatch (`ordered_match/item.rs:55-91`, build `derived/src/domain/ordered.rs:20-92`) is a greedy FIFO one-to-one pairing of each ordered left row with the earliest later right row in the same key group, consuming it, and errors on duplicate (key, order). **Neither is "exists a later row in the partition"; the plan is right that they are not replacements by name.** OrderedMatch's `DuplicateOrder` error alone rules it out for ISC004, where equal `token_end` values are legal.
- Existing recognisers to pattern after, not to extend: `core/src/domain/column_kernel/service.rs:52-140` already matches `Project(Dropped) ← Select(symbol = literal) ← Join(Step, Relation)` in the core domain and lets the application pick the kernel. That is the precedent for where this recogniser belongs: core domain, over the physical plan, no new `Operation` variant and no RyoQL change. `recognize_same_sets` (`query_language/src/domain/idioms/service.rs:263-317`) is the only place that checks `left == right` today and fires only inside the text-set antijoin idiom. Consumer counting exists: `reads_of`/`layout_of` in `core/src/domain/layout/service.rs:22-77` (a self-join counts as two reads; `preceding` has three and is kept in a table), `readers == 1` in `pushing/service.rs:413-423`.
- Kernel home: the portable equality join (`operators/src/domain/equality_join/service.rs:7-53`) is pure and driven through `JoinDriver` (`operators/src/domain/contract/types.rs:170-188`) with candidates "in original bag order". A partition-extremum kernel belongs beside it, with the engine supplying a borrowed driver over the partitioned rows, which is the shape the plan asks for.
- A lease (`core/src/application/file_dag/execution/lease.rs:24-214`) borrows an invocation's acquired views, scratch, symbols and predicate memo for one operator call and is dropped after. Plain Joins bypass it (`:107-149`). Nothing about the rewrite needs a new lease kind.
- Secondary holes located as described: the DISTINCT guard is `.filter(|selector| !selector.distinct)` at `core/src/domain/stored_selection/types/program/value.rs:49`; `reclaim_optional_preserving` has one caller at `core/src/application/file_dag/derived/owner/graph_policy.rs:37`.

Latent hazard noticed in passing, outside this POC: `join/item/ordinary.rs` applies `Join.filter` only on the native tree branch (`:62-73`); the hash-index branch (`:80-90`) never consults it. The lowering never produces a filtered hash join today, but nothing was found that enforces it. Worth one assertion or lint, in its own bead.
