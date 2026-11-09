# Rust gate, checkpoint 333, second pass: PASS (7 of 7)

- Source reviewed: `e4f1d396eb8cdd57ecf77357ad72b58b9487b76a`.
- Base: `6af005e9f749d03192fb87f68db577ef42944ce5`, where the first pass failed item 2 (F1).
- Scope: only the delta `6af005e9f7..e4f1d396eb`. Commits in it:
  - `8bf8ae89a1`: the repair pass (F1, leak 332 checks 2a, 2b and 3);
  - `7e5beb9128` and `563c0bc69b`: the unfed list loses `relation:Entry` and `relation:Mapping`;
  - `e4f1d396eb`: the record (docs and evidence only);
  - the merged docs-only commits `3d1e6d91a4`, `623ef2feaa` and `226de88f02`.

  The full validation ran at `563c0bc69b`. After that only docs and evidence changed.
- Tree: HEAD was `e4f1d396eb` both before and after the pass, and `git status --porcelain` was empty both times.

## 1 Ownership and API shape: PASS
- The only public-surface change is the version bump of the ryo workspace from 0.1.2 to 0.1.3. It is at `crates/ryo/Cargo.toml:23`, in every crate's path dependencies (for example `crates/sdk/Cargo.toml:18-26`), and in Tastemaker's registry requirements (`crates/tastemaker/cli/Cargo.toml:35-46`).
- `8bf8ae89a1` gives the reason: 0.1.2 is a released tag, and the local crates carry unreleased API. Tastemaker keeps the `[patch.crates-io]` until release (`crates/tastemaker/Cargo.toml:238`).
- No `api.txt` and no SDK source changes.
- No clone, `Arc` or interior mutability is added.

## 2 Compiler over tests: PASS
- F1 is repaired. `failure()` now lists the 15 remaining `Operation` variants as `RowFailure::Never` (`crates/ryo/crates/execution_domain/src/domain/operator/contract/service.rs:230-244`). This matches the exhaustive form of `presence()` and `invalid_patterns`, so a new variant is now a compile error rather than a silent `Never`.
- The compile ran green at `563c0bc69b` in the full validation. The arm is therefore exhaustive over the enum as it stands.
- The delta adds no new wildcard and no test that reads source.

## 3 Hot path: PASS
- The only new Rust code is test-side. `replay_languages()` (`cli/tests/fixtures/replay/base.rs:336-342`) builds a two-element `Vec` once per lint composition, not per file or per row.
- No production loop, map or dispatch changes.

## 4 One representation per concept: PASS
- The pending check, the hosted lint and the counting lint each had their own language list. They now share one `replay_languages()` (`base.rs:336`, `:352`, `:488`; `counting.rs:176`).
- The delta removes a duplicate and adds none.

## 5 Standard crate over home-grown: PASS
- No dependency is added in either workspace.
- In `crates/tastemaker/Cargo.lock`, 11 dependents re-resolved from `windows-sys 0.60.2` to `0.61.2`. Both versions were already locked (`:4218-4228`), so this is not a new crate (see A2).

## 6 Tests: PASS
- **Leak 332 check 2a, a harness change.** `run.py` `own()` now refuses an `OWNERS` entry whose result or replay test is not red (`run.py:269`), and the 30 stale entries are deleted. This tightens the harness: a red with no owner is still reported (`:272-278`).
  - `8bf8ae89a1` names the test `test_a_green_result_with_only_a_stale_owner_fails_the_check` (`run_test.py:102`).
  - `validation.json` `red_green` records it as red at base `8e4d22b3a5` and green at merge `563c0bc69b`.
  - The existing test now uses the `owned_reds()` fixture (`run_test.py:89`), so the new refusal does not fire on its setup. Its assertions are unchanged.
  - The 7 owners that remain match today's reds in `validation.json`: `benchmark_library`, `core_allocation`, `cli_cost` and `tastemaker_workspace` are failed results, and the three `::` keys are exactly `cli_replay_reds`.
- **Check 2b.** Deleting `recordings/dependency-feature-sprawl/pending.txt` turns a waiting recording into a compared one: its 6 recorded rows are now checked.
- **The unfed list.** Removing `relation:Entry` and `relation:Mapping` from `fixtures/unfed.toml` is what the shrink-only test `ruff_rules_feed_every_catalog_feature_a_thousand_rows` demands once they are fed (`answers.rs:517-527`).
  - `7e5beb9128` and `563c0bc69b` state the reason.
  - `SequenceElement` stays in the list, and its reason now carries the measured count of 268 rows (`unfed.toml:50-51`). The test was run green in a narrow run.
- **F1** is a compile-only change with no behaviour change. The existing presence `service_test` stays the behaviour proof.
- The delta adds no skip, `continue`, caught error, filtered result or wall-time assertion.

## 7 Rust fit: PASS
- The delta adds no `unsafe`, no `unwrap` or `expect`, no panic and no `#[allow]` (`service.rs:199-245`, `base.rs:333-492`, `counting.rs:157-177`).

## Repaired
- F1 (item 2, first pass) is repaired in `8bf8ae89a1` (`service.rs:230-244`).

## Advisories (open-source hygiene; for each, the PM files a bead or drops it with a reason)
- **A1, the doc of `replay_languages` says more than the code does.** `base.rs:333-335` and the 333 record (`crates/ryo/docs/contracts/README.md:102`) say that the hosted lint links these languages.
  - In fact `lint_report_with_languages` links them only when `manifests_read` is true. Otherwise it links Rust alone (`base.rs:485-492`).
  - This is not a leak. The replay path always passes `true` (`base.rs:431`), and a relation that `provided()` reports but a Rust-only run lacks makes the run fail loudly, not pass.
  - Fix: reword the doc to "the pending check, the counting lint, and the hosted lint when it reads manifests". Alternatively, drop the `bool` and always link `replay_languages()`.
- **A2, the Tastemaker lock churn is unexplained.** In `crates/tastemaker/Cargo.lock`, 11 `windows-sys 0.60.2` references became `0.61.2` in the bump commit, and no message mentions it.
  - Fix: state the re-resolution in the record, or regenerate the lock with only the version bump (`cargo update -p idiomatic-sh-ryo-sdk --precise 0.1.3` style).

The JSON lists both advisories as `open item: <title>`. Their bead ids are for the PM to fill in when filing.
