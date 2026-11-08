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

A review request carries three receipts (`tm check` over the review's scope at 0 findings and 0 suppressed, `run.py --check` exit 0, and the `no_print` target green with file-level cycles 0 in `validation.json`), and a request without them is refused by the reviewer, because what a linter or check catches is resolved before a person reads the source.

## PM check-in

The main session posts a message starting `**CHECK-IN HH:MM UTC**` at least every 60 minutes, holding eight tables under bold numbered headings in this order: 1. Roadmap, 2. Running, 3. Changed since last check-in, 4. Cycle metrics, 5. Decisions I took without you, and anything outside what we agreed, 6. Suggested process improvements, 7. Needs you, 8. Next; `.claude/hooks/check_in_on_stop.py` (a `Stop` hook) blocks the main session's stop when the last check-in is older than 60 minutes or the one posted lacks a table or has them out of order.

## Builder brief contract

A builder stops in exactly two states, and `.claude/hooks/acceptance_on_stop.py` enforces both; `.claude/hooks/guard_shell.py` refuses the shell habits listed in its source.

- Every builder brief begins with `BEAD: <id>` and `WORKTREE: <path>`.
- Requirements are numbered R1..Rn, and each names the check that proves it.
- The brief has a Decisions table holding every decision the builder would otherwise have to ask.
- The bead's notes carry an `ACCEPTANCE:` ... `END ACCEPTANCE` block of shell lines that must exit 0 in the worktree.
- The report starts with `DONE:` (the acceptance output pasted, each R-number answered) or `BLOCKED:` (exactly one question the Decisions table does not answer). No other stop state exists.
- The integrator runs the acceptance block before merging.
- Builders never spawn subagents; the Agent tool is refused for them by a hook. One builder, one worktree; a scope too big for one builder is split by the PM into beads.
- Builders run tm continuously and `cargo check -p <their crate> --target-dir /workspaces/idiomatic-check-target/<worktree name>` before each count they append and before they stop; they never run tests, builds or clippy; the integrator compiles once.

The integrator's cadence: merge everything first; run ONE `cargo check --workspace --all-features --all-targets --keep-going --message-format=json --target-dir /workspaces/idiomatic-target/<worktree name>` under the build lock (every build, test and clippy of a tree uses that tree's own target, never another tree's; see [method](crates/ryo/docs/contracts/method.md) "Build targets") from pass 1, because library-only checks hid test errors until pass 3 at checkpoint 323; fix every error the pass reports, bottom-up by crate dependency, before checking again; run the full test build only when check is clean. The checkpoint record carries the pass count and the error count per pass, so a pass that fixed fewer errors than it reported is visible. Four passes (resolution, types, borrows, lints) is the floor rustc imposes; any pass beyond that is the integrator's.

Principal Rust review: after the compile is clean and before the push, the PM runs the [rust-principal-review agent](.agents/agents/rust-principal-review.md) as a read-only subagent on the merged tree; it scores seven items PASS or FAIL and the verdict is FAIL if any item fails. A PASS is committed as `crates/ryo/docs/contracts/evidence/checkpoint/rust-review.json`; a FAIL stops the landing, its findings become beads, and `.claude/hooks/guard_shell.py` refuses the push of `claude/ryo-consolidation` until HEAD carries a PASS whose `source` is an ancestor of HEAD. Every perf branch gets the same review before it merges.
