# Contract review agent
Use Astra as lead and independent Opus through Claude Code.
Read [ownership matrix](../../crates/ryo/docs/contracts/README.md),
[language contract](../../crates/ryo/docs/language-contract.md), and matrix-linked
relation, operator, derived, and full-route audits; check their source dates.
Keep both sessions alive; resume the same Opus session across rounds.
Pin fresh base/head/tree/evidence; source changes invalidate approval.
Print `Source: base=<sha> head=<sha> tree=<sha>` in each raw verdict.
Review read-only: never edit, build, or run tests during a reviewer pass.
Record actual command, model, session, round UUID, exit, and raw result.
Transport errors and `Unassessed` are missing reviews, never approvals.

## Inputs
Require immutable commits, exact source/diff, approved claim, matrix,
test/performance receipts, and explicit exclusions.
Record HEAD/status before and after; discard source-mutating passes.
Check source/evidence hashes; reuse only proven-identical executed postimages.
A bounded review never closes wider architecture categories.

## Adversarial pass
- Trace shared layers through real imports and dependency edges.
- Enumerate all callers, bypasses, fallbacks, and alternate routes.
- Prove mandatory scheduling, cache identity/invalidation, ownership, and lifetime.
- Trace guarantees from configured input through engine to observable output.
- Check stale populations, foreign owners, errors, early exits, and retirement.
- Require independent negative controls that reject weaker implementations.
- Reject self-derived inventories, string markers, and checkbox-only proof.
- Match matrix claims to executable evidence and its actual breadth.
- Keep orchestration thin; enforce shared guarantees in shared contracts.
- Missing evidence for a claimed guarantee blocks approval.

## Output and enforcement
Report short numbered blockers with file:line, failure, and required proof.
Retain both reviewers' findings and dispositions. Only final independent PASS
verdicts approve exact source; author dispositions cannot overrule blockers.
Bind both model/session/round identities, source, raw results, and proof in an artifact.
Normal CI must reject missing/stale/invalid/unresolved reviews; prove failures and success.
CI loads the trusted-base validator or immutable bootstrap 871be75a.
This file is not enforcement: name `ci/contract_review/command.py` and its proof.
Workflow edits still require trusted governance; do not claim server protection.
