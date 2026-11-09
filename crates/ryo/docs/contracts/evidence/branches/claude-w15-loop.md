# Leak review: claude/w15-loop at 944b21be87 (delta 94a0b93588..944b21be87)

The delta merges claude/w15-hooks 62b71db6cd. `git diff 94a0b93588 944b21be87` is byte-identical to `git diff 94a0b93588 62b71db6cd`. 28e1070bee has an empty tree change, and so does the merge 759fe35207. No other file changed. The delta touches six files: `.claude/hooks/{README.md,acceptance_on_stop.py,guard_shell.py,test_hooks.py}` and `ci/contract_review/{service.py,service_test.py}`. Plants ran in a `git clone --shared` copy of 944b21be87 under the scratch directory, which was deleted after the run. Both reviewed trees are unchanged: /workspaces/ryo-w15-loop at 944b21be87, clean; /workspaces/ryo-334 at afb0dbaa5d, with only these two files written.

## Check 1: proof where the contract lives: PASS
No Rust changed. No engine, core or SDK contract is asserted from `benchmarks/` or `languages/`.

## Check 2: every enforcement claim fires: FAIL
**Fixed, and the plants fire.** On the copy without plants, ReviewerStop passes 7/7 and service_test passes 82/82.
- `guard_shell.py:706` (`stands_for`): with `--no-renames` removed, the ReviewerStop subtest "code renamed into the evidence directory since" fails (count 1).
- `service.py:141` (since-review diff): with `--no-renames` removed, `test_a_protected_file_renamed_into_the_receipts_is_a_protected_change` fails (count 1).
- `acceptance_on_stop.py:111-115`: with the try removed, `test_a_guard_that_fails_to_load_holds_the_tm_steward` fails, 1 != 2 (count 1). That test also asserts `ModuleNotFoundError` in stderr, so it cannot pass through the shape-error hold instead. hook-loader-exception-releases-hold-218b1b33: fixed.
- tracker-id-definition-split-78f9a5a1 (from the 333 review): fixed. `service.py:56-81` reads the globs that name `ryo-` from the newest idiomatic-project placement (`spec.toml:16`). A missing spec or an unparsable version directory raises, and the shape check turns that into a hold, so it fails closed.

**Still open: name-only-diff-hides-rename-source-25b627f8, one instance my earlier enumeration missed.** `ci/contract_review/command.py:18` runs `git diff --name-only -z HEAD` without `--no-renames`. command.py imports service.py, so it is inside the closure. Plant: a staged `git mv crates/ryo/Cargo.toml crates/ryo/docs/contracts/evidence/Cargo.toml` gives 0 protected paths, and 1 with `--no-renames`. So the gate's "protected working tree changes must be committed before review" passes on an uncommitted move of protected source. Required change: add `--no-renames` there.

The rest of the class is closed. `run.py:394` uses `diff-tree`, which does no rename detection by default. `run.py:379` filters to `AM`, so a rename is dropped and the check fails closed. Outside the closure, as an advisory: `ci/perf/tree/adapters/git/adapter.py:19` has the same shape and misses the crate a file moved out of.

Advisory: the `--no-renames` additions at `service.py:125` (activation) and `guard_shell.py:364` (`branch_packages`) have no test. Reverting either one leaves every test green (count 0). They are correct now, but nothing pins them. The fix is one row each in the existing tables, not a new test.

**New, a regression from this delta: tracker-id-without-digit-unmatched-6c8faa27.** The 94a0b93588 validator refused a parenthesized four-letter id with no digit. `names_tracker` at 944b21be87 accepts it (plant count 0), and it also accepts 5-character ids such as `(ryo-ab1cd)`. The builder recorded this as a non-goal in `.claude/hooks/README.md:29`. Under the threat model it is not acceptable. The README admits a non-goal only when it "is a spelling an agent writes by accident" is false (README:20), and this spelling has been written by accident in the real tree, in the same kind of review evidence the validator guards:
- `crates/ryo/docs/contracts/audits/2026-11-06-phase3/review-314.json:61`
- `crates/ryo/docs/contracts/audits/2026-11-06-phase4/review-314.json:81`
- `crates/ryo/docs/contracts/records-326.md:174`

About 10 of the 69 distinct parenthesized ids in commit trailers have no digit. The rule misses all of them in shipped text too, because the validator now shares its gap.

Keeping one definition was right. The fix belongs in that definition. Required change: a new idiomatic-project spec version whose no-tracker-text-shipped placement adds a context form for a digitless 3- or 4-letter id: in parentheses (`*(ryo-[a-z][a-z][a-z][a-z])*`, `*(ryo-[a-z][a-z][a-z])*`, the same before `, ` or ` `), and after `bead`. Also update README:29 to list only the residual: a bare digitless id outside those contexts, which cannot be told apart from a package word like `ryo-core`. Then clear the three tree instances. The validator then follows the rule without any change of its own.

## Check 3: boundaries hold across crates: PASS
`service.py:56` reads the spec tree by path, read-only. No visibility, manifest, allowlist or import boundary changed.

## Check 4: one implementation and one representation per concept: PASS
The validator's own regex is gone. The tracker id has one definition, the rule placement (`service.py:60-69`).

## Check 5: layer direction: PASS
No Rust changed.

## Check 6: test seams stay out of production: PASS
The additions are confined to tests: `test_hooks.py:1448`, `service_test.py:74`, and the `None` removal in the fixture at `service_test.py:29`. No production path imports them.

## Enforcement candidates
All 35 earlier candidates are carried forward. Changes in this round:
- hook-loader-exception-releases-hold-218b1b33: fixed.
- name-only-diff-hides-rename-source-25b627f8: still open. `command.py:18` is added to its scope and plants.
- tracker-id-definition-split-78f9a5a1: carried from the 333 review, fixed.
- tracker-id-without-digit-unmatched-6c8faa27: new, open.

VERDICT: FAIL: 2
