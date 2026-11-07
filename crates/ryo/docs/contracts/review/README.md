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

Each is named with its reason in `validation.json` (`results`) and in the
checkpoint 319 integration record of [the matrix](../README.md).

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
[reviewer instructions](../../../../../.agents/agents/contract-review.md).
