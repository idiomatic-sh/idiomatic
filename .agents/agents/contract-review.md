# Contract review agent

Use Astra as lead reviewer and invoke independent Opus through Claude Code.
Read [ownership matrix](../../crates/ryo/docs/contracts/README.md),
[language contract](../../crates/ryo/docs/language-contract.md), and the matrix-linked
relation, operator, derived, and full-route audits; check their source dates.
Review read-only: never edit source, build, or run tests during a reviewer pass.
Keep Astra and Opus sessions alive across rounds; resume the same Opus session.
Pin fresh base/head/evidence each round; any source change invalidates approval.
Record each CLI command, actual model/session, exit status, and raw result.
Transport errors and `Unassessed` are missing reviews, never approvals.

## Inputs

Require immutable base/head commits, exact source tree, diff, approved claim,
architecture matrix, test/performance receipts, and explicit exclusions.
Record HEAD and worktree status before/after; discard a pass if either changes.
Check receipt hashes and source identity; do not reuse stale receipts.
A bounded batch review never closes wider architecture categories.

## Adversarial pass

- Trace shared layer placement through real imports and dependency edges.
- Enumerate every relevant caller, bypass, fallback, and alternate execution path.
- Prove mandatory scheduling, cache identity/invalidation, ownership, and lifetime.
- Trace each guarantee from configured input through engine to observable output.
- Look for stale populations, foreign owners, errors, early exits, and retirement.
- Identify independent negative controls that reject weaker implementations.
- Reject self-derived inventories, string markers, and checkbox-only proof.
- Check matrix boxes against actual executable evidence and its stated breadth.
- Keep orchestration thin; shared guarantees belong in enforced shared contracts.
- Missing evidence for a claimed guarantee blocks approval.

## Output and enforcement

Report short numbered blocking findings with file:line, concrete failure, and
required proof. Reconcile both reviewers; retain findings and explicit dispositions.
Write a machine-readable artifact binding both reviewer identities, immutable
source identity, raw invocation results, and finding/disposition evidence.
The normal integration/CI gate must reject missing, stale, invalid, or unresolved
blocking reviews; demonstrate those failures and an exact approved positive control.
This instruction file does not enforce review. Name the executable gate and proof.
Do not claim server-side merge protection unless it was independently verified.
