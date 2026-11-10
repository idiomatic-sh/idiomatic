# TM steward, landed checkpoint 332 (decided for the landing of 333)

Landed sha `dcea2ee7df`, previous `b1e81f7242`. The gate ran at the 333 head `e4f1d396eb`, as the PM asked (asymptote.json is deleted at 333). Verdict: **FAIL**.

## Gate

- **gate** (FAIL): asymptote check --groups lint,check in crates/ryo at e4f1d396eb (tm 0.1.2 release build sha256 33bba487 first on PATH, CARGO_TARGET_DIR the 333 tree's own target, under the build lock): exit 2, 16 new / 174 resolved. Lint: cargo-fmt, cargo-machete and cargo-deny pass; tastemaker gives 1 new finding, idiomatic.arch/file-length on docs/contracts/evidence/branches/claude-w15-loop.json (708 lines at the head, 713 in the uncommitted PASS overwrite), and 15 in-source ignores shown against the lock. Check: test-core red (core allocation list panics: a file allocated more than one block beyond the parser library, owned as at 332); test-cli 119 passed / 5 failed (replay behavior::ruff_findings_equal_the_expected_findings and every_recorded_rule_gives_the_recorded_rows, cost cold peak, every-live-rule cold and warm/edit ratchets: the owned replay and cli_cost reds); test-language/python red (its allocation list panics: 40 blocks in 20 files beyond the library's 1500); test-language/javascript and test-language/typescript red on manifest_diagnostic item_test row a_manifest_that_is_not_json_is_a_row_of_the_parse_at_the_file_origin (expected a leading 1, answered 0), not among the reds the 333 record lists; correctness-python-ruff could not start (no python executable on PATH, only python3)
- **candidates** (PASS): the push hook's candidate_problem (guard_shell.py), run against e4f1d396eb with this file: every candidate id decided; plant_mismatch empty (no decision other than none)
- **hook-tracker-false-positive** (FAIL): the same call refuses before reading this file: leak-review.json candidate cross-product-path-patch-a711e823 'carries a tracker id' because service.py TRACKER (\bryo-[a-z0-9]{3,}) matches the published SDK and core package names (idiomatic-sh- then ryo- then the crate) inside its signature. With the lookbehind (?<![\w-]) the call returns None. The landing is blocked until the validator or that signature changes; the steward changes neither
- **bloat-budget** (PASS): crates/ryo/tastemaker.toml at e4f1d396eb: 27 rules, 11 specs, 27 templates, 185 placements (51 rule, 39 spec, 95 template entries) before and after; no asset added

## Candidate decisions

44 candidates: 4 from `checkpoint/leak-review.json` (332), 7 from `branches/claude-w15-ci.json`, 33 from `branches/claude-w15-loop.json` (including `census-cfg-test-header-array-length-angle-1f312aff` of its PASS at 794ec68025). `branches/claude-w14-isolation-lens.json` has none. Every candidate is decided `none`. The reason for each is in the JSON.

| id | decision |
| --- | --- |
| `stale-red-owner-5fdfc825` | none |
| `cross-product-path-patch-a711e823` | none |
| `replay-language-set-split-e2460590` | none |
| `test-only-import-in-production-allowlist-77f00ace` | none |
| `secret-allowlist-unscoped-by-path-2ff533be` | none |
| `lint-range-against-every-remote-3613f7e4` | none |
| `lint-range-fails-open-on-missing-ref-6b69463f` | none |
| `secret-allowlist-key-last-part-unrestricted-4be7069e` | none |
| `red-owned-by-coarse-key-9a5b6f2b` | none |
| `crashed-binary-hidden-behind-owned-red-ce18d982` | none |
| `red-owned-despite-truncated-log-1456be1a` | none |
| `hook-argument-scan-past-double-dash-10512216` | none |
| `cargo-package-glob-accepted-3b04ff14` | none |
| `lock-substring-match-38bf0f42` | none |
| `cargo-verb-set-misses-aliases-c5f1d132` | none |
| `push-refspec-empty-source-skipped-675cdcce` | none |
| `reviewer-verdict-second-validator-28e12b70` | none |
| `reviewer-hold-any-file-in-directory-bdd367cd` | none |
| `shared-failure-counter-per-agent-type-252e8c5c` | none |
| `reviewer-token-first-match-332f487a` | none |
| `flock-long-option-unlisted-4240df61` | none |
| `reviewer-token-ambiguous-a57ec567` | none |
| `wrapper-option-cluster-hides-command-c198ea5b` | none |
| `cargo-behind-unrecognised-launcher-2ea2a219` | none |
| `push-destination-outside-argv-9151a27d` | none |
| `diff-filter-skips-renames-aceba679` | none |
| `fail-open-on-missing-base-3183081a` | none |
| `doc-exemption-by-name-only-c7d940e1` | none |
| `fix-entry-unbound-to-test-and-log-bdafa8b8` | none |
| `fix-exemption-by-commit-type-da27fb6c` | none |
| `fix-scope-excludes-harness-code-db7cadbc` | none |
| `fix-skips-merge-commits-8f027951` | none |
| `fix-log-provenance-unbound-d24d168c` | none |
| `doc-examples-heading-only-074a596b` | none |
| `perf-evidence-unbound-to-commit-e5d70ab6` | none |
| `test-file-classifier-three-definitions-c7d88a5c` | none |
| `census-literal-name-regex-edc8de66` | none |
| `census-tests-rs-counted-production-ef46c4d0` | none |
| `census-test-split-by-text-position-2c355bb0` | none |
| `census-vocabulary-decoy-declaration-c9bc4e18` | none |
| `census-cfg-test-field-extent-5c02f2ee` | none |
| `census-cfg-test-angle-bracket-depth-334fc686` | none |
| `census-cfg-test-header-initializer-angle-c993d1af` | none |
| `census-cfg-test-header-array-length-angle-1f312aff` | none |

## Classes

| class | decision (a-d) | mechanism | proof count |
| --- | --- | --- | --- |
| stale red owner | d | run.py --check stale-owner check with its run_test.py case (runtime) | 0 |
| cross-product path patch | d | unpublished 0.1.3 version; cargo resolver and release package verification | 0 |
| replay language set split | d | one function replay_languages() in the replay fixtures | 0 |
| test-only import in production allowlist | d | Rust #[cfg(test)]; TM cannot see cfg | 0 |
| checkpoint harness (run.py) red ownership, fix and doc checks | d | run_test.py cases on the branch | 0 |
| census scanner edge cases | d | census replacement by ryo queries | 0 |
| hook command-line and reviewer-token parsing | d | test_hooks.py cases; hooks README Known non-goals | 0 |
| root tool configuration ranges and secret allowlist | d | fixed on claude/w15-ci; the lint group runs the tools | 0 |

What Tastemaker cannot prove here:
- Whether a dependency version is published. That is registry state; the control is cargo's resolver plus the release job's package verification.
- `#[cfg(test)]` reach. That is Rust's own control.
- A run's exit code or log truncation. That is a runtime check in `run.py`.
- Shell command-line spellings. That belongs to the hooks, under their threat model.

## Asset counts

`crates/ryo/tastemaker.toml`: 27 rules, 11 specs, 27 templates, 185 placements (51 rule, 39 spec, 95 template entries), the same before and after. No asset was added, so no justification lines are needed.

## Branch

None. No asset was authored, so `claude/tm-steward-333` was not created.
