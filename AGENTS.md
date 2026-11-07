# Codex Instructions

Batch repairs by default. Collect the full diagnostic set, group related failures,
and fix the affected implementation, callers, and tests before the next combined
check. Parallel agents hand off cohesive batches; one integrator runs shared
checks. Do not compile after each individual fix. Use a narrow diagnostic run
only to resolve a specific uncertainty, and state why it is needed.

For architecture or performance audit findings, use the [systemic audit skill](.agents/skills/systemic-audit/SKILL.md) before claiming an implementation family is closed.

At 2% remaining weekly allowance, when reported by the user or a reliable usage
indicator, stop starting new work and prepare a Claude Code handoff. Checkpoint
all agent code on named branches, integrate coherent batches, and record exact
commits, remaining changes, live processes, failures and next commands in the
existing Beads issue. Preserve failing tests and useful build caches; do not
claim completion or weaken contracts to make the handoff look green. Keep this
checkpoint current during normal work. Never infer the allowance from token counts.

Never run bare `git stash` or `git stash pop`: the stash stack is shared across worktrees, so commit work in progress instead.

Use zero suppressions (no `// ignore:` comments, no `#[ignore]`, no allow attributes), because a suppressed finding is a hidden defect; split or fix the code instead.

A wait loop must never match its own command line with `pgrep -f`, because it finds itself and never exits; wait on a file or an exit code.

## Builder brief contract

A builder stops in exactly two states, and `.claude/hooks/acceptance_on_stop.py` enforces both; `.claude/hooks/guard_shell.py` refuses the shell habits listed in its source.

- Every builder brief begins with `BEAD: <id>` and `WORKTREE: <path>`.
- Requirements are numbered R1..Rn, and each names the check that proves it.
- The brief has a Decisions table holding every decision the builder would otherwise have to ask.
- The bead's notes carry an `ACCEPTANCE:` ... `END ACCEPTANCE` block of shell lines that must exit 0 in the worktree.
- The report starts with `DONE:` (the acceptance output pasted, each R-number answered) or `BLOCKED:` (exactly one question the Decisions table does not answer). No other stop state exists.
- The integrator runs the acceptance block before merging.
- Builders run tm and the acceptance block; they never run cargo. The integrator compiles once per checkpoint.
