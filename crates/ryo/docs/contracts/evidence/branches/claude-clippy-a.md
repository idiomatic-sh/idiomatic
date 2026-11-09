# Leak review: claude/clippy-a

Source `c8347684cd7229402f288a952155fa8734b81ec2`, base `279a19c614` (bead ryo-1k0y). Scope: values, cli, benchmark_contracts, sdk, go, rust, query_language, operators, derived, python and benchmark, plus the crates that import them. Reviewed read-only. Plants ran in a `git archive` copy, which was deleted afterwards.

## Behaviour-change hunt (the brief)
- **`try_from` conversions.** Every `x as usize` on a `u32` id became `usize::try_from(x).unwrap_or(usize::MAX)`. On 32- and 64-bit targets this conversion cannot fail, so `.get(...)` still returns `None` and indexing still panics exactly as before. Three spots were checked for a shifted bound and all are equal to the old code:
  - derived/src/domain/adjacency/service.rs:34 (`count <= u32::MAX - 2`).
  - adjacency/service.rs:84-90 (the vertex id bound).
  - adjacency/service.rs:104 (`+ 1` is now done on the u32 id, which is below `u32::MAX - 2`).

  The new `ShapeMismatch` paths in operators/src/domain/equality_join/service.rs:130 and :178 cannot be reached. `Arc::as_ptr(..) as usize` became `.addr()`, which gives the same value. In the values test, `i64::MAX >> 63_u32.saturating_sub(usize::BITS)` equals the old branch on both 32- and 64-bit.
- **Explicit arms.** Each one replaces a trailing `_` or `other`. They compile as exhaustive, so they match exactly the variants the wildcard matched. Three test rewrites were checked case by case and give the same results:
  - query_language/src/domain/refusing/fixtures.rs:212-222 (Distinct, Union, Intersection, Difference).
  - The operators `subject`/`selected` split (use_case_test.rs:152-260): SelectMissing gives Holds over [9, 8], and SelectRejected gives Fails over [9, 8].
  - In operators acceptance.rs:36-42, `unreachable!` became a debug string. That string cannot equal the expected numeric text, so a wrong cell still fails the test.
- **Fixtures that return Result.** No test case expects the error a fixture now returns when its own setup fails (`Refusal::Unexpected`, `Failure::Unexpected`, `"wire bytes"` and the like): grep of every `Expect::Error` and `Err(...Unexpected)` in the closure. Cucumber steps that return `Result` fail on `Err`. One caller drops the Result (check 2 below).
- **`#[expect]` reasons.** Each was checked against its code, and all are true:
  - token_links closing: the 3 bracket kinds.
  - lexical `complete` and `has_unclosed_delimiter`.
  - semantic `target`: Name, Attribute, Subscript, Tuple, List and Starred are the 6 ruff `Expr` variants that carry a `ctx`.
  - semantic `render`.
  - numeric_literals `write_float`: f64 Display writes only ASCII.
  - The two benchmark benches: the literal `RuleId` "fixture".
  - The walk fixture: the async For, With and FunctionDef statements.
  - demand_test and the selecting test: a wildcard that panics, which is the failure the test reports.
- **cfg(test) marking.** It applies only inside the `[[test]] ruff` target (languages/python/Cargo.toml:89-91, default harness), where cfg(test) is always on. bench.rs includes none of these modules. Planted type errors in schemas_test.rs and engine_test.rs were both reported when the ruff test target was compiled, so no test was dropped.
- **DerivedKind check.** It is sound. `ALL` and `ordinal` are generated from the same macro list, so `ALL` holds every variant once, in declaration order. `ordinal(ALL[i]) == i` therefore makes the written ordinal equal the declaration index. Both plants failed to compile with E0080: swapping two ordinals, and appending a kind with a duplicate ordinal.

## Check 1: proof where the contract lives: PASS
The only change under `languages/*/benchmarks` is the cfg(test) marking in the ruff test target (schemas_test.rs:2, tests.rs:6). No engine, core or SDK contract is newly asserted from there.

## Check 2: every enforcement claim fires: FAIL
- crates/ryo/crates/benchmark/src/outbound/suite/fixtures/contract.rs:279 `check` now returns `std::io::Result<()>`. It returns `Err` before running any case when its tempdir or its `rules` directory cannot be created; before this branch it panicked through `expect`.
- crates/ryo/crates/benchmark/src/outbound/suite/fixtures/fake_test.rs:8 calls `check(&SuiteFake::default());` and drops the Result. rustc only warns ("unused `std::result::Result` that must be used", fake_test.rs:8:5), because `unused` is warn in the workspace lints. So SuiteFake's contract test passes with zero cases run when setup fails. Its twin in receipts (receipts/fixtures/fake_test.rs:4) was given `.unwrap()`; this one was missed.
- The class was enumerated: compiling all targets of the 11 touched packages reports no other dropped `Result`.
- Claims that do fire:
  - The DerivedKind check: 2 plants, each reported.
  - The ruff cfg(test) modules: 2 plants, each reported.
  - The four deny-level clippy lints fired on the real tree at base, in /tmp/ryo-contract-validation334/clippy-at-0563dff6b3.log: as_conversions 48, wildcard_enum_match_arm 11, expect_used 5, unwrap_used 1. The findings are at the files this branch fixes, e.g. derived/src/domain/adjacency/service.rs:101 and query_language/src/domain/planning/service.rs:271. The reviewer cannot run clippy, because guard_shell.py allows clippy only in an integration worktree. Probe that would settle it: the integrator's clippy run at this sha, which should show 0 of these four lints in the touched crates.

**Required change:** `check(&SuiteFake::default()).unwrap();` at crates/ryo/crates/benchmark/src/outbound/suite/fixtures/fake_test.rs:8. Enforcement candidate `dropped-must-use-result-f839521b` (mechanism `compiler`): `unused_must_use = "deny"` in `[workspace.lints.rust]`.

## Check 3: boundaries hold across crates: PASS
No new `pub` in production code. The signature changes are fixtures behind `cfg(any(test, feature = "fixtures"))` (operators.rs:8-9) or in test files. Go's roles fixtures stay `#[cfg(test)]` (roles.rs:10-11).

## Check 4: one implementation per concept: PASS
assembling now uses the one step lookup, `resolving::service::step_of` (assembling/service.rs:201).

Advisory (`sentinel-index-hides-panic-62cdc96e`): planning/service.rs:270 still indexes `observation_inputs` inline with `unwrap_or(usize::MAX)`. It has no reasoned `indexing_slicing` expect, unlike `step_of` and `parameter_of` (resolving/service.rs:81-92), and the `expect` message it used to have is gone.

## Check 5: layer direction: PASS
The one new call is domain to domain (assembling to resolving). No inbound or application item is newly imported across crates.

## Check 6: test seams stay out of production: PASS
The cfg(test) marks are inside a test target. No production build reaches a new seam.

## Notes
- The brief says all 48 mod_module_files fixes were reverted, but commit 15e08bf977 keeps one: go's roles/fixtures/mod.rs became fixtures.rs. It is a fixtures module, not a test directory, so it does not touch the test-directory-layout question.

VERDICT: FAIL: 2
