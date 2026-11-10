# Leak review: claude/checkpoint-334 at 79c1329bdf (narrow re-check)

Source `79c1329bdf3382a990054948b36b2f107e6e2e81`. Delta reviewed: `a33f361f7a..79c1329bdf`. The prior review at `a33f361f7a` failed check 2 only; checks 1 and 3 to 6 (and 16 to 18) passed and carry forward. The delta changes Rust in two test lines only and touches no `Cargo.toml` or `Cargo.lock`.

## Check 1: proof where the contract lives: PASS
Carried forward. The delta adds no Rust proof and no benchmark or language-directory assertion.

## Check 2: every enforcement claim fires: PASS
- `ci/scripts/checks/sdk_api.py:65-82`: `internal_crates(metadata, package)` now walks cargo metadata's resolve graph from the SDK over normal edges (`kind is None`, `:77`) to workspace members, transitively. `check()` passes it the full resolve (`:180`, `cargo metadata --locked` without `--no-deps`; `:185`).
- Real tree, in a `git archive` copy of 79c1329bdf: on cargo metadata (default features and `--all-features` give the same set), the set holds 12 crates: ryo_core, ryo_derived, ryo_execution, ryo_execution_domain, ryo_execution_runtime, ryo_execution_store, ryo_language, ryo_matching, ryo_operators, ryo_query_language, ryo_relations and ryo_values. It reaches no language provider, benchmark or the CLI.
- The `leaks()` probe (`sdk_api.py:84`) ran on a one-function rustdoc document. It reports 1 for each of the 12 crates, under both the bare and the `idiomatic_sh_` spelling. It also reports 1 for `ryo_execution_domain::Covered` with its `resolved_path` written as `ryo_execution::Covered`, the forwarded re-export. serde, rayon and thiserror each report 0.
- `sdk_api_test.py:286` covers a forwarded member and a member that is only a dev-dependency (`demo_bench`). `sdk_api_test.py` passes 17/17 in the archive copy.
- `run.py:26-33` (`raw_dir`, `raw_identity`) only translate `{temp}`. A missing raw directory still produces failures in `reruns_without_log` and `fix_problems`, so no failure can turn into a pass.

## Check 3: boundaries hold across crates: PASS
Carried forward. No manifest, visibility or feature changes in the delta.

## Check 4: one implementation and one representation per concept: PASS
Carried forward. `sdk_api.py` still has one definition of the internal set and no list to keep.

## Check 5: layer direction: PASS
Carried forward.

## Check 6: test seams stay out of production: PASS
Carried forward. Both Rust edits are in test files: `conformance_scale/test.rs:228` (`usize::from(emit)`) and `population_reads/test.rs:150` (`u64::try_from(..).expect(..)`). They replace `as` casts, and the `2b4338...` operations lock drains the two matching clippy records.

## Record
The record lines added by the delta contain no machine path and no tracker id. `validation.json` names `raw_identity` as `{temp}/ryo-contract-validation334`, and command paths use `{target}` and `{temp}`. The only `ryo-` tokens are package names and directory names.

## Advisory
- `conformance_scale/test.rs:229`: `.0[..inputs.len()]` was dedented out of rustfmt shape by 91c711ca46. This is formatting only, not a leak.

## Enforcement candidates
All earlier ids are carried forward. `sdk-api-internal-crates-direct-dependencies-only-8edfee08` is now `fixed`. `sdk-api-internal-crate-list-hand-written-8fa957e4` stays `fixed` and `home-grown-libtest-argument-protocol-e9dbc575` stays `open`. No new candidates.

VERDICT: PASS
