# Asymptote

Asymptote is a runner for dev tools, meant for both local and CI. What makes Asymptote different than traditional runners like [pre-commit](https://pre-commit.com) and [husky](https://typicode.github.io/husky/), is that it tracks the output of the tools it runs as state. This allows for new tools and rules to be introduced with violations, that can be progressively addressed over time. Other tools in the progressive code quality space include [Betterer](https://github.com/phenomnomnominal/betterer) and [Ratchets](https://github.com/imbue-ai/ratchets).

## Install

| Language | Command |
| --- | --- |
| typescript | `pnpm i -D @idiomatic-sh/asymptote` |
| python | `uv add --dev idiomatic-sh-asymptote` |
| rust | `cargo install --locked idiomatic-sh-asymptote` |

## Usage

### Set up

```console
$ asymptote init
...
```

`init` writes an `asymptote.toml`, an empty lock in `.asymptote/`, and a git pre-commit hook that runs `asymptote check --hook pre-commit`.

### Add a lint tool (SARIF)

```toml
[[tools]]
id = "ruff"
group = "lint"
run = ["ruff", "check", "--output-format", "sarif"]
output = "sarif"
```

Each finding the tool reports is judged against the lock. Accepted findings live in `.asymptote/operations/*.lock`, one row each:

```toml
[[accepted]]
reasons = ["Existing backlog"]
records = [
  { tool = "ruff", rule = "F401", path = "src/app.py", level = "error", fingerprint = "ce8492c6238ed0db", count = 1, message = "`os` imported but unused", since_rev = "864c9c6e30c95e1dd443fc7cbfa2e624c7f6cbfa", since = "2026-10-26" },
]
```

The fingerprint does not depend on the line, so moving code does not make a finding new. A finding the lock does not hold fails the check.

### Add a perf tool (BMF)

```toml
[[tools]]
id = "bench"
group = "perf"
run = ["cargo", "bench", "--bench", "lint"]
output = "bmf"
```

The tool prints the [Bencher Metric Format](https://bencher.dev/docs/reference/bencher-metric-format/). The lock keeps the best value of each benchmark's measure:

```toml
[[perf]]
reasons = ["Median of 3 rounds"]
records = [
  { tool = "bench", benchmark = "ruff", measure = "peak-megabytes", best = 546.0 },
]
```

A run fails when even the low end of its range is above the best. When even the high end is below it, the check lowers the best.

### Groups

`group` is any label you choose. `asymptote check` runs every tool, and `--groups` runs only the tools of the groups named. `[hooks]` maps a git hook to groups:

```toml
[hooks]
pre-commit = ["lint"]
```

### Workspaces

The root `asymptote.toml` names its members, and each member has its own `asymptote.toml` and lock:

```toml
members = ["crates/*"]
```

`members` and `[hooks]` are read only at the root. `asymptote check` at the root runs the root's tools and every member's.

### Run

```sh
asymptote check                       # run every tool against the lock
asymptote check --groups lint,check   # run only these groups
asymptote check --since origin/main   # only what this branch introduced is new
asymptote check --fix                 # run each tool's fix command first
asymptote up --reason "<why>"         # accept unresolved violations
asymptote down                        # clear resolved violations
```

The lock is committed with the code. `up` and `down` each write a new file in `.asymptote/operations/`.

### Tool keys

| Key | Meaning |
| --- | --- |
| `id` | The tool's name in the lock and the output. Required. |
| `run` | The command, as a list. Asymptote runs CLIs on your `PATH`; it installs nothing. Required. |
| `group` | Any label; `--groups` and `[hooks]` select by it. |
| `output` | `sarif` or `bmf`. Without it, the tool passes or fails on its exit code. |
| `fix` | The command `check --fix` runs first. |
| `inputs` | Paths the tool reads, each named; a path dependency it reads must be named too, or a change in it reuses the last result. When none changed, the last result is reused. |
| `timeout-seconds` | How long the tool may run. Default 600. |
