# Harness hooks

`guard_shell.py` (PreToolUse, Bash), `guard_write.py` (PreToolUse, edits), `no_subagent_builders.py` (PreToolUse, Agent:
no subagent spawns another, and a `builder` or `contract-migration` launch needs `SCOPE=<path>`, a report whose second line is `BEADS: <id> ...` listing its bead, with a passing
`scope-review-<report stem>.json` that carries the report's sha256),
`acceptance_on_stop.py` (SubagentStop) and `check_in_on_stop.py` (Stop). `test_hooks.py` holds their cases; run it with
`python3 -m unittest test_hooks` in this directory.

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
- `gh pr merge`. Server-side branch protection on `main` is the complete fix for every push to it.
