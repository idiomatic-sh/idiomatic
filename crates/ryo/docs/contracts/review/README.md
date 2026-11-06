# Engine consolidation review

This branch is a **combined work-in-progress snapshot, not a passing candidate**.
It includes all active scheduling/walk, issuer/proof, test-placement and 13-kind
derived-view changes. Phase4 source was frozen at `22f955ed4f`; its overlap with
the newer runtime/preparation ownership split was resolved here without a build.

## Context

Ryo must reach full413-rule Ruff parity and eventually subsecond full-Django
execution with low memory, preserving Rust correctness/performance. The current
lift replaces dispersed per-rule scheduling and resource policy with one
request-wide program and a demand-driven shared file walk.

Keep the agreed architecture: semantic crates expose inbound APIs; execution owns
scheduling, admission, sharing, invalidation and lifetime. Required capabilities
must prevent bypasses. No optional owners, alternate executors or compatibility
reexports. Use cases/services should make control flow obvious; concept names
belong in directories and roles in filenames.

Work order: phases **2–6 consolidation**, then phase1 output repairs and phase7
acceptance. Output regressions may wait; architectural violations may not.
No new performance runs during consolidation.

## Review

- [ ] Trace every constructor, acquisition, dispatch, replay and release route;
      prove callers cannot forge or retarget authority, including aliases/Self.
- [ ] Verify exact publication/proof, owner/generation and descriptor/source
      identity. Check same-owner mismatches, stale inputs and failure paths.
- [ ] Check all13 derived kinds use the common acquisition/lease owner, including
      raw ordering, graph views and selection. A moved crate is not proof.
- [ ] Check shared-walk coverage includes synthetic observation unions and every
      admitted class. Distinguish configured413 coverage from representative tests.
- [ ] Inspect real positive/negative tests and TM enforcement; flag missing routes,
      dependency cycles, mixed responsibilities and stale matrix claims.

Return PASS/FAIL for **enforced boundaries**, **ownership/readability**, and
**proof coverage**. For each blocker give file:line, a concrete failing example,
its rightful owner, and the enforcement that would prevent recurrence.

## Evidence and limits

- Last successful combined build: `36e92e315`; execution433 passed/1 project
  lifecycle failure, public boundaries90/90, Core175 passed/6 deferred failures.
  Actual413 sharing failed before execution on a missing tracked fixture path;
  the fixture correction is now present but unverified.
- Newer issuer build on `ce26dd006a` failed with import/column-inference and test
  ownership errors. Import and Rc teardown repairs are included here, unverified.
- **This combined Phase4 snapshot has not been compiled or tested.** Its author
  reported75 TM findings; exhaustive13 owner tests, raw-order lifetime controls
  and Phase4 Rust/TM bypass enforcement are unfinished.
- Runtime lifecycle tests still use a nonstandard `#[path]` registration;
  conventional placement is required. Broader cycles/TM findings remain open.
- No phase is complete; no performance gain is claimed.

Start with [the matrix](../README.md), [execution plan](../execution-plan.md),
[paired review](../audits/2026-11-06-ownership/review.json), and
[reviewer instructions](../../../../../.agents/agents/contract-review.md).
Source hotspots: `execution/src/application/file_dag/program/`,
`execution/src/application/file_dag/derived/owner/acquisition/`,
`execution/src/domain/shared_walk/`, and `derived/src/inbound/use_cases/derived/`
(relative to `crates/ryo/crates`). The production sharing proof is
`languages/python/benchmarks/linters/ruff/sharing/` under `crates/ryo`.

First checkpoint: build execution, derived, operators, matching, values,
query_language, core, SDK and Python together (`--all-features --tests
--keep-going -j4`), then repair diagnostics in batches. Preserve the existing
build cache and serialize heavy commands. Run production boundary/lifecycle
and `ruff_sharing` tests after compilation; no performance run or suppression.
