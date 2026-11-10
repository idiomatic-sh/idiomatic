# Leak review: claude/prune at 8a43781af7b211283ada0b8c8d722da4bd7cabf4

This is a targeted re-check of the delta f5e6647370..8a43781af7 against the FAIL at f5e6647370 (checks 2 and 3, and the kept-without-reader, dead-code and docs-honesty items). The base of the branch is a6e880e1d7. The reviewed sha was read from git objects only. All builds, plants and self-tests ran in a git archive of 8a43781af7 at the review's archive copy, with its own target the review's own target directory under flock the shared build lock. Both were deleted at the end.

## 1. Proof where the contract lives: PASS

The delta f5e6647370..8a43781af7 deletes the --case-selection chain from the shared harness (crates/ryo/crates/benchmark/src/inbound/use_cases/select_cases/service.rs:20-38, crates/ryo/crates/benchmark_contracts/src/domain/requests/types.rs:147-150) and the complete-full-report mode from the bench (crates/ryo/languages/python/benchmarks/linters/ruff/authoring/profile_staging.py:74-176); it asserts no engine, core or SDK contract from benchmarks/ or languages/, and the only product-side Rust edit (crates/ryo/languages/python/benchmarks/linters/ruff/composition/commands.rs:142, :203) drops wiring.

## 2. Every enforcement claim fires: PASS

Plants in a git archive of 8a43781af7 with the profile_staging.py hash re-pinned in prepared.json: crates/ryo/languages/python/benchmarks/linters/ruff/authoring/profile_staging.py:117 (was :118) disabled -> verify_profiles.py --self-test exit 1, "shape missing-rule refusal did not fail before candidate work"; :140 (was :147) disabled -> exit 1, "shape oracle refusal did not fail before candidate work". Both are planted by guards_9 against shape-110 (crates/ryo/languages/python/benchmarks/linters/ruff/authoring/profile_guard_proof_tests.py:140-151, :165), which requires exit 2, the named message and no marker; it raises on an unknown kind (:150-151) and restores overlay and oracle bytes per kind (:166-172). The :131 count refusal is deleted with the mode. Unplanted --self-test exit 0, --self-test-native-standalone exit 0. Staleness confirmed: expanded-defaults benchmark_sha256 is 6aae2bb5 in prepared.json while its benchmark.toml hashes to 2755ee70 at both a6e880e1d7 and 8a43781af7, so crates/ryo/languages/python/benchmarks/linters/ruff/authoring/profile_staging.py:82-83 refuses it before any overlay logic. Rust: the case_selection test keeps its Succeeds rows with an Infallible failure (crates/ryo/crates/benchmark/src/application/case_selection/service_test.rs:69-89); the contract row that replaced WriteAndSelect reads back 13 real bytes through write_text and source_text (crates/ryo/crates/benchmark/src/outbound/suite/fixtures/contract.rs:169-173, :339-345). No skip, caught error or weakened assertion added.

## 3. Boundaries hold across crates: PASS

The flag without a producer is gone: git grep at 8a43781af7 finds no CaseId, IdSelection, SelectedIds, SelectedName, CASE_SELECTION, case-selection, SuitePort::selection or selection::schemas outside docs/contracts/evidence; --case-selection now falls to the unknown-flag refusal (crates/ryo/crates/benchmark/src/inbound/use_cases/select_cases/service.rs:32). CaseSelection keeps only shard (crates/ryo/crates/benchmark_contracts/src/domain/requests/types.rs:147-150). SelectCasesUseCaseImpl is a unit struct with pub const fn new() (crates/ryo/crates/benchmark/src/inbound/use_cases/select_cases/use_case.rs:9-15), no wider than before. The ruff bench package compiles against the narrowed API (nextest --no-run exit 0).

## 4. One implementation and one representation per concept: PASS

The delta adds no loop, type or data shape; it deletes IdSelection, CaseId, SelectedIds, SelectedName and the selection schema. Carried from the review of f5e6647370: The diff is 137 insertions and 450,576 deletions; it adds no second loop, type or data shape. Python edits remove branches (crates/ryo/languages/python/benchmarks/linters/ruff/authoring/profile_shards.py, verification_cli.py, profile_staging.py:74-190, rule_registration.py:41) and copy no shared benchmark capability into the product directory.

## 5. Layer direction: PASS

The delta now touches Rust. select_cases (inbound) no longer reaches into outbound::suite: SuitePort and ArtifactPath imports are removed (crates/ryo/crates/benchmark/src/inbound/use_cases/select_cases/service.rs:1-15, use_case.rs:1-8), and application/case_selection drops its ReportsError dependency (crates/ryo/crates/benchmark/src/application/case_selection/service.rs:1-27). No import is added in any direction.

## 6. Test seams stay out of production: PASS

No test seam is added. The bdd acceptance stops importing outbound::suite::fixtures::fake::SuiteFake (crates/ryo/crates/benchmark/src/inbound/use_cases/select_cases/acceptance.rs:1-17), and SuiteFake loses its selection forwarding (crates/ryo/crates/benchmark/src/outbound/suite/fixtures/fake.rs:132-135). No feature, cfg(test) bypass or pub helper is added.

## 16. Item 16: dependency and supply-chain hygiene: PASS

No manifest or lock change in f5e6647370..8a43781af7. Carried from the review of f5e6647370: No Cargo.toml, Cargo.lock, pyproject or tool manifest changes in a6e880e1d7..f5e6647370; no dependency added or removed.

## 17. Item 17: portable and reproducible: PASS

Reproduced from a git archive of 8a43781af7 with its own target the review's own target directory under flock: cargo nextest run -p idiomatic-sh-ryo-benchmark: 228 run, 227 passed, 1 failed, inbound::use_cases::verify_cases::use_case_test::process_capture::test::succeeds, a pre-existing owned red (RED_OWNERS row in crates/ryo/docs/contracts/evidence/checkpoint/tm-steward-334.md:14), whose code is untouched in a6e880e1d7..8a43781af7. By id, the only removed tests are application::case_selection::service_test::refuses and outbound::suite::adapters::local::selection::schemas::tests::{succeeds,refuses}; the contract rows WriteAndSelect/SelectMissing become WriteAndRead, and the select_cases rows are unchanged. verify_profiles.py --self-test and --self-test-native-standalone exit 0; verify_files: 1774 pins, 0 mismatches, set equals manifest_paths.

## 18. Item 18: use what everyone uses: PASS

No new tool, parser or format in the delta. Carried from the review of f5e6647370: No new parser, runner, format or checker; the change only deletes data and code paths.

## brief-deleted-data-readers. Brief: no remaining reader of deleted data: PASS

git grep at 8a43781af7 for every file the delta deletes (F821/cases-expanded.toml, the 10 source-handoffs files, EXE001_2.py, EXE001_3.py, three .snap files) finds no code reader. The remaining hits are the benchmark-linter spec placements with count min 0 (crates/ryo/tastemaker/specs/benchmark-linter/0.2.2/spec.toml:132-136, :569), other profiles' own cases-expanded.toml pinned in prepared.json, EXE001 cases.toml inline sources and the honest pruned note in crates/ryo/languages/python/benchmarks/linters/ruff/profiles/EXE001/evidence/upstream-identity.json:24.

## brief-kept-without-reader. Brief: no kept file without a reader: PASS

Every file the prior FAIL named is deleted at 8a43781af7. A scan of all 1153 files under crates/ryo/languages/python/benchmarks/linters/ruff/profiles and crates/ryo/languages/python/benchmarks/linters/ruff/results finds each named by path, basename or sha256 in another tracked file, except 5 top-level standard results (correctness.json, environment.txt, fixtures.json, profile.json, profile.txt), which the benchmark-linter spec places (crates/ryo/tastemaker/specs/benchmark-linter/0.2.2/spec.toml:623-628) and the prune did not touch.

## brief-status-guard. Brief: guard refuses non-archived combined profiles (P-T): PASS

profile_proofs.py is unchanged in the delta; the P-T plant result stands. Carried from the review of f5e6647370: crates/ryo/languages/python/benchmarks/linters/ruff/authoring/profile_proofs.py:135-136 refuses any combined descriptor whose validation_status is not archived-historical (absent key included); plant P-T fires (exit 1, guard message) per check 2.

## brief-dead-code. Brief: dead code left: PASS

The --case-selection chain is deleted end to end (see check 3). The complete-full-report branches are gone from crates/ryo/languages/python/benchmarks/linters/ruff/authoring/profile_staging.py. The worker docstring now reads "Reuse the frozen executable..." (crates/ryo/languages/python/benchmarks/linters/ruff/authoring/profile_worker.py:136). The Rust build of the benchmark crate and the ruff bench package compiles with no new unused-item warning on a delta line.

## brief-prepared-pins. Brief: prepared.json pins match the files: PASS

Independent run in the archive: profile_proofs.verify_files ok; 1774 pins, 0 sha256 mismatches, set(files) == manifest_paths; the three re-pinned authoring hashes match (prepared.json:11379-11387).

## brief-docs-honesty. Brief: doc links to deleted files handled honestly: PASS

A scan of every tracked .md at 8a43781af7 finds no markdown link into a path deleted in a6e880e1d7..8a43781af7. All 4 code-span references to pruned paths carry the pruned-after-a6e880e1d7 note: crates/ryo/languages/python/benchmarks/linters/ruff/results/report.md:181 and :246, crates/ryo/languages/python/benchmarks/linters/ruff/rules/RUF019/README.md:7 and crates/ryo/languages/python/benchmarks/linters/ruff/rules/RUF200/README.md:7. README.md:3 links the kept summary and states that the other receipts were pruned. EXE001/README.md:3 no longer claims that copies are retained.

## Plants

| Claim | File | Count | Reverted |
| --- | --- | ---: | --- |
| shape-110 missing-rule self-test catches a disabled expected-finding refusal | `crates/ryo/languages/python/benchmarks/linters/ruff/authoring/profile_staging.py:117` | 1 | yes |
| shape-110 oracle self-test catches a disabled sealed-oracle refusal | `crates/ryo/languages/python/benchmarks/linters/ruff/authoring/profile_staging.py:140` | 1 | yes |
| status guard P-T (planted at f5e6647370; profile_proofs.py unchanged since) | `crates/ryo/languages/python/benchmarks/linters/ruff/authoring/profile_proofs.py:135` | 1 | yes |

Each staging plant replaced the raise with `pass` and re-pinned the sha256 of profile_staging.py in prepared.json, so the hash check could not stand in for the guard.

## Enforcement candidates

All three earlier candidates are carried forward with instance_status fixed. The classes stay open for the TM steward to decide.

- `untested-refusal-1d8a82b3` (new rule): raise ValueError(f'...<message>') in a ruff authoring module whose <message> no authoring test module names
- `flag-without-producer-070501e7` (new rule): const NAME: &str = "--<flag>="; whose string literal occurs nowhere else in the tree outside tests
- `dangling-doc-link-ff109134` (new rule): [text](relative/path) in a tracked .md file where relative/path resolves to no tracked file or directory

## Advisories

- The expanded-defaults descriptor still declares `expectations_mode: complete-full-report` (crates/ryo/languages/python/benchmarks/linters/ruff/profiles/prepared.json:14079), and its expectations.json declares the same mode, a mode no code implements now. The descriptor cannot be staged: the benchmark hash refusal at crates/ryo/languages/python/benchmarks/linters/ruff/authoring/profile_staging.py:82-83 fires first. If it were re-pinned, the mode identity check at :90 would still pass, and the shape-overlay path would then refuse at :110 or on a missing count key. That fails closed, never open. It is historical, pinned data.
- crates/ryo/docs/contracts/evidence/inventory/items/benchmark.md:57 still lists `pub enum SelectedIds`. It is a generated census snapshot, refreshed with the next inventory run.
- The benchmark crate test build prints many existing warnings, for example missing docs on the fixture enums in outbound/suite/fixtures/contract.rs:15-53 and unnecessary qualifications. None is on a line the delta adds. The Rust gate owns them.
- Carried forward: crates/ryo/docs/contracts/records-326.md:21 cites a deleted results directory. It is a historical record.

VERDICT: PASS
