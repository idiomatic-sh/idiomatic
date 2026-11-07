# Engine consolidation review

Reviewer brief for the integration branch `claude/ryo-consolidation`, which
carries the engine consolidation toward the phases in
[the matrix](../README.md). One phase is active at a time; integration is never
red except for the known-red list below.

## Branch

Checkpoints 310-318 are evidence commits on the branch; each names the batches
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
runs each review. Astra is unavailable, so Opus is the single independent
reviewer. Each phase writes a verdict JSON (`contract-review-verdict/v1`) under
`audits/2026-11-06-phase<N>/`. A BLOCKING verdict gates the matrix tick for its
rows, not the merge. Source changes invalidate a verdict; pin fresh
base/head/tree each round.

## Requesting a review

A request carries three receipts, and a request without all three is refused by
the reviewer, because what a linter or check catches is resolved before a
person reads the source:

1. `tm --root crates/ryo check <the review's scope directories> --format json`
   reports 0 findings and 0 suppressed.
2. `python3 crates/ryo/docs/contracts/evidence/checkpoint/run.py --check` exits 0.
3. `validation.json` records the `no_print` clippy target green and file-level
   ownership cycles 0.

## Review priorities

The five adversarial bullets in the agent file apply unchanged: callers and
bypasses, authority and identity, derived-kind ownership, shared-walk coverage
of the configured 413 rules, and real positive and negative tests with TM
enforcement. See "Adversarial pass" and "Architectural enforcement" there.

## Known red

- `core_allocation` (core `allocation` test, exit 101, about 1,880 blocks per
  file): a Phase 7 measurement.
- Python `ruff` target: green at checkpoint 318 (12 passed / 0 failed,
  validation318); the syntax-poisoning case passes.
- Cli `replay` target (validation318): 72 passed / 6 failed, the six being the
  `(b)` group, named in `validation.json` (`results.cli_replay_reds`): the four
  behaviour tests `behavior::what_ryo_could_not_decide_over_ruff_is_the_golden_or_a_recorded_difference`,
  `behavior::ruff_findings_equal_the_expected_findings`,
  `behavior::every_recorded_rule_gives_the_recorded_rows` and
  `behavior::ruff_rules_feed_every_catalog_feature_a_thousand_rows`, and the cache
  store size and warm peak tests `cache::every_recorded_rule_gives_over_ruff_the_rows_a_warm_run_gives_again_from_a_small_store`
  and `peak::a_warm_unchanged_run_over_ruff_peaks_within_its_bound`.

## Verification limits

Not covered by the latest validation (`validation.json`, validated source
`c043a6dc68`, checkpoint 318, validation318; "not full product acceptance"):

- Whole-tree TM is measured, not clean: 3,676 findings, 0 suppressed. The
  scoped `tm_source` run is 344 findings over sixteen directories, 213 in
  production files and 131 in test trees (the split is recorded in
  `validation.json`).
- No new performance measurement; the benchmark harness is not part of the set.
- The cli replay target compiles and runs; its six red tests are listed above.
- Language conformance targets print no pass count; exit 0 is the receipt.

Links: [matrix](../README.md), [execution plan](../execution-plan.md),
[reviewer instructions](../../../../../.agents/agents/contract-review.md).
