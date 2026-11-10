# Harness hooks

`guard_shell.py` (PreToolUse, Bash), `closed_effect.py` (PreToolUse and PostToolUse, Bash), `guard_write.py`
(PreToolUse, edits), `no_subagent_builders.py` (PreToolUse, Agent:
no subagent spawns another, judged by the payload's `agent_id`; a prompt with a `BEAD:` line launches only
as `builder`, `contract-migration` or `integrator`; and a `builder` or `contract-migration` launch needs exact
`BEAD: <id>` and `WORKTREE: /abs/path` lines and `SCOPE=<path>`,
a report whose second line is `BEADS: <id> ...` listing every bead of the brief, with a passing
`scope-review-<report stem>.json` that carries the report's sha256), `acceptance_on_stop.py` (SubagentStop: the
builder, contract-migration and integrator stops run each bead's acceptance block, the reviewers and the scope review
hold until their verdict file is valid, a builder or contract-migration brief without those exact lines holds
unless it replies BLOCKED, and any error holds the stop; it reads the agent's own transcript,
`agent_transcript_path`, before `transcript_path`, which is the parent session's) and `check_in_on_stop.py` (Stop, the
main session's own `transcript_path`). `test_hooks.py` holds their cases; run it with
`python3 -m unittest test_hooks` in this directory.

Tastemaker manifests are closed for extension. `closed_effect.py` holds the fact model: a manifest is diffed as
TOML facts (scope `rules[<id>]`, `templates[<id> as <alias>]`, `specs[<id>]`, dotted key path, list positions erased),
and every added fact and every removal that loosens is refused: an agent may only remove an `exclude`, `except`,
`allow` or `rules-exclude` element or a single-file template placement, and edit descriptions, comments and order. An
added `ignore: idiomatic.` line is refused in any file, manifests included. `closed_for_extension.toml`, the owner's
allowlist beside the hooks in idiomatic-internal, admits named changes; the owner edits it by hand (absent or
unparsable, it admits nothing; each refusal prints the block that would admit it, and an entry admits only with its
`reason`), and agents never change it. `guard_write.py` applies the model before a Write or Edit. For Bash,
`closed_effect.py` judges the effect, never the spelling: before the command (PreToolUse) it snapshots each repository
the command may touch, every tracked and untracked file as a git tree written through a temporary index, with the
refs and the allowlist; after it (PostToolUse) it diffs the trees, refuses what the model refuses, and restores each
such file to its content before the command (a new file is removed). Content git brings from commits that existed
before the command (a checkout, merge, rebase or reset, or a side of a merge in progress) passes, and a conflicted
manifest passes until it is resolved. A snapshot that failed refuses after the command and restores nothing.

## Threat model
The hooks stop **honest mistakes by our own agents**: the command an agent writes by habit, by copying an old brief or
by not knowing the rule, such as a workspace-wide test from a builder tree, a push to `main`, a missing build lock or a
reviewer that stops before writing its verdict. They are not a defence against deliberate evasion. A text guard over a
shell command line cannot close every spelling, and chasing obfuscated spellings is a loop with no end. A leak review
grades a hook escape only for a spelling an agent would plausibly write by accident.

The census in `run.py --census` reads Rust source with a hand-written scanner until it is replaced by ryo queries over
the source (after the checkpoint-loop PR). A further edge case of that scanner is owned by that
replacement: a leak review reports it as an advisory, not a check 2 failure, unless the shape occurs in the real tree.

A here-document body is read as commands, like every other line: telling data from commands there was tried and
each attempt hid later lines from the guards. The cost is a refusal when a heredoc's text mentions `cargo` or a push;
write such text with the Write tool and pass the file (`git commit -F <file>`).

## Known non-goals
These reach past the guards and are left out on purpose; none is a spelling an agent writes by accident:
- option clusters that hide a wrapper's value from its parser beyond flock's own (`xargs -0n 1`, `timeout -vs KILL 5`,
  `env -iu X`, `env -iC /tmp`, `ionice -tc 2`);
- a command handed to a wrapper as one string (`flock <lock> -c '...'`, `env --split-string='...'`), `rustup run`,
  `make`, `$(which cargo)`, and an interpreter that runs a command (`python3 -c "os.system(...)"`);
- push configuration from the environment (`GIT_CONFIG_PARAMETERS`, `GIT_CONFIG_COUNT`/`KEY_n`/`VALUE_n`), git aliases
  that expand to `push` (`git -c alias.p=push p`, `git config alias.p push`), `push.default` and glob refspecs set in
  the repository's config before the push, and `git send-pack`;
- a hand-written log that copies a test runner's frame (`run.py --check` reads the runner's own header and summary
  lines, which catches a typed result, not a forged one);
- a bare tracker id with no digit and no child number (`ryo-abcd` standing alone), which reads like a package name
  (`idiomatic-sh-ryo-sdk`); one in parentheses or after "bead" is reported. The review evidence validator (`ci/contract_review/service.py`) reads its tracker-id globs from that placement in
  the newest `idiomatic-project` spec, so the rule and the validator hold one definition;
- `gh pr merge`. Server-side branch protection on `main` is the complete fix for every push to it;
- for the closed-for-extension effect check: a repository the command reaches only through a path its text does not
  spell (a variable, a relative `cd` out of the working directory's repository), files git ignores, a command's
  background processes that write after it returns, and a list element moved between two entries of the same scope
  (two unaliased entries of one rule), which keeps the fact counts equal. A change already committed by the command
  stays in that commit; the refusal says to commit the restored file.
