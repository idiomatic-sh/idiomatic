# Leak review, checkpoint 332

Landed sha `dcea2ee7df7c02711f42c332cbc403ff7b84a034`, base `b1e81f7242`. Scope: the integrator's own commits beyond the branch heads already covered by `evidence/branches/`: the replay harness repairs (4b9580619f, b18f6b1535, 7fd2dbf1f5, 1101b931f2), the file splits (e0ad1b1895, 930fc65c86, 5b9555b1a0), the file-length ignores (802aea996c), the Rust gate repairs (10c561ef6a, 5e7ca6fb96, beb55b206a), the provider-imports allow (883f0d6dd6), the memory guard (96b5545cb8, 1123506752) and the run.py changes (fbebd781e6, e0ad1b1895, 2991a41cd2). The closure covers the crates they touch and their importers: cli, core, execution_domain, execution_store, execution_runtime, query_language, operators, benchmark, the python and rust languages, and tastemaker core and cli.

Before and after: `git rev-parse HEAD` = dcea2ee7df7c02711f42c332cbc403ff7b84a034, and `git status --porcelain` showed the same 6 untracked reviewer drafts under `evidence/branches/` at both points. Plants ran in a `git archive` copy with the prebuilt tm 0.1.2 and a check target of its own, and both were deleted when the review ended.

## Check 1: proof where the contract lives: PASS
- The replay harness repairs stay in `crates/ryo/cli/tests/fixtures/replay/base.rs` and `counting.rs`.
- The edited recordings now equal the real rule assets: `recordings/crate-dependency-depth/query.ryoql:3` matches `tastemaker/rules/crate-dependency-depth/0.1.0/rule.toml:8`, and `recordings/layer-order/query.ryoql:1` matches `tastemaker/rules/layer-order/0.2.0/rule.toml:5`.
- The memory guard is a tm claim, in `crates/tastemaker/cli/tests/replay/check.rs:360-376`.
- The moved cases stay in their own crates (core `lint/fixtures/stores.rs:300`, runtime `project_phase/service_test.rs`).

## Check 2: every enforcement claim fires: FAIL
Plants (all reverted):
| Claim | Planted at | Count |
| --- | --- | --- |
| `idiomatic.ryo/provider-imports` | python `fixtures/module_identity_only/mod.rs:15` (`use std::fs::read_to_string`) | 1 (`std::sync::atomic` at :14 gives 0, as allowed) |
| `idiomatic.arch/file-length` | core `lint/fixtures/lookups.rs` padded to 428 lines | 1 |
| `#[expect(unnameable_types)]` on the PatternSemantics seal | `execution_domain/src/domain/matching/types.rs:14` made `pub mod sealed` | 1 (lint expectation unfulfilled, at :18) |
| run.py `own()` reports a red without an owner | a red `manifests_go::…` added to `cli_replay_reds` | **0**: `--check` exits 0 after the note rewrite |
| the same, control | an unowned red name | 1 (`has no owner`, exit 1) |

**Finding 2a: stale owners pre-own reds** (`run.py:245-283`). fbebd781e6 added 28 OWNERS entries, most pointing at the catch-all row "Close full TM, contracts, independent review and normal CI." Every one of them except `lowering::every_rule_gives_the_same_rows_on_two_runs_over_ruff` (:273, not in the recorded logs) is green at this sha: the 26 CLOSE tests at :257-272 and :274-283, and `tm_import_cli` (:255, exit 0). The same holds for `core_library` (:246, failed 0), `behavior::ruff_rules_feed_every_catalog_feature_a_thousand_rows` (:250) and `cache::the_store_every_recorded_rule_keeps_over_ruff_stays_within_its_bound` (:253). If any of them turns red, `own()` (:300-312) files it under that row, and `--check` passes it. The plant shows this.
Required change: `own()` reports every OWNERS key whose result or test is not red as a problem (a stale owner), and the stale entries are deleted. A red then needs an owner chosen at the time it turns red.

**Finding 2b: a pending recording the harness can run is skipped.** b18f6b1535 links the TOML language into every replay lint (`base.rs:401`, `base.rs:475-482`, `counting.rs:177-181`). `provided()` (`base.rs:335-347`) still asks only the Rust language. `recording_at` (`base.rs:281-290`) requires `pending.txt` to name exactly the relations that set lacks. `recordings/dependency-feature-sprawl/pending.txt` names `entry`, `mapping` and `sequence-element`, and the TOML language provides all three (`languages/toml/src/domain/relations/collections/relations/{entry,mapping,sequence_element}/item.rs`). So the recording is never compared (`answers.rs:100,132,337` filter `Pending::Missing`), and the exactness assert stops anyone from removing its `pending.txt`.
Every variant: `no-json-in-process/pending.txt` (`tastemaker.placed`, a host relation) is legitimate. No other `pending.txt` exists.
Required change: one function in the replay fixtures returns the replay's language connections, and `provided()`, `lint_report_hosted` and `lint_observing` all use it. Delete `dependency-feature-sprawl/pending.txt`, and its recorded rows are compared.

The other harness changes leave no pass path:
- `innermost` unwraps only `Located`; any other refusal still panics (`base.rs:292`).
- `ABSENT` makes the tool side stricter.
- The `TextSets` arm applies only to arrays of string arrays; anything else stays unbound and fails the bind assert.
- The memory guard refuses exit 2, an unreadable report and an empty report. It is red at 4,452,912 KiB.
- The `compare_corpus` fixture with `status: 0` keeps its scenario's assertions.

## Check 3: boundaries hold across crates: FAIL
**Finding 3: tastemaker builds against unreleased ryo under a released version.** `crates/tastemaker/Cargo.toml:236-253` (10c561ef6a) patches 15 ryo crates to `../ryo` paths. Tastemaker still requires `idiomatic-sh-ryo-sdk` `0.1.2` (`crates/tastemaker/crates/core/Cargo.toml:33`, `cli/Cargo.toml:35`; the language crates at `cli/Cargo.toml:37-46, 63-66`).
- 0.1.2 is released (tag `idiomatic-sh-ryo-v0.1.2`). It lacks the seven request fields the ryo adapter fills, which e1dfa19a93's own message states.
- `crates/ryo/Cargo.toml:23` still says 0.1.2, so one version names two APIs.
- A build without the patch would resolve the published 0.1.2 and fail to compile.

Required change: give the unreleased ryo crates a version that is not published (bump `crates/ryo/Cargo.toml:23`) and require that version in tastemaker. Then the patch resolves only unreleased source, and a registry build fails to resolve instead of compiling against the wrong API. The alternative is to drop the patch until the SDK release that carries the fields.

Advisory: 883f0d6dd6 allows `std::sync::atomic::*` for `languages/{lang}/src` (`crates/ryo/tastemaker.toml:205`), but every provider use of it is test-only:
- `ruff_python_parser/adapter.rs:26-31` (`#[cfg(test)]`)
- `parse/contract.rs:45-46` (`#[cfg(test)]`)
- the `*_test.rs` files
- `fixtures/module_identity_only/mod.rs:14`

So the production allowlist was widened for tests.

Holding: `prepared_sources` is `pub` only under `cfg(any(test, feature = "fixtures"))` (execution_store `derived/owner/mod.rs:252-253`), and the seal fires when it is made nameable (check 2).

## Check 4: one implementation and one representation per concept: PASS
- The parity per-rule document has one writer (bmf `schemas.rs:345` `parity_rules`), and `verify_profiles.py` holds no copy.
- certify's `Observations` wraps the one `ExecutionFake` (`certify/fixtures/observations.rs:14-17`).
- `ContextProbe` was moved, not copied (`module_identity_only/mod.rs:69`).
- The span refusals moved to `query_language/src/domain/refusing/service.rs:17,89`, and `assembling/service.rs:121-122` is their one caller.
- The `lookups.rs` case moved whole to `stores.rs:300`.

Advisory: `compiled_lint` (`stores.rs:366-390`) repeats the real-compiler wiring of `runs.rs:46-59` and `lookups.rs:196-208`. That is composition, which is allowed.

## Check 5: layer direction: PASS
- `refusing` is a domain concept, called by domain `assembling`.
- The `Sealed` blanket impl is in `execution_domain/src/application/matching/service.rs:13`, over the domain trait (`domain/matching/types.rs:14-22`).
- `import_rows` (tastemaker core `domain/import_rows/service.rs:104-113, 216-232`) stays in its domain.
- No inbound or application item crosses a crate.

## Check 6: test seams stay out of production: PASS
- No `[dependencies]` entry or default feature in any committed `Cargo.toml` enables a fixtures or test feature (manifest scan).
- The `serve_project` doctest's fixtures are behind `cfg(any(test, feature = "fixtures"))`, switched on only by the crate's own dev-dependency (`execution_store/Cargo.toml:27`).
- The parse counters are `#[cfg(test)]` fields.

## Enforcement candidates
- `stale-red-owner-5fdfc825` (new rule)
- `cross-product-path-patch-a711e823` (new rule)
- `replay-language-set-split-e2460590` (compiler: one function for the language set)
- `test-only-import-in-production-allowlist-77f00ace` (strengthen `idiomatic.ryo/provider-imports`)

VERDICT: FAIL: 2, 3
