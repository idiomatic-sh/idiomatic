# Leak review: claude/w15-loop at cde5651c7d (bead ryo-g3xh)

Base dcea2ee7df. The branch contains claude/w15-hooks at ecb3954140. Reviewed tree /workspaces/ryo-w15-loop: HEAD cde5651c7d5a82a72192aebbb045095395a98f59 and a clean status, both before and after the review. Plants ran in a `git archive` copy under the scratchpad, which was deleted afterwards. Evidence: `claude-w15-loop.json` beside this file.

## Check 1: proof where the contract lives: PASS
No contract is asserted from benchmarks/ or languages/. The only Rust-tree edit is the lint table, crates/ryo/Cargo.toml:149-152. The census (run.py:429-447) reads sources statically and proves nothing.

## Check 2: every enforcement claim fires: FAIL
The plants are in the JSON (count 0 means the violation passed).

**a. The builder test lane (guard_shell.py).** AGENTS.md:48 says builders run their own crate's tests and never run workspace-wide tests. These pass, each with exit 0:
- `flock LOCK cargo test --target-dir /workspaces/idiomatic-target/<tree> -- -p` (and `-- --package x`, and nextest with `-- -p`). `package_scoped` (guard_shell.py:249-257) scans the words after `--`. crates/ryo/Cargo.toml is a virtual workspace, so this runs every member.
- `cargo test -p '*'` and `cargo nextest run -p '*'`. Cargo documents glob patterns for -p. Also `-p a -p b ...`, because nothing ties -p to the builder's crate.
- The lock is matched as a substring (guard_shell.py:236). These all pass: `echo LOCK; cargo test ...`, `env Z=LOCK cargo test ...`, `flock LOCKx cargo test ...`, and `flock -s LOCK ...` (a shared lock, so builders run concurrently).
- `cargo t --workspace`, `cargo b --workspace`, `cargo coverage-all` (repo alias) and `cargo-nextest nextest run --workspace`. CARGO_BUILDS (guard_shell.py:14) has no aliases, and only `cargo` as the first word is recognised. This class predates the diff, but builders now have a test lane and these are its bypasses.

Required change in builder_check:
- Split `rest` at `--`.
- Refuse -p values that contain `*?[`, and require exactly one package, the worktree's crate.
- Require `flock` to be the wrapper word, with LOCK as its exact lock operand and without -s/--shared, instead of a substring anywhere.
- Treat every cargo invocation that is not check, test or nextest run as refused for builders (deny by default, not an allowlist of build verbs). Recognise `cargo-*` binaries.

**b. The reviewer verdict hold (acceptance_on_stop.py).** All of these pass without a real verdict:
- Leak-review accepts any `branches/*.json` with reviewer leak-review and a fresh mtime (line 18). So another branch's parallel review, or `touch` on an old file, releases a reviewer that wrote nothing.
- The verdict `"not decided"` with no source and no checks passes (line 80).
- A transcript with no timestamp switches off the mtime test (line 74).
- The failure counter `review-<agent>` (line 99) is shared by every instance of the agent type and survives across runs. The fifth stop exits 0.

Required change:
- Require the exact file named for this review (the branch from the prompt), validated by `shape()`.
- Require `source` to equal the sha the prompt names.
- Fail closed when the start time is unknown.
- Key the counter by agent id.

- Observed on this review: the tree regex (line 92) captured `/workspaces/ryo-333:` (the prompt's trailing colon; rstrip drops only `.,;`), so the hook held this reviewer although its verdict file exists. Required: strip `:` too, or take the tree from a fixed `INTEGRATION_TREE=<path>` token.

**c. Pushes to main (guard_shell.py:486-489).** These pass:
- `git push origin :main` and `+:refs/heads/main`: the empty-source skip at line 349 lets main be deleted.
- `git -c push.default=upstream push` from a branch tracking main.
- `git -c remote.origin.push=HEAD:refs/heads/main push origin`.

Required change:
- Check the destination even when the source is empty.
- Resolve a push with no refspec through `git push --dry-run --porcelain`, or refuse it unless the destination is spelled out.
- Advisory: `gh pr merge` is outside this hook.

**d. FIX (run.py:342-360).** These pass:
- `guard_shell::builder_check` in the message counts as a named test (NAMED_TEST, line 336).
- The entry's test is never matched to the commit's test.
- One existing log of any name, such as `cli_tests.log`, counts as the "red and green logs" (line 358).
- A behaviour change typed `chore:` is not checked.
- With no consolidation base, the check returns [] (line 347).

Required change:
- Bind the entry's test id to the test the message names.
- Require two distinct logs, and require that the red log shows that test failed and the green log shows it passed (parse them as `tests.json` does).
- Classify a commit by the files it changes, not by its type prefix.
- Fail closed without a base.

**e. DOC (run.py:381-397).** These pass:
- A new item in a renamed file (`--diff-filter=AM`, line 386).
- `pub fn new() -> Result` on a new type, because a `(fn, new)` is pub somewhere in the crate at base (line 394). A pub name in a base test file exempts a production item of the same name.
- A `src/test_*.rs` file (`is_test_file`).
- With no base, the check returns [] (line 384).

No re-export hole was found: `pub_use` is deny.

Required change:
- Diff with `--diff-filter=AMR`, or `--no-renames`.
- Key "old" by the item path (module and impl owner), not by kind and name, and only over base production files.
- Fail closed without a base.

**f. The census (run.py:422-447).** The contract review's isolation grade reads these counts, and they fall without any decoupling:
- `use Operation as O` takes operation_exhaustive_matches from 1 to 0.
- A same-named `pub struct` declared in any other crate's test removes the type from the vocabulary: replace_the_evaluator total goes from 3 to 1.
- A cross-crate test moved to `src/.../tests.rs` takes cross_crate_tests from 1 to 0, and the file is counted as production (line 426 checks `test.rs` but not `tests.rs`, and does not look at `#[cfg(test)]`).

Required change:
- Count with name resolution: alias-aware imports, and vocabulary by defining path rather than bare name.
- Classify `tests.rs` and `#[cfg(test)]` modules as tests.

**Not planted.** The clippy deny lints `missing_errors_doc` and `missing_panics_doc` were not planted. The guard refuses clippy outside an integration tree, and the build lock was held. This claim has no plant evidence.

## Check 3: boundaries hold across crates: PASS
No visibility, allowlist, import or feature change. All 26 package crates inherit `[lints] workspace = true`; the five manifests without it are test fixtures.

## Check 4: one implementation and one representation per concept: FAIL
acceptance_on_stop.py:69-81 `written_verdict` re-implements what counts as a valid review evidence file. review/README.md "Review evidence schema" names `shape` in ci/contract_review/service.py as the one definition, with exactly two callers. The copy is weaker: any verdict text passes, and no source or checks are required.

Required change: load service.py the way guard_shell.validated does, and call `shape(review, agent)`.

## Check 5: layer direction: PASS
No .rs file changed.

## Check 6: test seams stay out of production: PASS
No feature, cfg(test) bypass or public test helper was added.

## Beads (one per class, bead for the PM to file)
- builder test lane escapes: guard_shell.py:14, 236, 249-257, 269-275
- reviewer hold satisfied without its verdict: acceptance_on_stop.py:16-18, 60-99
- push to main by delete or config: guard_shell.py:349, 486-489
- FIX vacuous: run.py:336, 347, 358
- DOC vacuous: run.py:384, 386, 394, 329
- census gameable: run.py:426, 432, 439
- second verdict validator: acceptance_on_stop.py:69-81

VERDICT: FAIL: 2, 4
