# Engine consolidation review

Reviewer brief for the integration branch `claude/ryo-consolidation`, which
carries the engine consolidation (epic ryo-9ukc) toward the phases in
[the matrix](../README.md). One phase is active at a time; integration is never
red except for the known-red list below.

## Branch

Checkpoints 310-317 are evidence commits on the branch; each names the batches
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

## Review priorities

The five adversarial bullets in the agent file apply unchanged: callers and
bypasses, authority and identity, derived-kind ownership, shared-walk coverage
of the configured 413 rules, and real positive and negative tests with TM
enforcement. See "Adversarial pass" and "Architectural enforcement" there.

## Known red

- `core_allocation` (core `allocation` test, exit 101, about 1,880 blocks per
  file): a Phase 7 measurement, bead ryo-l3mj.
- Python `ruff` target, Phase 1: 11 passed / 1 failed,
  `i001_hidden_match_and_ruf100_output_follow_first_import_and_unknown_inventory`
  (the syntax-poisoning case), bead ryo-xqxb.

## Verification limits

Not covered by the latest validation (`validation.json`, source `18f96bccd7`,
checkpoint 317; "not full product acceptance"):

- Whole-tree TM is measured, not clean: 4,251 findings, 0 suppressed.
- Derived-scope TM findings are deferred (`tm_source`: 53 findings, 21 of them
  derived-scope bare integers/bool).
- No new performance measurement; the benchmark harness is not part of the set.
- Cli replay/blocks/lint tests and the core performance bench do not compile
  (bead ryo-4i09).

Links: [matrix](../README.md), [execution plan](../execution-plan.md),
[reviewer instructions](../../../../../.agents/agents/contract-review.md).
