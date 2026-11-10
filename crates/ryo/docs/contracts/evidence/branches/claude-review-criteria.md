# Leak review: claude/review-criteria (thirteenth pass, targeted)

Source `1392542771c2c92729ad6b80c9114e10f30d6372`, base `438798b2d2` (pass 12, FAIL: 2). This pass covers the delta `438798b2d2..1392542771` (the hooks README, `no_subagent_builders.py` and `test_hooks.py`) and checks that the pass-12 FAIL is closed. Everything else in pass 12 passed and carries forward. The reviewed tree the integration worktree stayed at `da77293b16`, and nothing changed in it except these two evidence files. Hooks ran from a `git archive` copy in the session scratchpad, with its own `CLAUDE_HOOKS_STATE`.

## The fix: the pass-12 FAIL is closed
`no_subagent_builders.py:241` is now `brief = bool(brief_beads(prompt))`. Any prompt with a `^BEAD:` line under a type outside `ACCEPTED` is refused at `:242-244` with `WRONG_TYPE`, before any filesystem access.

I replayed the pass-12 plants with real PreToolUse payloads (`tool_name: Agent`, no `agent_id`). The real 2026-09-23 brief is the first message of `agent-a89d59f998e6155c0.jsonl` (`BEAD: <task id> ...`, no WORKTREE line).

| Brief | general-purpose | untyped | Explore | Plan | fork |
| --- | --- | --- | --- | --- | --- |
| no worktree line | 2 | 2 | 2 | 2 | 2 |
| `Worktree: <path>` | 2 | 2 | 2 | 2 | 2 |
| `WORKTREE=<path>` | 2 | 2 | 2 | 2 | 2 |
| `**WORKTREE:** <path>` | 2 | 2 | 2 | 2 | 2 |
| indented `  WORKTREE: <path>` | 2 | 2 | 2 | 2 | 2 |
| CRLF | 2 | 2 | 2 | 2 | 2 |
| real 2026-09-23 brief | 2 | 2 | 2 | 2 | 2 |
| control: no `^BEAD:` line | 0 | 0 | 0 | 0 | 0 |

All 35 refusals name `subagent_type builder` in stderr.

When I planted the old classifier back (`and brief_worktree(prompt) is not None`), the suite failed with 36 failures. I reverted it, and the file's diff against the sha is empty.

`guard_shell.py` equals `e75e47b059`: the diff is empty, and both blobs are `c6cbceaa4439810dc65892e3a5cd76be11e74af4`.

The full suite passes from a complete archive of `1392542771`: `python3 -m unittest test_hooks` ran 182 tests OK in 62.8 s.

## 1 Proof where the contract lives: PASS
The delta's tests run on temp fixtures and assert no engine, core or SDK contract:
- `test_hooks.py:1848-1855` loops over `malformed_briefs` under five types.
- `:1925-1926` is `test_not_a_builder`.

## 2 Every enforcement claim fires: PASS
The wrong-type refusal now fires for every beaded brief under a type outside `ACCEPTED`; the evidence is above. Harness scan of the delta: every subTest asserts both the return code and the stderr, with no skip, caught error or weakened assertion. The `Bead:` case asserts 0 because it is not an exact `BEAD:` line, and the pass-12 gate still refuses it under builder.

Advisories, which do not decide the verdict:
- **Integrator (carried forward).** An integrator whose brief has `BEAD:` lines and a malformed WORKTREE line still stops 0 on a failing DONE (`acceptance_on_stop.py`, which is not in the delta). Candidate `3a4d8735` stays open.
- **Fail-closed side effect.** Any non-builder launch prompt with a line starting `BEAD:` is now refused. No agent definition under `.claude/` at this sha has such a line (`git grep -nE '^\s*BEAD:' -- .claude`, excluding the tests, is empty). A PM who pastes `BEAD:` into a reviewer prompt gets a refusal that tells them the fix.

## 3 Boundaries hold across crates: PASS
The delta touches only `.claude/hooks/`. It has no Rust visibility, manifest or import boundary.

## 4 One implementation and one representation per concept: PASS
- `brief_beads` (`no_subagent_builders.py:36-40`) stays the one parser.
- `WRONG_TYPE` is defined once (`:25-27`).

## 5 Layer direction: PASS
No crate is touched.

## 6 Test seams stay out of production: PASS
Only `test_hooks.py` sets `CLAUDE_HOOKS_STATE`. `guard_shell.py` is unchanged from `e75e47b059`.

## 16 Dependency and supply-chain hygiene: PASS
The delta adds no import.

## 17 Portable and reproducible: PASS
182 tests pass from a complete clean archive with their own state root.

## 18 Use what everyone uses: PASS
No new tool, parser or format.

## Enforcement candidates
- `beaded-brief-under-an-ungated-type-skips-every-gate-028cc8b1`: the instance is now fixed. The class stays with the TM steward.
- Open: `integrator-stop-releases-a-beaded-brief-without-an-exact-worktree-line-3a4d8735`, `acceptance-outlives-the-hook-timeout-07e399b3` and `stop-reads-only-the-first-brief-3fd912d9`.
- The other four are carried forward as fixed.

VERDICT: PASS
