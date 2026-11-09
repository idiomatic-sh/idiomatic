# Engine consolidation review

Reviewer brief for the integration branch `claude/ryo-consolidation`, which
carries the engine consolidation toward the phases in
[the matrix](../README.md). One phase is active at a time; integration is never
red except for the known-red list below.

## Branch

Checkpoints 310-327 are evidence commits on the branch; each names the batches
it merged and the validation it ran.

| Checkpoint | Sha | Landed |
|---|---|---|
| 310 | `18fe9b6916` | sparse-activation, route-coverage and tm-fixture batches |
| 311 | `d92fce4796` | Phase 2 scheduling (structure, submission, lifecycle) |
| 312 | `51aeb2cf4b` | Phase 3 shared-walk batches |
| 313 | `4966f7e3f9` | Phase 2 review repairs, runtime structure |
| 314 | `cd2b23f100` | Phase 4 derived ownership, Phase 3 repairs |
| 315 | `7e93be040a` | Phase 2/3 repairs, rule re-version, Phase 5 fusion and route |
| 316 | `1e3eba5c90` | Phase 6 reuse and invalidation, Phase 4 census and bypasses |
| 317 | `d3f16fcd11` | Phase 7 TM batches (whole-tree TM 8,060 to 4,251) |
| 318 | `80f473c67d` | Phase 1 conformance and syntax poisoning, Phase 2 to 6 review repairs III, blow-up replay repairs, Phase 7 operators/core TM (validated source `c043a6dc68`; whole-tree TM 3,676) |
| 319 | `f2d8aaf1db` | the frozen 318 branches (whole-tree TM 3,676 to 2,762) |
| 320 | `61cb444739` | wave 2 (whole-tree TM 2,762 to 1,223) |
| 321 | `0015475b6e` | wave 3 and the benchmark branch (whole-tree TM 1,223 to 34) |
| 322 | `d13686afb5` | wave 4 (whole-tree TM 34 to 24) |
| 323 | `7d93840b19` | perf iteration 1, enforcement rules, ratchet 321, live hooks, review repairs VII |
| 324 | `14a8352416` | landing 1 (execution_domain and execution_store crates), review repairs VIII, harness 324 |
| 325 | `dcd94840fc` | review repairs IX, Django unknowns second pass, ratchet 323 |
| 326 | `f3a9a7e95c` | review repairs X, perf iteration 2 |
| 327 | `e49dc1dfc5` | review repairs XI, perf iteration 3 (validated source `142fe40a98`) |

## How work lands

Builders write uncompiled batches on side branches. The integrator merges them,
compiles once, runs the committed validation set, and lands only green. The
checkpoint commit message carries the numbers. Review the exact merged source,
never a branch tip.

## Evidence

[`evidence/checkpoint/validation.json`](../evidence/checkpoint/validation.json)
and `identity.json` (source revision plus the command list) are committed; the
set is described in the [checkpoint README](../evidence/checkpoint/README.md).
Raw logs live under `/tmp` per run and are never committed.

## How reviews run

The [contract-review agent](../../../../../.agents/agents/contract-review.md)
runs each review. Each phase writes its verdict as the
[review evidence schema](#review-evidence-schema) file under
`audits/2026-11-06-phase<N>/`. A BLOCKING verdict gates the matrix tick for its
rows, not the merge. Source changes invalidate a verdict; pin a fresh source
sha each round.

## Requesting a review

A request names both reviews: the contract review above and the
[rust-principal-review agent](../../../../../.agents/agents/rust-principal-review.md),
which scores seven Rust items PASS or FAIL on the same merged source and
writes `evidence/checkpoint/rust-review.json`. A FAIL from either stops the
landing. A request carries three receipts, and a request without all three is refused by
the reviewer, because what a linter or check catches is resolved before a
person reads the source:

1. `tm --root crates/ryo check <the review's scope directories> --format json`:
   tm over the review scope is 0 after excluding findings each named in the
   request with its owning OPEN design row (row id, rule, count); an unnamed
   finding fails the receipt.
2. `python3 crates/ryo/docs/contracts/evidence/checkpoint/run.py --check` exits 0.
3. `validation.json` records the `no_print` clippy target green and file-level
   ownership cycles 0.

The request points at the checkpoint's integration record, and the record's
pass/error table is part of it: one row per `cargo check` pass with the errors
that pass reported and the errors it fixed.

## Review priorities

The five adversarial bullets in the agent file apply unchanged: callers and
bypasses, authority and identity, derived-kind ownership, shared-walk coverage
of the configured 413 rules, and real positive and negative tests with TM
enforcement. See "Adversarial pass" and "Architectural enforcement" there.

## Review evidence schema
Every review evidence file (rust-review, contract-review, leak-review, tm-steward and the per-branch leak result) has this shape, and `.claude/hooks/guard_shell.py` refuses a file that does not:

```json
{"source": "<40-hex>", "verdict": "PASS"|"FAIL"|"NOTE"|"BLOCKING", "reviewer": "<subagent name>",
 "checks": [{"id": "...", "verdict": "...", "evidence": "..."}], "findings": [...]}
```

Evidence JSON carries no tracker ids (the no-tracker-text-shipped check flags them): an advisory in `findings` reads `open item: <short title>`, and its bead id goes only in the Markdown report, where the PM files the bead.

`source` is the full sha reviewed, never a ref name. `reviewer` is the agent's frontmatter name (`rust-review`, `contract-review`, `leak-review`, `tm-steward`); `evidence/checkpoint/asymptote.json`, the integrator's quick-check record, is the one file whose `reviewer` is `integrator`. `checks` is a list (the items, criteria or gate results of that agent); further fields (`checkpoint`, `date`, `plants`, `gate`, `classes`) may sit beside them.

One definition validates this shape: `shape` in `ci/contract_review/service.py`. Its two callers are `review_shape` in `.claude/hooks/guard_shell.py` (which loads it by path and returns its message; it checks every push and merge that reads evidence, including `evidence/checkpoint/asymptote.json` when present) and the `contract-review` CI job. `VARIANTS` in `ci/contract_review/service_test.py` is the behaviour table; the `ci-tests` tool reruns it when either file changes.

## Known red

Each is named with its reason in `validation.json` (`results`) and in the
checkpoint 319 integration record in [the records](../records.md).

- `core_allocation` (core `allocation` test, exit 101, about 1,880 blocks per
  file): a Phase 7 measurement.
- Cli `replay` target (validation319): 73 passed / 5 failed, the five being
  `results.cli_replay_reds`: three behaviour tests, the cache store size test and
  the warm peak test. The sixth red of checkpoint 318,
  `behavior::ruff_findings_equal_the_expected_findings`, passes.
- Cli `cli_cost`: 3 of 7 ratchet rows fail until the perf ratchet is seeded on a quiet machine.
- Python `ruff` target: 11 passed / 1 failed, the I001 observation row of
  `engine_test`, which was green at checkpoint 318; the same step-key anchoring
  fails five `project_phase::restoration::partition` rows of `boundary_contract`
  (98 passed / 5 failed).
- Execution library 549 / 2 and core library 108 / 2: the two `errored_rule` rows
  and two review-repair rows named in the record.

## Verification limits

Not covered by the latest validation (`validation.json`, validated source
`da1fe3eff3`, checkpoint 319, validation319; "not full product acceptance"):

- Whole-tree TM is measured, not clean: 2,762 findings, 0 suppressed (3,676 at
  checkpoint 318). The scoped `tm_source` run is 250 findings over sixteen
  directories, 200 in production files and 50 in test trees (the split is
  recorded in `validation.json`).
- No new performance measurement; the benchmark harness is not part of the set,
  and the route differential of the kernel consolidation is left to its
  quiet-machine run.
- The cli replay target compiles and runs; its five red tests are listed above.
- Language conformance targets print no pass count; exit 0 is the receipt.

Links: [matrix](../README.md), [execution plan](../execution-plan.md),
[reviewer instructions](../../../../../.agents/agents/contract-review.md),
[Rust reviewer instructions](../../../../../.agents/agents/rust-principal-review.md).
