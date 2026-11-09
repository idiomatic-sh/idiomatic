# Leak review: checkpoint 333 (landed)

Source `40e49021520d4392d29df28ee5bd2fef739e5d37`, base `dcea2ee7df7c02711f42c332cbc403ff7b84a034`. The four branch heads already have leak PASS files under `evidence/branches/`. This review covers what integration added on top of them: the merge repairs and file splits (`a73aa2bee8`, `28efeb7bf0`, `5339ed5df3`), the target-supplement repairs, `own()` and the removal of stale owners, `replay_languages()` and the deletion of `pending.txt`, `unfed.toml`, the 0.1.3 bump (`8bf8ae89a1`, `7e5beb9128`, `563c0bc69b`), the tracker-pattern fix (`98e0fcb703`, `40e4902152`), the removal of `asymptote.json` (`5e2aad3bf1`) and the census (`6451254521`). The closure is execution, execution_domain, execution_runtime, values, operators, the cli replay harness, the python ruff authoring package, `ci/contract_review`, `.claude/hooks`, and the Tastemaker manifests. The reviewed tree was read-only: HEAD and `git status --porcelain` were the same before and after. The plants ran in a `git archive` copy with its own check target, and both were deleted afterwards.

## 1 Proof where the contract lives: PASS
- The F1 exhaustiveness guard is in execution_domain (`operator/contract/service.rs:206-245`).
- The stale-owner refusal is tested by the runner's own test (`run_test.py:370-377` against `run.py:263-269`).
- `replay_languages()`, the pending check and `unfed.toml` stay in the cli replay harness (`cli/tests/fixtures/replay/base.rs:336-357`, `counting.rs:176`).
- The target-supplement validator stays in the python ruff authoring package (`authoring/target_supplement.py:17-27`).
- No engine, core or SDK contract is newly asserted from `benchmarks/` or `languages/`.

## 2 Every enforcement claim fires: FAIL
Plants (all reverted, since the copy was deleted):
- F1 exhaustiveness guard: a `Planted` variant on `Operation` (`values/src/domain/plans/types.rs:67`). `cargo check -p idiomatic-sh-ryo-execution-domain` reports E0004 at `operator/contract/service.rs:206`. Count 1.
- `own()` with a stale owner: adding `'tm_import_cli':CLOSE` (green) to OWNERS makes `run.py --check` exit 1 with `stale owner: tm_import_cli`. Count 1. Deleting `run.py:269` turns `test_a_green_result_with_only_a_stale_owner_fails_the_check` red. Count 1.
- The tracker validator against a candidate class "see " followed by a tracker id with a child number: refused. Count 1. The same id preceded by `bead-`: accepted. Count 0.

Findings:
- **(a) A gate group was dropped. New in `5e2aad3bf1`.** The TM steward's gate became `asymptote check --groups lint,check` (`.agents/agents/tm-specialist.md:18`, `evidence/checkpoint/README.md:5`). The deleted `asymptote.json` ran `lint,check,clippy` and recorded the clippy group red. No checkpoint gate now runs the clippy group (`crates/ryo/asymptote.toml:30-37`): not the steward, not the Rust gate, not `checkpoint.yml`, and `commands.json:84` is a print-only clippy over five crates. A clippy denial now passes every checkpoint. The owner's decision (contracts README:336) removed the record file, not the group. `tm-specialist.md:32` still shows `lint,check,clippy`. **Required change:** the steward's gate runs `--groups lint,check,clippy` (tm-specialist.md:18 and :32, checkpoint README:5, the decision row), and the clippy red stays owned.
- **(b) The tracker validator was narrowed. New in `98e0fcb703`.** `TRACKER` (`ci/contract_review/service.py:55`) now has the lookbehind `(?<![\w-])`, so a tracker id preceded by a hyphen passes the validator, yet bare `ryo-core` is still refused. The validator and `idiomatic.arch/no-tracker-text-shipped` (`tastemaker/specs/idiomatic-project/0.3.1/spec.toml:16`, which needs a digit or a child number) are two definitions of a tracker id. **Required change:** one definition. Either the validator reads the rule's patterns, or its regex gets the rule's shape (a digit or a child number, any prefix) in place of the hyphen lookbehind.
- **(c) Tests pass when their input is absent** (they were there before 333, but the diff touches or cites these tests):
  - `test_target_supplement.py:189-190` calls `skipTest` when `latest-failed-observations` is absent.
  - `test_target_supplement.py:179-180` silently uses `ruf101-target-supplement` in its place. This test is cited in validation.json `red_green`.
  - `test_preflight_staged_profile.py:56-59` uses `skipUnless` on the worker environment.
  - cli_cost's Django and one-file rows pass with a `NotAvailable` reason when `RYO_PREFLIGHT_WORKER` is unset (`cli/tests/fixtures/replay/workload.rs:148`).

  **Required change:** each test asserts that its input is present and fails when it is absent. Drop the fallback target. cli_cost's absent-worker rows count as not run, not passed.

These hold:
- `own()` refuses a stale owner (`run.py:269`).
- `pending.txt` cannot outlive provision: `base.rs:285-294` asserts `absent == expected` over `replay_languages()`.
- `unfed.toml` only shrinks (its header) and both commits only removed entries.
- `28efeb7bf0` keeps every assertion: after whitespace normalisation, the assert counts are equal in all 16 changed `.rs` files.

## 3 Boundaries hold across crates: PASS
- The 0.1.3 bump closes leak 332's check 3. `crates/ryo/Cargo.toml:23` and every path dependency are 0.1.3, and no ryo manifest still names 0.1.2.
- Tastemaker requires the unpublished SDK 0.1.3 (`crates/tastemaker/crates/core/Cargo.toml:33`, `cli/Cargo.toml:35-46, 63-66`), so its requirement no longer names a released API. It names only the SDK, and the `[patch]` serves the transitive crates.
- The new fixtures modules are `cfg(test)`.
- execution's `fixtures` features are dev-dependencies only (`execution/Cargo.toml:30-37`).

## 4 One implementation per concept: PASS
- The skip-rules merge kept one file-demand table. 332's memo was dropped with its test, not kept beside the new table.
- `trust/reads.rs:77` calls the single `output_origins` (`operation_schema/projection.rs:139`).
- `replay_languages()` is now the one language set for the pending check, the recording lint and the counting lint, which closes 332's split.
- The splits move code; they do not copy it.
- Advisory: the tracker-id pattern exists twice, outside any crate. It is recorded under 2(b).

## 5 Layer direction: PASS
- The presence probes moved within `execution_runtime` `application/file_dag/program/facade_runtime` (`file_execution/presence/service.rs:352-390`).
- The execution_domain merge-repair edits are fmt and import order, plus the lazy select scratch.
- F1 changes one match in `domain/operator/contract`.
- No inbound or application item crosses a crate.

## 6 Test seams stay out of production: PASS
- `region_probe` and `retire_again` keep `#[cfg(test)]` after the move.
- `executable_program/fixtures.rs` and `conformance/fixtures.rs` are `cfg(test)` modules.
- `acceptance` stays behind `cfg(any(test, feature = "fixtures"))` (`execution_runtime/src/application/file_dag.rs:21-22`) and is switched on only from dev-dependencies.

## Enforcement candidates
These are new at 333. They are not carried from 332, because this is not a re-review of 332. In 332's terms: `stale-red-owner-5fdfc825` is fixed by `own()`, `cross-product-path-patch-a711e823` is fixed by 0.1.3, `replay-language-set-split-e2460590` is fixed by `replay_languages()`, and `test-only-import-in-production-allowlist-77f00ace` is still open (`crates/ryo/tastemaker.toml:205` still allows `std::sync::atomic::*`).
- `checkpoint-gate-drops-a-quality-group-d347eb4d`: new rule.
- `absence-gated-test-pass-080c5466`: strengthen `idiomatic.arch/no-ignored-tests`, which today covers only Rust `#[ignore]`.
- `tracker-id-definition-split-78f9a5a1`: strengthen `idiomatic.arch/no-tracker-text-shipped`, as the single definition the validator reads.

VERDICT: FAIL: 2
