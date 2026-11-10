# TM steward, checkpoint 333 (run late, for the push of 334)
Source 40e49021520d4392d29df28ee5bd2fef739e5d37 (the 333 landed head), previous dcea2ee7df7c02711f42c332cbc403ff7b84a034 (332). Verdict **FAIL**. Reviewer tm-steward, 2026-11-10T07:37:00Z. This replaces the 332 steward's file at this path, as the push of 334 reads tm-steward.json for the previous landed head; the 334 pass stays at tm-steward-334.json.
The gate at the 333 head 40e4902152 has reds the 333 record does not own: the clippy group and correctness-python-ruff could not start (their run lines call python, which this container lacks), test-language/javascript and test-language/typescript fail manifest_diagnostic item_test, and test-language/python's allocation runner panics with no owner row. Each is fixed or owned at 334: ba6a6e35d0 (python3 run lines), d0ddb8914a (the manifest_diagnostic expectation), dee086491c (the python allocation case owned), with the 334 steward's gate PASS in tm-steward-334.json. The owned reds (core allocation, the two replay behaviour rows, the three cost ratchets) are as the record lists. Every one of the 71 enforcement candidate ids the push of 334 checks is decided, as tm-steward-334.json decided them: 69 none with reasons, 2 strengthen whose plants were rerun here (internal-package-allowlist 1, library-surface 1). No asset is authored; 27 rules, 11 specs, 27 templates, 185 placements before and after; branch null.
## Checks
- **gate** (FAIL): Run in crates/ryo of a detached worktree at 40e4902152 created for this pass and removed afterwards: PATH=<scratch>/bin:$PATH CARGO_TARGET_DIR=<the worktree's own target dir> flock <the build lock> asymptote check --groups lint,check,clippy --format json, tm 0.1.2 (sha256 beddba2a, the integrator's debug build for checkpoint 334) first on PATH, asymptote 0.1.1. Exit 2, 15:20 wall, 4,425,276 KB peak, no tool reused; summary new 15, resolved 174, accepted 4, unjudged 0, all 193 of them in-source ignores with reasons shown against the lock (0 unsuppressed findings). lint: cargo-fmt, tastemaker, cargo-machete, cargo-deny pass. clippy: cargo-clippy exit 127, 'bash: python: command not found' in its run line, so the group checked nothing (fixed at 334 by ba6a6e35d0). check: test-bdd, test-doc, cargo-doc, test-language css/go/html/json/markdown/rust/toml pass; test-core exit 104, the core allocation list panics (1672 to 1674 blocks), owned by core_allocation in the 333 record; test-cli 119 passed / 5 failed, replay behavior ruff_findings_equal_the_expected_findings and every_recorded_rule_gives_the_recorded_rows (owned, CLOSE) and cost a_cold_run_over_ruff_holds_the_peak_ratchet, every_live_rule_together_holds_the_cold_ratchet_of_ruff, a_warm_run_and_an_edit_run_over_ruff_hold_their_ratchets (owned, cli_cost), 74 not run behind fail-fast; test-language/javascript and test-language/typescript 1 red each, manifest_diagnostic item_test::succeeds, not in the 333 record (fixed at 334 by d0ddb8914a); test-language/python exit 104, the allocation runner panics (relations/annotates/input.py allocated 40 blocks in 20 files beyond the library's 1500), not owned at 333 (owned at 334 by dee086491c); correctness-python-ruff unstartable, 'python: No such file or directory' (fixed at 334 by ba6a6e35d0; at 334 it runs and fails an owned row).
- **candidates** (PASS): 71 unique enforcement candidate ids (72 entries; tracker-id-definition-split-78f9a5a1 is in both leak-review.json and branches/claude-w15-loop.json), computed as guard_shell.py candidate_problem does for the push of 334: leak-review.json (333, source 40e4902152, 3 candidates) plus the branch JSON files added or modified in 40e4902152..f953078b4b (17 files with candidates). Every id is decided: 69 none with reasons, 2 strengthen with plant_counts [1] matching one plant each. Each decision equals the one tm-steward-334.json made for the same set; the two strengthen plants were rerun here. candidate_problem run against a scratch commit carrying this file: candidate_problem returns None, review_shape (service.py shape) returns None and stands_for(source, 40e4902152) is True (guard_shell.py loaded from the 334 integration tree, its git show of tm-steward.json at f953078b4b replaced by this file)
- **plants** (PASS): One tm check --format sarif crates/sdk/Cargo.toml crates/execution/tests/publication/store/test.rs in crates/ryo of a git archive of f953078b4b under the scratch directory (tm 0.1.2 sha256 beddba2a, 188.70 s wall, 4,322,420 KB peak, exit 1): idiomatic.arch/internal-package-allowlist 1 at sdk/Cargo.toml (plus no-language-names 2 and crate-dependency-cycles 1 on the same plant), idiomatic.arch/library-surface 1 and test-imports 1 at execution/tests/publication/store/test.rs:37. The archive copy was deleted after the run.
- **bloat-budget** (PASS): crates/ryo/tastemaker.toml at 40e4902152: 27 rules, 11 specs, 27 templates, 185 placements (51 rule, 39 spec, 95 template entries); this pass authors no asset, so after equals before. The 334 tree holds 191 placements (53/39/99): library-surface split into scoped placements and four application-part template placements, authored by 334's builders and reviewed in 334 (tm-steward-334.json bloat-budget), not by this steward.
## Gate reds against their owners

| tool | red | owner at 333 | at 334 |
|---|---|---|---|
| cargo-clippy | exit 127, `python: command not found` in the run line; the group checked nothing | none | fixed, ba6a6e35d0 (python3 run lines); 334 clippy 0 new |
| test-core | core allocation list panics (1672 to 1674 blocks) | core_allocation, "Measure full-Django cold/warm/edit and Rust regressions; ..." | owned |
| test-cli | replay behavior ruff_findings_equal_the_expected_findings, every_recorded_rule_gives_the_recorded_rows | CLOSE | owned |
| test-cli | replay cost cold peak, warm/edit, every live rule cold ratchets | cli_cost | owned |
| test-language/javascript, typescript | manifest_diagnostic item_test::succeeds | none | fixed, d0ddb8914a |
| test-language/python | allocation runner panics (40 blocks in 20 files beyond 1500) | none | owned, dee086491c (RED_OWNERS default_features row) |
| correctness-python-ruff | unstartable, `python: No such file or directory` | none | fixed, ba6a6e35d0; at 334 it runs and fails an owned row |

Lint: cargo-fmt, tastemaker, cargo-machete, cargo-deny pass. Tastemaker's 15 new / 4 accepted / 174 resolved are in-source ignores with reasons shown against the lock; 0 unsuppressed findings.
## Per-class decisions

| class | decision | mechanism | proof count |
|---|---|---|---|
| checkpoint gate reds the 333 record does not own (clippy and correctness-python-ruff unstartable without python; javascript and typescript manifest_diagnostic item tests; python allocation runner) | d | none in Tastemaker; fixed or owned at 334: ba6a6e35d0 (asymptote.toml run lines call python3), d0ddb8914a (manifest_diagnostic item tests expect file 0), dee086491c (run.py RED_OWNERS owns the python allocation case); the runtime controls are the asymptote gate and run.py --check | 0 |
| cross-crate test or fixture naming another crate's internals by deep path (contract review 333 C1, C2) | b | idiomatic.arch/library-surface with the placement narrowed at 334 (f9a294ed54, merged 99482c61f8), plus compiler visibility sealed at 334 (audit F2, 47d437feaa) and the run.py --census cross-crate-test ratchet (51 at 334) | 1 |
| wildcard match hides a new variant (Rust gate 333 F1) | a | compiler: exhaustive match, repaired at 333 in 8bf8ae89a1 (failure() lists every Operation variant) | 0 |
| checkpoint gate drops a quality group (leak review 333 check 2a) | d | none; fixed at 334 in 81b6e551ec (the steward's gate runs clippy again) | 0 |
| tracker id definition split (leak review 333 check 2b) | d | none; one definition at 334: service.py tracker_patterns() reads no-tracker-text-shipped's patterns (62b71db6cd) | 0 |
| absence-gated test pass (leak review 333 check 2c) | d | none; fixed at 334 in afb0dbaa5d (the tests fail when their input is absent) | 0 |
| usize::MAX index sentinel (usize-max-index-sentinel, sentinel-index-hides-panic) | d | none now; bead: (a) From<Id> for usize plus usize::BITS assert, then (b) no-unwrap-or-on-lookup new version | 0 |
| sdk-api internal crate list hand-written | d | none in Tastemaker; sdk_api.py derives the set from cargo metadata (b6aff49ae3, 56169f083f) | 0 |
| sdk-api allowed impl keyed by type alone | d | none in Tastemaker; bead: key ALLOWED_IMPLS by trait and type in sdk_api.py | 0 |
| home-grown libtest argument protocol | d | none; one-off, samples_test.rs is the control | 0 |
| From<Id> for usize lossy below 32-bit usize | a | compiler: const _: () = assert!(usize::BITS >= 32) beside the impls | 0 |
| internal package missing from the inventory | b | idiomatic.arch/internal-package-allowlist with variables.internal-packages (extended at 334) | 1 |
| library-surface admits a concrete type's inherent items | b | idiomatic.arch/library-surface placement on crates/execution/tests (narrowed at 334) | 1 |
| linter package publishable | d | none; release publish closure plus internal-package-allowlist (plant 0 on benchmark-targets@0.2.0) | 0 |
| tracker-id spellings without a digit, short name in parentheses | d | none; a glob cannot tell a tracker id from a short name (plants 0 and 1) | 0 |
| compiler-held test and API shapes (dropped must_use, unconstructed variant, hand-listed catalogue, conditional failure) | a | compiler: unused_must_use, dead_code under narrower visibility, exhaustive match | 0 |
| test meaning (vacuous contract, constant for measured value, twin test, compile_fail probes, dead seam forward) | d | none; seam-mutant run and review | 0 |
| harness and hook classes (.claude/hooks, run.py, profile_proofs.py, commands.json, agent prompts) | d | none; each script's runtime check and its test file | 0 |
| proof binding (contract review early: byte identity, unbound runs, semver step measuring nothing, machine paths) | d | none in Tastemaker; run.py --check and the contract review | 0 |
| per-file template placements for one split test (four application-part entries) | d | advisory: one glob placement when next touched | 0 |
| sdk-api internal crates direct dependencies only | d | none in Tastemaker; sdk_api.py internal_crates resolve-graph walk and its test (56169f083f) | 0 |

What Tastemaker cannot prove here: deep-path imports from `src/**/*_test.rs` files and fixtures (contract review C1, C2) are outside the library-surface placement the plant exercised (execution/tests); visibility (audit F2) and the run.py --census ratchet are their controls. Harness, hook, registry and runtime-measurement classes are held by their own scripts and tests, which Tastemaker does not read.
## Candidate decisions

71 ids; each matches tm-steward-334.json's decision for the same set.

| id | decision | plant counts |
|---|---|---|
| checkpoint-gate-drops-a-quality-group-d347eb4d | none | [] |
| absence-gated-test-pass-080c5466 | none | [] |
| tracker-id-definition-split-78f9a5a1 | none | [] |
| internal-package-missing-from-the-inventory-8c9a41be | strengthen | [1] |
| linter-package-publishable-f479d638 | none | [] |
| stale-validation-command-package-58736d1c | none | [] |
| unconstructed-public-variant-58a9552d | none | [] |
| feature-gated-sdk-method-hands-out-an-internal-trait-1090d557 | none | [] |
| allowed-sdk-impl-keyed-by-its-target-type-alone-641c27f1 | none | [] |
| sdk-api-internal-crate-list-hand-written-8fa957e4 | none | [] |
| home-grown-libtest-argument-protocol-e9dbc575 | none | [] |
| sdk-api-internal-crates-direct-dependencies-only-8edfee08 | none | [] |
| dropped-must-use-result-f839521b | none | [] |
| sentinel-index-hides-panic-62cdc96e | none | [] |
| self-reexec-without-reentry-guard-4e52ebed | none | [] |
| usize-max-index-sentinel-0c59892e | none | [] |
| conditional-failure-declaration-counted-as-unconditional-a9f03941 | none | [] |
| fault-catalogue-audits-a-hand-listed-subset-of-variants-abc4eeee | none | [] |
| vacuous-port-contract-3390d83d | none | [] |
| compile-fail-doctest-several-probes-73e2218f | none | [] |
| constant-in-place-of-measured-value-9ffcaaea | none | [] |
| dead-seam-forward-de90151f | none | [] |
| twin-test-without-route-pin-22c0cc60 | none | [] |
| library-surface-admits-a-concrete-type-s-inherent-items-910da577 | strengthen | [1] |
| subagent-hook-reads-the-session-transcript-5e77b381 | none | [] |
| builder-brief-accepted-by-agent-type-b4b7a32a | none | [] |
| hook-crash-releases-the-stop-8bcd4be6 | none | [] |
| gated-launch-without-a-worktree-line-releases-the-stop-68f94134 | none | [] |
| acceptance-outlives-the-hook-timeout-07e399b3 | none | [] |
| stop-reads-only-the-first-brief-3fd912d9 | none | [] |
| beaded-brief-under-an-ungated-type-skips-every-gate-028cc8b1 | none | [] |
| integrator-stop-releases-a-beaded-brief-without-an-exact-worktree-line-3a4d8735 | none | [] |
| hook-argument-scan-past-double-dash-10512216 | none | [] |
| cargo-package-glob-accepted-3b04ff14 | none | [] |
| lock-substring-match-38bf0f42 | none | [] |
| cargo-verb-set-misses-aliases-c5f1d132 | none | [] |
| push-refspec-empty-source-skipped-675cdcce | none | [] |
| reviewer-verdict-second-validator-28e12b70 | none | [] |
| reviewer-hold-any-file-in-directory-bdd367cd | none | [] |
| diff-filter-skips-renames-aceba679 | none | [] |
| fail-open-on-missing-base-3183081a | none | [] |
| doc-exemption-by-name-only-c7d940e1 | none | [] |
| fix-entry-unbound-to-test-and-log-bdafa8b8 | none | [] |
| census-literal-name-regex-edc8de66 | none | [] |
| census-tests-rs-counted-production-ef46c4d0 | none | [] |
| shared-failure-counter-per-agent-type-252e8c5c | none | [] |
| wrapper-option-cluster-hides-command-c198ea5b | none | [] |
| cargo-behind-unrecognised-launcher-2ea2a219 | none | [] |
| push-destination-outside-argv-9151a27d | none | [] |
| reviewer-token-first-match-332f487a | none | [] |
| fix-exemption-by-commit-type-da27fb6c | none | [] |
| fix-scope-excludes-harness-code-db7cadbc | none | [] |
| fix-skips-merge-commits-8f027951 | none | [] |
| fix-log-provenance-unbound-d24d168c | none | [] |
| doc-examples-heading-only-074a596b | none | [] |
| census-test-split-by-text-position-2c355bb0 | none | [] |
| census-vocabulary-decoy-declaration-c9bc4e18 | none | [] |
| test-file-classifier-three-definitions-c7d88a5c | none | [] |
| flock-long-option-unlisted-4240df61 | none | [] |
| reviewer-token-ambiguous-a57ec567 | none | [] |
| perf-evidence-unbound-to-commit-e5d70ab6 | none | [] |
| census-cfg-test-field-extent-5c02f2ee | none | [] |
| census-cfg-test-angle-bracket-depth-334fc686 | none | [] |
| census-cfg-test-header-initializer-angle-c993d1af | none | [] |
| census-cfg-test-header-array-length-angle-1f312aff | none | [] |
| name-only-diff-hides-rename-source-25b627f8 | none | [] |
| hook-loader-exception-releases-hold-218b1b33 | none | [] |
| tracker-id-without-digit-unmatched-6c8faa27 | none | [] |
| tracker-id-spelling-unmatched-8d65fb3b | none | [] |
| short-name-in-parentheses-reported-as-tracker-id-f026c12e | none | [] |
| unsealed-combined-input-reference-dfaacc29 | none | [] |
## Asset counts

Before: 27 rules, 11 specs, 27 templates, 185 placements (crates/ryo/tastemaker.toml at 40e4902152). After: the same. No asset added, so no justification lines are owed. The PR freeze allows only (a), (b) or (d); no bead note is needed, since nothing sizeable is open beyond the beads tm-steward-334.json already names.
## Branch

None: no asset authored.
