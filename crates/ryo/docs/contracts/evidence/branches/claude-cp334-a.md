# Leak review: claude/cp334-a (builder A, checkpoint 334 gate repairs .14 and .15), re-check

Source `cc649856d90a27ee2e8545e2ec0ec54b3f76506c`. Delta `3c249d1aec..cc649856d9`: one commit, touching only `run.py` and `run_test.py`. The prior review of `3c249d1aec` failed check 2 on the self re-exec loop and passed everything else. Those passes carry forward, because the delta does not touch what they rest on.

Reviewed tree the builder worktree: HEAD and `git status --porcelain` were the same (clean) before and after. All probes ran under the scratch directory, in a `git archive` copy and in a `git clone --shared` at the base and source shas. Both were deleted afterwards.

## Check 1: proof where the contract lives (PASS)
Carried forward. The delta adds one `run_test.py` case beside `run.py`.

## Check 2: every enforcement claim fires (PASS)
**The fix** (`run.py:7-17`):
- The probe now imports `tree_sitter, tree_sitter_rust` (`run.py:12`). This is the set the guarded import at `run.py:8` imports.
- `RYO_RUN_REEXEC` is set before `execvp` (`run.py:16`).
- A start that already carries the marker does not relaunch (`run.py:12`).
- The marker is dropped once both modules import (`run.py:9`).
- `OSError` from a broken native module is caught as well (`run.py:10`).

**Reproduced the prior loop environment**: a pytest tool environment with tree-sitter-rust 0.24.2 and no tree-sitter, run as `python3 -I run.py --census`. A `uv` wrapper counted the calls.
- tree_sitter missing: one probe, no relaunch, `run tools/install.py first`, exit 1.
- A planted `tree_sitter` that raises `OSError`: the same result.
- A planted `tree_sitter` that raises `ImportError`: the same result.

**At most once.** A fake `uv` whose probe succeeds but whose relaunch runs a Python without tree_sitter (`-I -S`):
- `cc649856d9`: exactly one probe and one relaunch, then the message and exit 1.
- `3c249d1aec`, under the same fake: looped, with 184 uv calls in 5 s, killed at exit 124.

**Census.** Measured at `752cc02c32` in a git clone, because the census reads `git ls-files`. Three runs:
- old `run.py` under the pinned env (CPython 3.12.14, `cp334a-uvtools`);
- new `run.py` under the same env;
- new `run.py` started from system `python3`, through the re-exec.

All three are byte-identical: 1152 bytes, sha256 `af503dd0...`.

**run_test.py** at `cc649856d9`: `Ran 74 tests ... OK`. That is 73 plus the new re-entry case.

The rest of the prior check 2 evidence carries forward: the cargo_diagnostics plants and mutations, FIX/DOC identity, and the empty tool env.

## Check 3: boundaries hold across crates (PASS)
Carried forward. The delta is Python only.

## Check 4: one implementation per concept (PASS)
Carried forward. There is still one launch path.

## Check 5: layer direction (PASS)
Carried forward. No Rust changed.

## Check 6: test seams stay out of production (PASS)
Carried forward. `RYO_RUN_REEXEC` is a re-entry marker that production uses, not a test seam. The new test sets it only to reach the second-start branch directly.

## Advisories (not failing)
- A1 to A4 from the prior report stand.
- **A5: the new case pins only the second-start branch.** `run_test.py:610-613` uses `-S` with `RYO_RUN_REEXEC=1` already set, so the probe never runs. Two things are left unpinned:
  - that the probe's import set matches the guarded import;
  - that the first start sets the marker.

  The prior report suggested a fake `uv` whose probe succeeds. That case (the probe above) would pin both, but it is not required by this re-check.

## Enforcement candidates
`self-reexec-without-reentry-guard-4e52ebed` is carried forward with `instance_status` `fixed`. The class stays for the TM steward to decide.

VERDICT: PASS
