# Leak review: claude/w15-loop at 94a0b93588 (delta 794ec68025..94a0b93588)

The last PASS was at 794ec68025. Its report is in git history, at ryo-333's `evidence/branches/claude-w15-loop.md`. The delta is only the two merges of claude/w15-hooks: 39dad80909 (1f4942d9f1) and 94a0b93588 (5763611e6d, the same TRACKER line as 98e0fcb703). It touches five files:
- `.agents/agents/tm-specialist.md:15`
- `.claude/hooks/acceptance_on_stop.py:84-115,139`
- `.claude/hooks/test_hooks.py:1422-1442`
- `ci/contract_review/service.py:56`
- `ci/contract_review/service_test.py:148`

## Check 1: proof where the contract lives: PASS
No Rust changed. No engine, core or SDK contract is asserted from `benchmarks/` or `languages/`.

## Check 2: every enforcement claim fires: FAIL
**The steward hold releases on a code change spelled as a rename.** `acceptance_on_stop.py:111-112` judges `tm-steward.json`'s source with `guard_shell.stands_for` (`guard_shell.py:700-707`). That function lists changes with `git diff --name-only source head` (line 706). Git detects renames by default and prints only a rename's new path. In a scratch repo, a commit that runs `git mv code.rs crates/ryo/docs/contracts/evidence/checkpoint/code.rs` after the decided sha makes `stands_for(A, B, EVIDENCE)` return True: a code change passes as "only evidence changed".

Every instance of the class:
- `guard_shell.py:706`, the shared definition.
- `acceptance_on_stop.py:112`, the new stop-hook use.
- `guard_shell.py:743`, the push gate's `previous_reviews`.
- `guard_shell.py:828`, `names_pushed_work`.
- `ci/contract_review/service.py:116`. Planted: moving `crates/ryo/src/x.rs` to `crates/other/src/x.rs` gives `since == ['crates/other/src/x.rs']`, so no protected change is seen. With `--no-renames` both paths appear and the change is protected.
- `ci/contract_review/service.py:100`, the activation check.
- `guard_shell.py:364`, `branch_packages`. It misses the package a file was moved out of.

Required change: add `--no-renames` to each of these `diff --name-only` calls, as `guard_shell.py:570,576` already do. Add one case to the existing tests, a rename into the allowed prefix that must hold, to `ReviewerStop.test_the_tm_steward_decides_the_landed_head_as_the_push_gate_reads_it` and to the service_test table. Do not add a parallel test.

**A loader failure releases the hold.** `guard_module()` (`acceptance_on_stop.py:85-92`, called at :111) runs outside any try. `main` catches nothing, so an exception exits 1, which releases the stop. In an archive copy, a planted `raise ImportError` at the top of `guard_shell.py` made all four subtests of the new test exit 1, including the two that must hold (expected 2). This contradicts the policy `shape_problem` states at :77: "holds the stop, never releases it". Required change: wrap the `guard_module()` and `stands_for` call the way `shape_problem` is wrapped, and return a hold on any exception.

**What holds.** On the reviewed tree, without plants, the new test passes 4/4, covering the landed head, an evidence-only ancestor, an ancestor with code changed since, and a non-ancestor. ReviewerStop passes 6/6. The source is shape-validated as 40-hex (`service.py:34`), so no option or ref spelling reaches git. The stop hook and the push gate use the same prefix (`guard.EVIDENCE`, evidence/checkpoint/).

**Tracker.** `(?<![\w-])ryo-[a-z0-9]{3,}(\.[0-9]+)*\b` still matches `ryo-g3xh`, `(ryo-g3xh)`, `bead:ryo-abc1`, `see ryo-fpg4.2.1.92` and `ryo-tq5.18`, and no longer matches `idiomatic-sh-ryo-sdk`. The scan covers `json.dumps(entry)` (`service.py:91`), and the class `see ryo-ab12.3` is still refused (`service_test.py:147`). The service_test table passes 62. Its 15 setup errors in the archive copy are environmental: the copy is not a git repo. Advisory: hyphen- and underscore-joined forms (`fix-ryo-g3xh`, `bead_ryo-abc`) are no longer caught. The old `\b` also missed the underscore form.

## Check 3: boundaries hold across crates: PASS
No visibility, manifest, allowlist or import boundary changed. The loader reads its sibling `guard_shell.py` by path.

## Check 4: one implementation and one representation per concept: PASS
The stop hook reuses `stands_for` and `guard.EVIDENCE` instead of a second ancestry check. Advisory, older than this delta: `EVIDENCE` means the evidence root at `acceptance_on_stop.py:16` and the checkpoint subdirectory at `guard_shell.py:29`, so one name has two meanings.

## Check 5: layer direction: PASS
No Rust changed.

## Check 6: test seams stay out of production: PASS
The only addition is the unittest at `test_hooks.py:1422`. Production imports nothing from it.

## Enforcement candidates
The 33 earlier candidates are carried forward unchanged. Two are new and open:
- `name-only-diff-hides-rename-source-25b627f8`
- `hook-loader-exception-releases-hold-218b1b33`

VERDICT: FAIL: 2
