# Leak re-review: claude/w15-loop

Source `07bc701e6c`, delta since `944b21be87` (merge of claude/w15-hooks `e75e47b059`). Scope: the delta only.

## Check 1: Proof where the contract lives: PASS

git diff 944b21be87..07bc701e6c (one merge of claude/w15-hooks e75e47b059) touches 12 files: .claude/hooks/{README.md,test_hooks.py}, ci/contract_review/{command.py,service_test.py}, the idiomatic-project spec 0.3.1->0.3.2 rename, crates/ryo/tastemaker.lock:153, and three docs with ids cleared; no Rust, no benchmarks/ or languages/ proof.

## Check 2: Every enforcement claim fires: PASS

Each --no-renames removal fails its own test: command.py:18 -> test_an_uncommitted_rename_of_a_protected_file_is_a_protected_working_tree_change; service.py:125 -> test_a_protected_file_renamed_into_the_receipts_since_activation_needs_a_review; service.py:141 -> test_a_protected_file_renamed_into_the_receipts_is_a_protected_change; guard_shell.py:364 -> test_a_file_moved_out_of_a_package_still_counts_as_that_packages_change. tm 0.1.2 check of crates/ryo with plants in crates/sdk/src/lib.rs:23-36 reported 8: all seven claimed spellings (the id in parentheses, in parentheses before a word, and after bead, bead_, bead-, Bead: and beads, the id being <ryo- and four letters, no digit>) and one false positive (<ryo- and a three-letter name>) at :36; package names at :30 and the bare id at :31 not reported; docs/ reports 0, so the cleared tree is clean. Dropping the five new globs fails service_test variants 49 and 50 (validator reads the same definition). Advisory: beads: <ryo- and four letters, no digit>, a wrapped (<ryo- and four letters, no digit> at line end, [<ryo- and four letters, no digit>] and (see <ryo- and four letters, no digit>) escape; three bare digitless ids remain at audits/2026-11-06-phase3/review-314.json:77, phase4/review-314.json:68, records-326.md:150, the class README.md:29 leaves out on purpose.

## Check 3: Boundaries hold across crates: PASS

Deleting idiomatic-project 0.3.1 breaks no pin: crates/ryo/tastemaker.lock:153 is 0.3.2; crates/asymptote/tastemaker.lock:134 and crates/tastemaker/tastemaker.lock:136 pin 0.1.0 (present); the root tastemaker.lock and crates/tastemaker/examples/*/tastemaker.lock place no idiomatic-project; nothing under tastemaker/ names 0.3.1 of it except the 0.3.2 CHANGELOG history line.

## Check 4: One implementation and one representation per concept: PASS

service.py:60-69 still reads the tracker globs from the newest idiomatic-project spec (sorted numerically: 0.3.2), so the rule and the validator share one definition; plant confirms (service_test variants fail when the spec globs are removed).

## Check 5: Layer direction: PASS

No Rust changed in the delta.

## Check 6: Test seams stay out of production: PASS

Only test additions: service_test.py:82-97,181-183 and test_hooks.py:211-223; no production path imports them.

## Plants

| Claim | File | Count |
| --- | --- | --- |
| command.py working-tree check --no-renames | `ci/contract_review/command.py:18` | 1 |
| validate activation --no-renames | `ci/contract_review/service.py:125` | 1 |
| validate since-review --no-renames | `ci/contract_review/service.py:141` | 1 |
| branch_packages --no-renames | `.claude/hooks/guard_shell.py:364` | 1 |
| no-tracker-text-shipped 0.3.2 digitless id in parentheses and after bead (7 spellings) | `crates/ryo/crates/sdk/src/lib.rs:23` | 7 |
| no-tracker-text-shipped 0.3.2 leaves package names unmatched | `crates/ryo/crates/sdk/src/lib.rs:30` | 0 |
| validator refuses digitless ids from the spec globs (globs removed) | `tastemaker/specs/idiomatic-project/0.3.2/spec.toml:16` | 2 |

The package-name row is a negative control: 0 is the expected count.

## Enforcement candidates

- `name-only-diff-hides-rename-source-25b627f8`: fixed (all four sites carry `--no-renames`, each with a test that fails without it).
- `tracker-id-without-digit-unmatched-6c8faa27`: fixed (parenthesized and after-bead spellings reported; docs report 0).
- `tracker-id-spelling-unmatched-8d65fb3b` (advisory, open): `beads: <ryo- and four letters, no digit>`, `(<ryo- and four letters, no digit>` at line end, `[<ryo- and four letters, no digit>]`, `(see <ryo- and four letters, no digit>)` are not reported.
- `short-name-in-parentheses-reported-as-tracker-id-f026c12e` (advisory, open): `(<ryo- and a three-letter name>)` is reported as a tracker id; no instance in the tree.
- Every earlier candidate is carried forward with its status.

VERDICT: PASS
