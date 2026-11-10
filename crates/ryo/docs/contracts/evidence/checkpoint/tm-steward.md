# TM steward, checkpoint 334 (gate re-run at the final head)
Source 79c1329bdf3382a990054948b36b2f107e6e2e81 (final head of 334), previous 40e49021520d4392d29df28ee5bd2fef739e5d37. Verdict **PASS**. Reviewer tm-steward, 2026-11-10T07:03:04Z. This report replaces the pass at 333641a62a (verdict FAIL on the gate only).
The gate re-run at the final head 79c1329bdf, in the integration tree, has no unowned red: lint and clippy hold 0 new findings in tracked files (291 clippy::as_conversions accepted, the lock drained as 2eea0b8575 records; validation.json now 318 lines), and every red of check and api is an owned row: benchmark process_capture, the two replay behaviour reds and the three cost ratchets, the python allocation runner, the native-standalone pin of correctness-python-ruff, and exactly the 4 owned semver-checks lints of sdk-api. The one new Tastemaker error is in an untracked file that is not part of the source (contract-review-334-early.md, a machine path). 71 enforcement candidate ids are decided: the 70 of the earlier pass kept (one reason updated for the sdk_api.py fix) and the one the delta added (sdk-api-internal-crates-direct-dependencies-only-8edfee08, none: held by sdk_api.py's resolve-graph walk and its test, which Tastemaker cannot read). No asset authored, branch null, 27 rules, 11 specs, 27 templates, 191 placements before and after.
## Checks
- **gate** (PASS): Run in crates/ryo of the integration tree at HEAD 79c1329bdf (the 4 untracked *-334-early review files present): PATH=<scratch>/bin:$PATH CARGO_TARGET_DIR=<integration target dir> asymptote check --groups lint,check,clippy,api --format json, with tm 0.1.2 (sha256 beddba2a, the integrator's debug build for checkpoint 334) first on PATH; asymptote 0.1.1. Exit 2, 13:03.4 wall, 4,933,652 KB peak, no tool reused; summary new 2, resolved 1, accepted 318, unjudged 0, 29 suppressed. lint: cargo-fmt, cargo-machete, cargo-deny pass; tastemaker 1 new error, idiomatic.arch/no-machine-paths-shipped at docs/contracts/evidence/checkpoint/contract-review-334-early.md:12, a file git does not track at the source (so not a red of 79c1329bdf; it must not be committed as is). clippy: 0 new, 291 clippy::as_conversions accepted (the two of 333641a62a fixed by 91c711ca46). check: test-bdd, test-doc, cargo-doc and test-language css/go/html/javascript/json/markdown/rust/toml/typescript pass; test-core 1 red, ryo_benchmark verify_cases process_capture::test::succeeds (RED_OWNERS row, owner 'Close full TM, contracts, independent review and normal CI.'), 372 of 568 not run behind fail-fast; test-cli 5 reds, replay behavior ruff_findings_equal_the_expected_findings and every_recorded_rule_gives_the_recorded_rows (RED_OWNERS, CLOSE) and cost a_cold_run_over_ruff_holds_the_peak_ratchet, a_warm_run_and_an_edit_run_over_ruff_hold_their_ratchets, every_live_rule_together_holds_the_cold_ratchet_of_ruff (RED_OWNERS, the cli_cost owner), 88 of 199 not run behind fail-fast; test-language/python 1 red, allocation::python::reading_allocates_as_the_parser_library_does (RED_OWNERS default_features row, owner 'Measure full-Django cold/warm/edit and Rust regressions; meet final objective.'); correctness-python-ruff exit 2, verify_profiles.py 'native source profile identity mismatch' (the record's owned row: the stale native-standalone pin, owned by its tracker item and 'Repair every deferred output failure; rerun all 413 and full corpus.'). api: sdk-api exit 1 at cargo semver-checks (exit 100) after the leak check passed; rerun directly as uv run ../../ci/scripts/checks/sdk_api.py in the same tree (exit 1, 8.4 s) its report fails exactly constructible_struct_adds_field, enum_missing, enum_no_repr_variant_discriminant_changed, struct_missing: 4 major, 0 minor, the 4 RED_OWNERS sdk_api rows owned by 'The next SDK release is breaking (semver 0.x major, i.e. 0.2.0), owner-started via release-plz.'. No unowned red.
- **candidates** (PASS): 71 unique enforcement candidate ids decided: the 70 of the pass at 333641a62a (unchanged by the delta to 79c1329bdf except the reason of sdk-api-internal-crate-list-hand-written-8fa957e4, whose hand list b6aff49ae3 and 56169f083f replaced), plus sdk-api-internal-crates-direct-dependencies-only-8edfee08, which branches/claude-checkpoint-334.json gained at a33f361f7a: none, held by sdk_api.py's internal_crates walk of cargo metadata's resolve graph and its test test_a_type_a_member_forwards_through_another_members_reexport_is_internal_and_leaks (uv run --with pytest python -m pytest ci/scripts/checks/sdk_api_test.py: 17 passed). 2 strengthen with plant counts [1], the rest none with reasons.
- **plants** (PASS): One tm check --format sarif in crates/ryo of the archive copy with five plants, 140.4 s wall, 4,284,780 KB peak, exit 1, reverted after (git status clean): internal-package-allowlist 1 at sdk/Cargo.toml; library-surface 1 (and test-imports 1) at execution/tests/publication/store/test.rs:37; benchmark-targets 0 on publish = true in the ruff linter Cargo.toml; no-tracker-text-shipped 0 on the four no-digit spellings (sdk/src/lib.rs:23-26) and 1 on the short name in parentheses (lib.rs:27).
- **bloat-budget** (PASS): crates/ryo/tastemaker.toml at 79c1329bdf (unchanged since 333641a62a): 27 rules, 11 specs, 27 templates, 191 placements (53 rule, 39 spec, 99 template entries), the same before and after this pass: no asset added, branch null. The checkpoint itself grew 185 -> 191 (library-surface split into three scoped placements; four per-file application-part placements for the split cross_file service_test interpreter files, an advisory to merge into one glob placement when next touched).
## Gate reds against their owners

| tool | red | owner |
|---|---|---|
| tastemaker | no-machine-paths-shipped in contract-review-334-early.md:12 | none needed: the file is untracked, not part of the source; it must not be committed with the machine path |
| test-core | ryo_benchmark verify_cases process_capture::test::succeeds | RED_OWNERS, "Close full TM, contracts, independent review and normal CI." |
| test-cli | replay behavior ruff_findings_equal_the_expected_findings, every_recorded_rule_gives_the_recorded_rows | RED_OWNERS, "Close full TM, ..." |
| test-cli | replay cost cold peak, warm/edit, every live rule cold ratchets | RED_OWNERS, the cli_cost owner (Acceptance: 413/413 ...) |
| test-language/python | allocation::python::reading_allocates_as_the_parser_library_does | RED_OWNERS default_features row, "Measure full-Django cold/warm/edit and Rust regressions; ..." |
| correctness-python-ruff | verify_profiles.py: native source profile identity mismatch | the record's native-standalone pin row (its tracker item and "Repair every deferred output failure; ...") |
| sdk-api | semver-checks: constructible_struct_adds_field, enum_missing, enum_no_repr_variant_discriminant_changed, struct_missing | the 4 RED_OWNERS sdk_api rows, "The next SDK release is breaking (semver 0.x major, i.e. 0.2.0), ..." |

## Per-class decisions

| class | decision | mechanism | proof count |
|---|---|---|---|
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

What Tastemaker cannot prove, and the control it needs: test meaning (whether a contract compares an answer, whether a literal replaced a measured value) needs the seam-mutant run; a tracker id without a digit is the same text as a short product name, so only review or the tracker's own id list can tell them apart; the sdk-api checks read rustdoc JSON, which Tastemaker does not, so their crate list and impl allowance are fixed in sdk_api.py with test cases; a lossless id-to-index conversion is a Rust type question (From<Id> for usize plus a usize::BITS assertion).
## Asset counts

crates/ryo/tastemaker.toml is unchanged between 333641a62a and 79c1329bdf: 27 rules, 11 specs, 27 templates, 191 placements before and after. No asset added, so no justification lines. Branch: none.

## Beads proposed for after the PR freeze

1. Index conversions: replace the 280 `usize::try_from(<id>.0).unwrap_or(usize::MAX)` sites (131 files) and the bare `id.0 as usize` casts (two are this gate's new clippy findings) with the existing From<Id> for usize impls, add `const _: () = assert!(usize::BITS >= 32);` beside them, then give idiomatic.arch/no-unwrap-or-on-lookup a new version that refuses `unwrap_or(usize::MAX)`, proved by a plant.
2. sdk_api.py allowed_impl keys ALLOWED_IMPLS by trait and type, with a test planting `impl PartialEq<Language> for LanguageConnection`.

The former second item (the hand-written internal crate list) is done: b6aff49ae3 and 56169f083f derive the set from cargo metadata.

## Advisories

- The checkpoint grew crates/ryo/tastemaker.toml from 185 to 191 placements; four of them are per-file idiomatic.ryo/application-part entries for the split cross_file service_test interpreter files. One glob placement over that directory holds the same files; merge when next touched.
- The untracked contract-review-334-early.md names a machine path (no-machine-paths-shipped at line 12): fix it before it is committed, or leave it out.
- test-core and test-cli stop at the first failure (372 of 568 and 88 of 199 tests not run), so an owned red can hide an unowned one behind it in this gate; the record's validation run, which runs every test, is the evidence for those, not this gate.

## Candidate decisions

| id | decision | mechanism | plant counts |
|---|---|---|---|
| sdk-api-internal-crate-list-hand-written-8fa957e4 | none | none | [] |
| home-grown-libtest-argument-protocol-e9dbc575 | none | none | [] |
| internal-package-missing-from-the-inventory-8c9a41be | strengthen | idiomatic.arch/internal-package-allowlist with variables.internal-packages in crates/ryo/tastemaker.toml (inventory extended at 334) | [1] |
| linter-package-publishable-f479d638 | none | none | [] |
| stale-validation-command-package-58736d1c | none | none | [] |
| unconstructed-public-variant-58a9552d | none | none | [] |
| feature-gated-sdk-method-hands-out-an-internal-trait-1090d557 | none | none | [] |
| allowed-sdk-impl-keyed-by-its-target-type-alone-641c27f1 | none | none | [] |
| dropped-must-use-result-f839521b | none | none | [] |
| sentinel-index-hides-panic-62cdc96e | none | none | [] |
| self-reexec-without-reentry-guard-4e52ebed | none | none | [] |
| usize-max-index-sentinel-0c59892e | none | none | [] |
| conditional-failure-declaration-counted-as-unconditional-a9f03941 | none | none | [] |
| fault-catalogue-audits-a-hand-listed-subset-of-variants-abc4eeee | none | none | [] |
| vacuous-port-contract-3390d83d | none | none | [] |
| compile-fail-doctest-several-probes-73e2218f | none | none | [] |
| constant-in-place-of-measured-value-9ffcaaea | none | none | [] |
| dead-seam-forward-de90151f | none | none | [] |
| twin-test-without-route-pin-22c0cc60 | none | none | [] |
| library-surface-admits-a-concrete-type-s-inherent-items-910da577 | strengthen | idiomatic.arch/library-surface placement on crates/execution/tests/**/*.rs (crates/ryo/tastemaker.toml, params narrowed at 334) | [1] |
| subagent-hook-reads-the-session-transcript-5e77b381 | none | none | [] |
| builder-brief-accepted-by-agent-type-b4b7a32a | none | none | [] |
| hook-crash-releases-the-stop-8bcd4be6 | none | none | [] |
| gated-launch-without-a-worktree-line-releases-the-stop-68f94134 | none | none | [] |
| acceptance-outlives-the-hook-timeout-07e399b3 | none | none | [] |
| stop-reads-only-the-first-brief-3fd912d9 | none | none | [] |
| beaded-brief-under-an-ungated-type-skips-every-gate-028cc8b1 | none | none | [] |
| integrator-stop-releases-a-beaded-brief-without-an-exact-worktree-line-3a4d8735 | none | none | [] |
| hook-argument-scan-past-double-dash-10512216 | none | none | [] |
| cargo-package-glob-accepted-3b04ff14 | none | none | [] |
| lock-substring-match-38bf0f42 | none | none | [] |
| cargo-verb-set-misses-aliases-c5f1d132 | none | none | [] |
| push-refspec-empty-source-skipped-675cdcce | none | none | [] |
| reviewer-verdict-second-validator-28e12b70 | none | none | [] |
| reviewer-hold-any-file-in-directory-bdd367cd | none | none | [] |
| diff-filter-skips-renames-aceba679 | none | none | [] |
| fail-open-on-missing-base-3183081a | none | none | [] |
| doc-exemption-by-name-only-c7d940e1 | none | none | [] |
| fix-entry-unbound-to-test-and-log-bdafa8b8 | none | none | [] |
| census-literal-name-regex-edc8de66 | none | none | [] |
| census-tests-rs-counted-production-ef46c4d0 | none | none | [] |
| shared-failure-counter-per-agent-type-252e8c5c | none | none | [] |
| wrapper-option-cluster-hides-command-c198ea5b | none | none | [] |
| cargo-behind-unrecognised-launcher-2ea2a219 | none | none | [] |
| push-destination-outside-argv-9151a27d | none | none | [] |
| reviewer-token-first-match-332f487a | none | none | [] |
| fix-exemption-by-commit-type-da27fb6c | none | none | [] |
| fix-scope-excludes-harness-code-db7cadbc | none | none | [] |
| fix-skips-merge-commits-8f027951 | none | none | [] |
| fix-log-provenance-unbound-d24d168c | none | none | [] |
| doc-examples-heading-only-074a596b | none | none | [] |
| census-test-split-by-text-position-2c355bb0 | none | none | [] |
| census-vocabulary-decoy-declaration-c9bc4e18 | none | none | [] |
| test-file-classifier-three-definitions-c7d88a5c | none | none | [] |
| flock-long-option-unlisted-4240df61 | none | none | [] |
| reviewer-token-ambiguous-a57ec567 | none | none | [] |
| perf-evidence-unbound-to-commit-e5d70ab6 | none | none | [] |
| census-cfg-test-field-extent-5c02f2ee | none | none | [] |
| census-cfg-test-angle-bracket-depth-334fc686 | none | none | [] |
| census-cfg-test-header-initializer-angle-c993d1af | none | none | [] |
| census-cfg-test-header-array-length-angle-1f312aff | none | none | [] |
| name-only-diff-hides-rename-source-25b627f8 | none | none | [] |
| hook-loader-exception-releases-hold-218b1b33 | none | none | [] |
| tracker-id-definition-split-78f9a5a1 | none | none | [] |
| tracker-id-without-digit-unmatched-6c8faa27 | none | none | [] |
| tracker-id-spelling-unmatched-8d65fb3b | none | none | [] |
| short-name-in-parentheses-reported-as-tracker-id-f026c12e | none | none | [] |
| unsealed-combined-input-reference-dfaacc29 | none | none | [] |
| checkpoint-gate-drops-a-quality-group-d347eb4d | none | none | [] |
| absence-gated-test-pass-080c5466 | none | none | [] |
| sdk-api-internal-crates-direct-dependencies-only-8edfee08 | none | none | [] |

The reason of each decision is in tm-steward.json.
