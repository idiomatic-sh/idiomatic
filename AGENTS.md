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

A census of code items (variants, kinds, routes, entry points) is a test that derives the list from the source, never a hand-kept list, because a list kept by hand drifts from the code it counts.

Every new branch or refusal path names its positive and negative control in the batch report, so a reviewer finds both tests without searching.

A test whose checked property changes is renamed in the same commit, and its matrix row says what it checks now, because a name that states the old property misleads every later reader.
