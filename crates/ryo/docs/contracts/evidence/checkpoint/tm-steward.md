# TM steward, checkpoint 334

Source 333641a62a01e87ce8bab84f94a43a710f813b9a (landed 334), previous 40e49021520d4392d29df28ee5bd2fef739e5d37. Verdict **FAIL**. Reviewer tm-steward, 2026-11-10T06:42:00Z.

The gate fails at the landed 333641a62a: lint has 1 new Tastemaker error (file-length on checkpoint/validation.json, 790 lines), clippy has 2 new as_conversions whose fingerprints moved, check has the owned reds (benchmark process_capture, the replay and cost reds, the python allocation list, correctness-python-ruff), and api fails at sdk_api.py's semver-checks step (exit 100). All 70 enforcement candidate ids the push of 334 requires are decided: 2 strengthen, each held by an existing asset whose placement or inventory was strengthened at 334 and whose plant fires 1 (internal-package-allowlist, library-surface); 68 none with reasons. The open Rust classes the PM named (usize::MAX index sentinel, 280 sites; the hand-written sdk-api internal crate list; the ALLOWED_IMPLS key) are deferred to beads after the PR freeze, compiler or script fix first. No asset authored, branch null, counts 27 rules, 11 specs, 27 templates, 191 placements before and after.

## Checks

- **gate** (FAIL): PATH=<scratch>/bin:$PATH CARGO_TARGET_DIR=<scratch>/target asymptote check --groups lint,check,clippy,api --format json, run in crates/ryo of a git archive copy of 333641a62a (git init and one commit so Tastemaker reads it, the integration tree's .asymptote/cache copied in; tm 0.1.2 sha256 8cddc4c1, the build the record names from 8568d6f1a2, first on PATH because the root tastemaker.toml needs 0.1.2 and /usr/local/cargo/bin/tm is 0.1.0; asymptote 0.1.1). Exit 2, 15:08.7 wall, no tool reused; summary new 4, resolved 3, accepted 318, 29 suppressed. lint FAIL: cargo-fmt, cargo-machete, cargo-deny pass; tastemaker 1 new error, idiomatic.arch/file-length on docs/contracts/evidence/checkpoint/validation.json (790 lines at the landed sha, which its last commit grew). clippy FAIL: 2 new clippy::as_conversions, each replacing a resolved one in the same file (execution use_case_test/interpreter/conformance_scale/test.rs:228 `emit.0 as usize`, execution tests/interpreter/project_phase/population_reads/test.rs:150 `table.count.0 as u64`), 291 accepted. check FAIL: test-bdd, test-doc, cargo-doc and test-language css/go/html/javascript/json/markdown/rust/toml/typescript pass; test-core red on benchmark verify_cases process_capture::succeeds (the red validation.json owns since validation326); test-cli 4 reds (replay behavior every_recorded_rule_gives_the_recorded_rows and ruff_findings_equal_the_expected_findings, cost cold peak and warm/edit ratchets, the replay and cost reds owned as at 333); test-language/python red on allocation reading_allocates_as_the_parser_library_does; correctness-python-ruff exit 2 (verify_profiles.py: native source profile identity mismatch, possibly the archive copy's path; not settled). api FAIL: in the first run sdk_api.py could not archive its baseline tag idiomatic-sh-ryo-v0.1.2 (the copy had no tags, an artifact of the copy); with that tag fetched into the copy a rerun of --groups api exits 2: cargo semver-checks against the baseline root exits 100, the traceback at sdk_api.py:221, its own report truncated in asymptote's said text. One attempt made; the probe that settles it is the same sdk_api.py run in the integration tree, which has the tag history.

- **candidates** (PASS): 70 unique enforcement candidate ids decided: the 2 of the checkpoint 334 leak review (branches/claude-checkpoint-334.json), the 3 of checkpoint/leak-review.json (333) and every candidate of the branch files added or modified between 40e4902152 and 333641a62a (the duplicate tracker-id-definition-split counted once). 2 strengthen, each plant firing 1 under tm check (tm 0.1.2 sha256 8cddc4c1) in a git archive copy of 333641a62a, reverted; the rest none with reasons.

- **plants** (PASS): One tm check --format sarif in crates/ryo of the archive copy with five plants, 140.4 s wall, 4,284,780 KB peak, exit 1, reverted after (git status clean): internal-package-allowlist 1 at sdk/Cargo.toml; library-surface 1 (and test-imports 1) at execution/tests/publication/store/test.rs:37; benchmark-targets 0 on publish = true in the ruff linter Cargo.toml; no-tracker-text-shipped 0 on the four no-digit spellings (sdk/src/lib.rs:23-26) and 1 on the short name in parentheses (lib.rs:27).

- **bloat-budget** (PASS): crates/ryo/tastemaker.toml at 333641a62a: 27 rules, 11 specs, 27 templates, 191 placements (53 rule, 39 spec, 99 template entries), the same before and after this pass: no asset added, branch null. The checkpoint itself grew 185 -> 191 (library-surface split into three scoped placements; four per-file application-part placements for the split cross_file service_test interpreter files, an advisory to merge into one glob placement when next touched).

## Per-class decisions

| class | decision | mechanism | proof count |
|---|---|---|---|
| usize::MAX index sentinel (usize-max-index-sentinel, sentinel-index-hides-panic) | d | none now; bead: (a) From<Id> for usize plus usize::BITS assert, then (b) no-unwrap-or-on-lookup new version | 0 |
| sdk-api internal crate list hand-written | d | none in Tastemaker; bead: sdk_api.py reads variables.internal-packages | 0 |
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

What Tastemaker cannot prove, and the control it needs: test meaning (whether a contract compares an answer, whether a literal replaced a measured value) needs the seam-mutant run; a tracker id without a digit is the same text as a short product name, so only review or the tracker's own id list can tell them apart; the sdk-api checks read rustdoc JSON, which Tastemaker does not, so their crate list and impl allowance are fixed in sdk_api.py with test cases; a lossless id-to-index conversion is a Rust type question (From<Id> for usize plus a usize::BITS assertion).

## Beads proposed for after the PR freeze

1. Index conversions: replace the 280 `usize::try_from(<id>.0).unwrap_or(usize::MAX)` sites (131 files) and the bare `id.0 as usize` casts (two are this gate's new clippy findings) with the existing From<Id> for usize impls, add `const _: () = assert!(usize::BITS >= 32);` beside them, then give idiomatic.arch/no-unwrap-or-on-lookup a new version that refuses `unwrap_or(usize::MAX)`, proved by a plant.
2. sdk_api.py reads the internal crate set from crates/ryo/tastemaker.toml variables.internal-packages (one definition, as service.py reads the tracker globs), with a test that a public SDK function returning an ryo_execution type is a leak.
3. sdk_api.py allowed_impl keys ALLOWED_IMPLS by trait and type, with a test planting `impl PartialEq<Language> for LanguageConnection`.

## Advisories

- The checkpoint grew crates/ryo/tastemaker.toml from 185 to 191 placements; four of them are per-file idiomatic.ryo/application-part entries for the split cross_file service_test interpreter files. One glob placement over that directory holds the same files; merge when next touched.
- validation.json is 790 lines at the landed sha and fails file-length; the record outgrew its file, so it needs splitting by checkpoint or a shorter entry shape, not a suppression.

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

The reason for every entry is in tm-steward-334.json. The 33 claude-w15-loop entries checkpoint 332 decided carry the 332 reason, which still holds at 333641a62a.

## Asset counts

| | rules | specs | templates | placements |
|---|---|---|---|---|
| before | 27 | 11 | 27 | 191 |
| after | 27 | 11 | 27 | 191 |

No asset added, so there are no justification lines.

## Branch

None. No asset was authored, so there is nothing for the next checkpoint's reviewers.
