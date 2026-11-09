# Rust gate: checkpoint 332, second pass

- Reviewed source: `c3639e10a62b7e89bc711d9c678dc018b6f59171`. The code repairs are in `10c561ef6a`; the record and `validation.json` are in `c3639e10a6`.
- Base: `b1e81f7242`, the checkpoint 331 landing. The first pass reviewed `6402b7dd06` and FAILED on items 1 and 6 (F1 to F4).
- Scope: `332`. This pass re-checks F1 to F4 against `git diff 7d7888247c..c3639e10a6`, and checks whether the repairs broke anything. The rest of the 332 diff is as judged in the first pass.
- Evidence read: the diffs, the files they touch, and the integrator's run logs `gatefix_*.log` in the validation332 directory. Under the review definition I ran no cargo.
- Tree: HEAD stayed at `c3639e10a6` for the whole pass. The only working-tree changes are this file and its JSON. The first pass's `rust-review.json` was never committed; this pass overwrites it.

**Verdict: PASS.** All seven items pass.

| Item | Verdict |
| --- | --- |
| 1 Ownership and API shape | PASS (F1 repaired) |
| 2 Compiler over tests | PASS |
| 3 Hot path | PASS |
| 4 One representation per concept | PASS |
| 5 Standard crate over home-grown | PASS |
| 6 Tests | PASS (F2, F3, F4 repaired) |
| 7 Rust fit | PASS |

## Checking the integrator's claims

| Claim | Verified | Evidence |
| --- | --- | --- |
| The doctests pass | Yes | `gatefix_doctests.log`: `Doc-tests ryo_execution_domain` reports 2 passed and `Doc-tests ryo_execution_store` 1 passed, both with 0 failed. |
| compare_corpus is green | Yes | `gatefix_compare_corpus.log`: 2 passed, 0 failed. `gatefix_benchmark_lib.log`: 229 passed, 1 failed. The failure is `verify_cases ... process_capture::test::succeeds`, red since validation326 and already listed. `gatefix_benchmark_bdd.log`: every scenario passes. |
| tm_import_cli exits 0 through the patch | Yes | `gatefix_tm_import_cli.log`: `imports::visibility_scopes_do_not_authorize_real_imports_types_or_calls ... ok`, 1 passed. `crates/tastemaker/Cargo.toml:238-253` patches the 15 ryo crates. Every third-party crate the lock gains has the same version in `crates/ryo/Cargo.lock`. |
| 6 new red_green entries, 3 of them compile proofs | Yes, with a correction to the count | `validation.json` entries 25-30. Only one is a behaviour red; the other **five** are compile reds, not three. See F4 below. |

## 1 Ownership and API shape: PASS

F1 is repaired.

**`PatternSemantics`** (`crates/ryo/crates/execution_domain/src/domain/matching/types.rs:22-37`):
- Its runnable example wraps `MatchingUseCaseImpl` in `PatternMatcher`, shows `compile_regex("(")` refusing, and shows `^ab+$` matching `abb` and differing on `ba`.
- It is sealed: `pub trait PatternSemantics: sealed::Sealed`, where `Sealed` lives in a `pub(crate) mod sealed` (`:13-20`).
- The only implementation of `Sealed` is the blanket impl at `application/matching/service.rs:13`, which sits beside the blanket `PatternSemantics` impl.

**`DerivedOwner::serve_project`** (`crates/ryo/crates/execution_store/src/application/file_dag/derived/owner/setup.rs:57-81`):
- It now states `# Errors` (`WrongOperation`).
- Its runnable example shows a fresh owner answering `Ok(())` and an owner after `prepare_program_file` answering `WrongOperation`.
- The example needed `owner/fixtures/mod.rs:11` `prepared_sources` to become `pub`. That module stays behind `#[cfg(any(test, feature = "fixtures"))]` (`owner/mod.rs:253`).
- Its return type `RequestDerived` is `pub` (`derived/prepared.rs:13`), so the workspace's `private_interfaces = "deny"` still holds.

`crates/ryo/crates/sdk/api.txt` is unchanged.

## 2 Compiler over tests: PASS

The seal is a compiler guarantee: no crate outside `execution_domain` can write `impl PatternSemantics for X`. No test stands in for it.

## 3 Hot path: PASS

`10c561ef6a` adds no runtime code. The seal is a marker supertrait with no methods, so dispatch through `&dyn PatternSemantics` is unchanged.

## 4 One representation per concept: PASS

- The patch points each ryo crate at its one source in this repository, instead of a published copy that had drifted.
- The 18 third-party crates the lock gains match ryo's lock exactly: pyproject-toml 0.13.7, pep440_rs 0.7.3, pep508_rs 0.9.1, thiserror 1.0.69 and 2.0.20, boxcar 0.2.14, toml 0.9.12 and 1.1.4, urlencoding 2.1.3, and the rest.

## 5 Standard crate over home-grown: PASS

No dependency was added. The seal uses the standard sealed-supertrait pattern.

## 6 Tests: PASS

**F2 is repaired.** `crates/ryo/crates/benchmark/src/inbound/use_cases/compare_corpus/fixtures.rs:170-173` now hands `ExecutionFake { status: 0, ..ExecutionFake::default() }`, and the logs above show the tests green.

**F3 is repaired.** `tm_import_cli` passes.

**F4 is repaired.** The six entries prove their tests, though the record calls only three of them compile proofs:
- **Behaviour red** (`gatefix_red_green_a_red.log`): `operation_schema::service_test::refuses` FAILED with `cycles_whose_source_and_destination_differ_in_kind_is_refused: expected ShapeMismatch, answered Ok(())`, and the same for the closure case. Its `base_sha` is `add03c25b3`, the parent of `09718aee51`. `a_green` is green.
- **Compile reds**, each green after restoring the production code:
  - `b_red_language`: E0432 on `holding`;
  - `b_red_python`: E0432 on `ancestors` and `operand_place`, covering the `comparisons_test` and `nested_test` entries;
  - `c_red_python`: E0432 on `enclosing` and `project`, covering the two "at 616dbb9433 tests" entries.

The record says outright that these count tests can only go red by failing to compile. That is the proof form the 332 record already accepts for `serve_project` and `refuse_spans_alone`, so it satisfies the red-then-green rule. Advisory B5 covers what it leaves unproven.

**Did the repairs break anything?**
- No `impl PatternSemantics` exists outside the bridge, so the seal breaks no fake.
- `tm_import_cli` compiled every ryo dependent against the sealed trait.
- The doctest build compiled `execution_domain` and `execution_store` cleanly.
- None of the warnings in the gatefix logs points at a line `10c561ef6a` added. The `execution_store owner/setup.rs` warnings are at lines 13, 35, 38, 39 and 47, all older.

## 7 Rust fit: PASS

**The `#[expect(unnameable_types)]` on the seal** (`matching/types.rs:15-19`) is correct, for three reasons:
- `crates/ryo/Cargo.toml:84` denies `unnameable_types`. A sealed supertrait is unnameable outside its crate by construction, so the lint must fire, and the seal cannot exist without answering it.
- `#[expect]`, unlike `#[allow]`, turns into an `unfulfilled_lint_expectations` warning if the trait ever becomes nameable, for example if `mod sealed` is made `pub` and the seal opened. No gatefix log reports an unfulfilled expectation.
- The reason it states is the real one.

One wording nit on the trait's doc is advisory B4.

The repairs add no `unsafe`, no `#[allow]`, and no `unwrap` or `expect` in production code. The doctests use `expect` and `?` as examples do.

### The warnings seen in the tastemaker build

They are ryo's existing warning debt. They also appear in ryo's own build, and they are not a gate item.

- **They are in ryo's own build.** `gatefix_doctests.log` builds in ryo's workspace and reports the same lint classes (unnecessary qualification, missing docs): language 57, values 76, operators 169, execution_domain 116, execution_store 146 warnings. The integration record's full workspace check counts 4,406 warnings at 332, down from 4,460 at 331.
- **Why they surfaced only now.** Both workspaces set `missing_docs` and `unused_qualifications` to `warn` (`crates/ryo/Cargo.toml:101,103`, `crates/tastemaker/Cargo.toml:75,77`). Cargo resolves a patched path crate's `lints.workspace = true` against that crate's own workspace, so the levels are ryo's. Registry dependencies are built with `--cap-lints`; path dependencies are not. Before the patch, the ryo crates came from crates.io and their warnings were hidden. Now they come by path and show.
- **Why the per-crate counts differ** (core 36 in the tastemaker build against 43 in ryo's): the two workspaces unify features differently, so `cfg` code differs.
- **Why they don't block the gate.** None is on a line this diff added. Item 7 concerns `#[allow]` without a reason and panics in production, not existing warn-level lint debt. They do matter to the project's zero-warning aim and to anyone building Tastemaker, whose own warnings now drown in ryo's (advisory B2).

## Advisories

No bead ids here. Each advisory becomes a bead, or is dropped with a reason.

- **B1. The tastemaker-core manifest understates the SDK it needs.** `crates/tastemaker/crates/core/Cargo.toml:33` still says `version = "0.1.1"`, but the adapter needs 0.1.2's fields. The patch hides this. Published as is, a consumer resolving 0.1.1 fails to compile. Fix: require `version = "0.1.2"`. The patch still resolves it here, and the manifest then says what the code needs.
- **B2. ryo's warnings are now visible in Tastemaker.** Through the path patch, about 900 warning lines in `gatefix_tm_import_cli.log` come from ryo crates. Fix: pay down the debt in ryo; the record already counts it. Alternatively, until the SDK is published, have the Tastemaker gate count only warnings from its own crates.
- **B3. The `serve_project` example is built from test fixtures.** It uses `owner::fixtures::prepared_sources` and `file_dag::fixtures::program_derived_port`, which exist only under the `fixtures` feature, a "test support, not a public API" switch (`execution_store/Cargo.toml:40`). On docs.rs the example's items do not exist. Fix: build the example from the public constructor path, or mark it `no_run` with a note that it needs `fixtures`.
- **B4. The `Sealed` doc claims more than the seal does.** It says "no crate outside this one adds a second matching semantics". But the blanket impl covers any type that implements the public, open `MatchingUseCase`, so an outside crate gets a second semantics by implementing that trait. The trait's own doc, "any matching use case answers it through this crate's blanket bridge", is accurate. Fix: reword the `Sealed` doc the same way.
- **B5. The count tests were never shown failing against the old scan.** Every count test calls an item its fix added, so the bound was never seen to fail against a scan. This extends the first pass's A9. Fix: add a negative-control row that runs a linear scan through the same counting closure and asserts that it exceeds the bound.
- **B6. Most Tastemaker tests have not run under the patch.** The patch moves the whole Tastemaker workspace from ryo 0.1.1 to 0.1.2, but only `tm_import_cli` ran under it (1 of 39 replay tests; its other 38 were filtered out). This is Tastemaker-crate territory, so it can go with the tm 0.1.2 branch. Fix: run `cargo test --manifest-path crates/tastemaker/Cargo.toml` once on this head.
- **B7. The first-pass advisories are tracked, not fixed.** A1 to A9 are carried in the record as their own items. The record says their beads are filed and keeps the ids out of shipped text.
