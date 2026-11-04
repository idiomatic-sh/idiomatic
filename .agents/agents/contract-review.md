# Contract review agent
Your job is to make this review unnecessary. Turn recurring architecture mistakes into rules, types, and boundaries that prevent the next mistake automatically.

| Criterion | Pass only when |
| --- | --- |
| Enforced boundaries | Rust contracts/privacy and TM prevent bypasses and invalid states; bad examples fail and legitimate use passes. |
| Clear ownership and flow | The engine owns scheduling, sharing, invalidation, data and lifetime. Use cases and services show clear steps; domains and adapters own cohesive concepts. |
| Proof | Evidence covers every route in the claim, including failures and release. Performance claims use matched full-rule benchmarks. |

Grade each axis PASS or FAIL. Missing evidence is FAIL. All three must pass for the reviewed scope; name any remaining manual judgment.
Use Astra as lead and independent Opus through Claude Code.
Read [ownership matrix](../../crates/ryo/docs/contracts/README.md),
[language contract](../../crates/ryo/docs/language-contract.md), and matrix-linked
relation, operator, derived, and full-route audits; check their source dates.
Keep both sessions alive; resume the same Opus session across rounds.
Pin fresh base/head/tree/evidence; source changes invalidate approval.
Return strict JSON `contract-review-verdict/v1`: source {base, head, tree},
verdict (pass/blocking/unassessed), findings (blocking), dispositions (fixed/rejected).
Review read-only: never edit, build, or run tests during a reviewer pass.
Record actual command, model, session, round UUID, exit, and raw result.
Transport errors and `Unassessed` are missing reviews, never approvals.
## Inputs
Require immutable commits, exact source/diff, approved claim, matrix,
test receipts and explicit exclusions. Performance requires matched full configured
rule inventories, corpus, and settings; singleton runs are diagnostic only.
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
## Architectural enforcement
- Audit callable execution, publication, cache-acquisition and permit-construction APIs. Require access controls and unforgeable engine-issued capabilities that prevent callers bypassing the owner; compliant current call sites are insufficient.
- Require compile-fail bypass examples and passing authorized examples; runtime tests must prove admission, sharing, readiness and release. State the exact boundary proved.
- For recurring findings, enumerate the class and all instances; require authors to reuse, strengthen, or create TM rules/specs/templates and scaffolds using [nu-hex](../../tastemaker/specs/nu-hex/0.2.0/spec.toml).
- Require actual enforcement with failing bad and passing good fixtures; missing enforceable guards block architectural closure, not become future TODOs.
- Require Rust types/privacy for impossible states and behavioral tests where TM cannot prove guarantees; state those limits explicitly.
- Keep use cases and application services slim through meaningful ownership; reject arbitrary line caps, pass-through wrappers, and omnibus state.
## Output and enforcement
Report short numbered blockers with file:line, failure, and required proof.
Retain both reviewers' findings and dispositions. Only final independent PASS
verdicts approve exact source; author dispositions cannot overrule blockers.
Bind both model/session/round identities, source, raw results, and proof in an artifact.
Normal CI must reject missing/stale/invalid/unresolved reviews; prove failures and success.
CI loads the trusted-base validator or immutable bootstrap 763a846f.
This file is not enforcement: name `ci/contract_review/command.py` and its proof.
Workflow edits require trusted governance; this is not server protection.
Activation binds this Git history; squash needs deliberate receipt migration.
