# asymptote

asymptote is the ratchet: its own product, not a part of Tastemaker, reading any tool's SARIF. This page is its design: the commands, what each prints, the lock, and each design decision with its reason.

## Commands

```text
asymptote init [-n]                         # write a minimal asymptote.toml and the git hook scripts [hooks] lists
asymptote check [--since <rev>] [--fix] [--groups <group>[,<group>]... | --hook <name>]
                                            # run the tools, judge them against the lock; 0 clean, 1 new, resolved or perf failing, 2 a tool failed
asymptote up --reason <why> [--rule <id>]... [--path <glob>]... [--tool <id>]... [--at <path>:<line>]... [-n]
asymptote down [-n]                         # drain every record the last check found fixed or no longer reported
```

### Groups

Every `[[tools]]` entry of `asymptote.toml` names one `group`, required: a non-empty label of your choosing. Asymptote gives
no group a meaning, and a tool's `output` (`sarif`, `bmf`, or its exit status when absent) is read the same in any group. A
plain `check` runs every tool; `check --groups lint,check` runs only the tools labelled `lint` or `check`. A tool outside the
groups is not run and not reported, and the lock's records of it are neither new nor resolved nor unjudged: they are not
looked at. `--groups` is the one flag `check` takes for it; a group no tool of the products checked declares is an
error naming it.

```toml
[[tools]]
id = "ryo-bench"
run = ["cargo", "run", "--release", "-p", "ryo-bench", "--", "--bmf"]
group = "perf"
output = "bmf"
```

### Tool keys

A `[[tools]]` entry takes `id`, `run` and `group` (required), and `fix`, `output` (`sarif` or `bmf`), `timeout-seconds`,
and `inputs` when needed. A tool whose inputs did not move keeps its last run. A tool whose result depends on what the
files do not hold, as the root's `commitizen` tool reads git's history, names the range in its `run`
(`origin/main..HEAD`, or `a...b`): the stamp of a tool whose command line names a range holds the commits its two ends
resolve to, so a new commit runs it again and an unchanged branch keeps its run. No key says so.

An input may write a capture, `{name}`, with Tastemaker's syntax and meaning: one or more characters within one path
segment, a name written twice taking one value. A tool whose inputs write one is one tool per entry they match on disk,
sorted, its id `<id>/<value>` (each value after a `/`, in the order the inputs first write the names), and the value
written into its `run`, `fix` and inputs; each is stamped, cached and reported on its own, so an edit in one directory
reruns only its tool, and a directory that appears is a tool more with no edit of `asymptote.toml`. A `run` or `fix`
that writes a capture no input does is refused. Braces around anything that is not a name (`{7}`, `{a,b}`) are text.

`run` and `fix` name a bare command on the PATH: asymptote assumes nothing about how a tool is installed, and a
version or a launcher in the line is the installer's business, not the configuration's.

### Hooks and `asymptote init`

The root `asymptote.toml` (only the topmost one, like `members`) may map git hooks to the groups they run:

```toml
[hooks]
pre-commit = ["lint"]
pre-push = ["lint", "check"]
```

With no table no hook maps any group, and `check --hook` is refused; `asymptote init` writes `pre-commit = ["lint"]` into a new `asymptote.toml`. Only the hook names a git client runs are accepted
(`applypatch-msg`, `pre-commit`, `commit-msg`, `pre-push`, ...), and a hook that maps a group no tool of the workspace
declares is an error naming it, for every `check`, not only `check --hook` (a workspace with no tool at all declares
none yet, so the manifest `asymptote init` writes checks clean until the first tool is added); a member that declares `hooks` is an error
naming the member.
`asymptote check --hook <name>` runs the groups its entry maps (inside a member it reads the root's table), is an error
with `--groups`, and an error naming the mapped hooks for a hook the table does not map.

`asymptote init` is run once in a git repository (`-n`/`--dry-run` says on stderr what it would write and writes
nothing):

1. where there is no `asymptote.toml` it writes a minimal one and an empty lock at the repository's top level, where
   the hook runs, whatever directory it ran in; an existing one is left as it is and said so;
2. for each hook `[hooks]` lists it writes an executable script where git keeps that hook (the answer of `git rev-parse
   --git-path hooks/<name>`, so `core.hooksPath` and linked worktrees hold):

   ```sh
   #!/bin/sh
   # written by asymptote init; edit [hooks] in asymptote.toml instead
   exec asymptote check --hook pre-commit
   ```

3. it prints what to run next (`next: asymptote check`).

Rerunning it rewrites its own scripts, found by the comment line, and removes the ones it wrote for hooks no longer
listed. It never touches a hook it did not write, and a symlink, dangling or not, is never one it wrote: it refuses before writing anything, naming the file (a failure, exit
1, with a remedy). A directory in no git repository is refused the same way. The commit being made is checked by the
`pre-commit` hook; commit messages are checked by the root's `commitizen` lint tool over the branch's commits, so the
commit being made is checked at the next commit or in CI.

### Perf records

A `perf` tool prints [Bencher Metric Format](https://bencher.dev/docs/reference/bencher-metric-format/) on stdout and
exits 0 (any other exit is a failed tool, as for an exit-status tool): an object of benchmarks, each an object of
measures, each `{ "value": n, "lower_value": n, "upper_value": n }` with the two bounds optional. The measures are the
tool's slugs (`wall-ms`, `cpu-ms`, `peak-mb`). Anything that is not that shape, a missing or non-numeric `value`, a negative number, a
spread that does not hold its `value` (`lower_value <= value <= upper_value`), or an output with no measurement, is a
failed tool.

The lock keeps one record for each `(tool, benchmark, measure)` in the same operations as the findings, one more section
of the operation file, written only when an operation has one. The fold is the lock's: a descendant wins, a concurrent
removal wins, and concurrent settings keep the strictest, the lowest `best`.

```toml
version = 1
parents = []

perf-removed = [
  { tool = "ryo-bench", benchmark = "check-ryo", measure = "cpu-ms" },
]

[[perf]]
reasons = ["seeded"]
records = [
  { tool = "ryo-bench", benchmark = "check-ryo", measure = "wall-ms", best = 831.0 },
]
```

`best` is a number of zero or more, and `reasons` say why this best stands, each reason `up` was given for it, written
once for the records set for the same reasons. A run is
judged by its spread: `lower_value` and `upper_value`, or its `value` alone where the tool reports no spread.

| the run against the record | `check` |
|---|---|
| its low end is above `best` | fails (exit 1): `regressed: check-ryo/wall-ms 920 is over the best 831 (ryo-bench)` |
| its high end is below `best`, and its `value` too | rewrites `best` to the run's `value` in the lock, in a new operation, and passes: `improved: check-ryo/wall-ms 831 -> 700, commit the lock (ryo-bench)`. This is the one place `check` writes the lock, only a plain `check` writes it, and only ever lower: `--fix` and `--since` never do, and say so (``the lock is left as it is: a plain `asymptote check` rewrites the best``) |
| otherwise | passes |
| no record | fails (exit 1): ``unseeded: check-ryo/wall-ms 831; seed with `asymptote up --reason <why> --tool ryo-bench` `` |
| a recorded measure the tool no longer reports (its run completed) | resolved (exit 1): `asymptote down` drops the record |

`asymptote up --reason <why> --tool <id>` seeds and raises: the last check's medians of the tool that have no record, or
whose low end is above their `best`, are written as records with `best` the median, in one operation however many there
are, together with any findings the other selectors choose. A record raised keeps its reasons and appends `--reason`.
`--rule`, `--path` and `--at` name findings and leave perf alone. `asymptote down` drops the records whose tool ran and
no longer reports them.

### Workspaces

A root `asymptote.toml` may carry `members`, globs of directories that each hold an `asymptote.toml`, beside its own
tools (it may carry only `members`):

```toml
members = ["crates/*"]
```

`asymptote check` at the root runs the root's own tools, then each member's, each member against its own lock, cache and
report, with `--groups` and `--fix` applied to all of them. Every line is prefixed with the product's path (`.` for the
root), and the run ends as the worst of them: 2 when a tool failed, then 1 for something new or resolved, then 0.
`inputs` stay relative to the directory of the `asymptote.toml` that names them. Run inside a member (the nearest
`asymptote.toml` is the member's), `check` runs that member alone, and so do `up` and `down`, which never reach the
workspace. At the root, `up` and `down` reach the products `check` reaches there, the root's own tools when it has any
and then each member, each against its own lock and last check, every line prefixed with the product's path; `up`'s
selectors apply within each product. A glob that matches no directory, a path that does not exist, a directory with no
`asymptote.toml`, and a member that declares `members` are errors naming the member; `members` is allowed only in the topmost `asymptote.toml`.

The topmost `asymptote.toml` may pin the asymptote the repository needs. A full version is that version exactly;
`^0.1` (or `0.1`) is Cargo's caret requirement, met by any 0.1.x, compared as numbers:

```toml
asymptote = "0.1.0"
```

Every command, a hook's included, reads it first and stops before reading or writing anything else when another
asymptote runs:

```text
error: this repo needs asymptote 0.1.0; 0.2.0 is installed
help: install asymptote 0.1.0 as the README's Install section says, as in `cargo install --locked idiomatic-sh-asymptote@0.1.0`
```

`--format sarif` is one product's log, so a workspace refuses it; `--format json` carries one entry for each product.

`check` prints one line per finding, compiler-style, sorted by path and line, so a terminal, an editor or a script
opens it directly, then one summary line:

```text
crates/core/src/lint.rs:1:1: tastemaker/idiomatic.arch/file-length: this file is longer than 400 lines
crates/core/src/lint.rs:88:5: cargo-clippy/clippy::needless_pass_by_value: this argument is passed by value
2 new, 0 resolved
```

The summary line goes on with `, <n> unjudged: a tool did not run, or a rule was not evaluated` when records could not
be judged, and with `; perf: <n> regressed, <n> unseeded, <n> improved, <n> resolved, <n> held` when a perf tool
measured. A tool that failed is said on stderr, `<tool>: failed: <why>`, in every format and under `--quiet` too.

`--format json` carries the same fields for each finding (`path`, `line`, `column`, `tool`, `rule`, `message`,
`level`, `fingerprint`, `status`: `new`, `resolved` or `accepted`, and for a resolved record its `count` and
`remaining`; a record has no `line` or `column` and states its `count`). A finding an in-source ignore suppresses is in
`suppressed` instead, with the ignore's `reason`; a record that cannot be judged is in `unjudged`, with `why`. `up` and
`down` print their records and findings in the same row: `up`'s `recorded` and `left`, `down`'s `recorded`, and its
`drained` with `why` and `reasons`. Each tool is `{id, reused, error?}`, its `error` how it ended (`ended`, and its
`code`, `after_seconds` or what it `said`). `--format sarif` is one SARIF 2.1.0 log, one run per tool, each result's
`baselineState` `new`, `unchanged` or `absent`, its fingerprint in `partialFingerprints`, a suppressed one's
justification its ignore's reason.

## What did my branch introduce

`check --since <rev>` judges the run against what `<rev>`'s branch accepted: the union of the lock at the merge base of
`HEAD` and `<rev>` and at `<rev>` itself, carried through the renames committed since the merge base. A finding this
branch accepted with `up` is still new against it, so a run sees exactly what its branch introduced:

```text
$ asymptote check --since origin/main
crates/core/src/new.rs:12:9: cargo-clippy/clippy::unwrap_used: used `unwrap()` on a `Result` value
1 introduced since origin/main, 0 resolved
```

## Fast on a warm tree

`check` keeps its report in `.asymptote/cache/check.store` (the directory ignores itself in git): flat tables, each text stored once, the build that wrote them kept once, so another build starts empty. Each tool's run is kept under the stamp
of its inputs: its command line and the BLAKE3 digest of every file its `inputs` pathspecs name (the product, when
`asymptote.toml` names none), the lock left out; an input git ignores counts when it is named by its own path. A
tool's inputs reach only what they name: a compiling tool names each path dependency out of its product and the
build configuration it reads (`inputs = [".", "../sdk", "../../.cargo", "../../rust-toolchain.toml"]`), so a change to
a sibling SDK reruns it; asymptote reads no build manifest to find them. A tool whose stamp did not move is not run again.
A check in which a tool fails still keeps its report: the failed run is recorded, so `up` and `down` refuse naming the
tool, and the tool's last completed run is kept beside it, so the other tools, and the tool itself once its inputs are
back, are reused, not run again.
Digests are kept as git's index keeps them: each file's size, modification time and digest are recorded, and a file
whose size and time still match keeps its digest unread. A record taken less than 2 s after the file was modified is
racy, so the file is read again; a same-size write in the tick the record was taken in is never missed. A modification
time restored by hand over changed content is accepted, as git and cargo accept it. Over a large Rust workspace (5
warm runs each, no build running): 0.45-0.50 s, against 0.52-0.55 s when every input was read and digested and git
was asked about ignored inputs once per pathspec; after a one-line edit of `cli/src/main.rs` every tool
reruns and the check took 7.5 s; the first, cold, run takes the tools' minutes. `up` and `down` run no
tool: they read that report, and refuse when the commit, `asymptote.toml` or a file it rests on has changed since.

## Design decisions

The lock is immutable, content-addressed operations in `.asymptote/operations/<blake3>.lock`, each stating `version = 1`
and naming its parents, folded on read. The version stays 1 until the first release, whatever the schema becomes before
it; an operation of another version is refused, naming its file. An operation writes each section only when it holds something: the keys it removes, the suppressed
findings it accepts (their reason is the in-source ignore, so they carry none), and its accepted records grouped under
the reasons they were accepted for, each reason written once. Once the lock holds 64 operations, `up` or `down` replaces
them with one snapshot that covers them and deletes their files, so the lock stays bounded; an operation a concurrent
branch wrote on a covered parent folds as concurrent with the snapshot, so a snapshot also carries the removals that
stand and the coverage of every snapshot it replaces. Compaction fails closed: a branch that accepts a record again on a
parent a snapshot has covered is refused when the lock is compacted and accepted when it is not, never the other way
round. The removals that stand are kept for good, so the number of files in the lock stays bounded while its bytes grow
with them.

| decision | why |
|---|---|
| one file per operation, never edited, merged by git | "Concurrent branches add different files, so stock Git, jj, and Mercurial merge them without a custom driver or local installation." |
| the fold: a descendant wins, concurrent accepts keep both reasons and the stricter count, a concurrent removal wins | "one concurrent branch removes an id: removal wins, so the gate fails closed" |
| `--since <rev>` | the accepted set is the union of the lock at the merge base and at the target's tip, so a run cannot pass by accepting its own findings; a caller uses it to decide what a branch introduced |
| `check --fix` runs each tool's fix command first, and never writes the lock | the path to a clean run is `check --fix` then `check`; accepting a finding is `up`, with `--reason` |
| `up` needs `--reason`, which applies to every record it writes | "`--reason` is mandatory and lands on every record it writes, so accepting a finding is a visible, attributable act in review" |
| renames carried through git: a record moves to the new path only when the run reports its identity there; `down` drops it otherwise | "A git rename carries a record to the new path only when the run reports its identity there; otherwise the record is dropped." |
| compaction is automatic: `up` and `down` append a snapshot once the lock holds 64 operations since its last one, when every rename projection is carried | "The snapshot advances each live record's origin, so future rename projection starts there instead of replaying older repository history." and "Compaction is an optimization, never a read requirement." |
| the fingerprint is the item fingerprint: the reported line's text, whitespace removed, and the name of the item that encloses it, read from indentation (a body that is not indented belongs to the item whose opening brace ends the line above it, until its closing brace); a whole-file result's is one fixed value; a tool's own `primaryLocationLineHash` is used only when the file cannot be read | "it tells apart the same text in two functions, so fixing `value == None` in one function while writing it in another is one fixed and one new finding, never a move" |
| a record its own path reports fewer times than it accepts takes the unaccepted findings of the same content at another path | "That is code that moved, as a file renamed or code carried to another file; SonarQube's issue tracker matches a moved file's issues the same way." |
| a file moved with `mv` and not yet added is a rename | "moving a file with `mv` is a rename before `git add` says so" |
| perf is one more section of the same operations, one format, folded like the rest | the lock stays one immutable, content-addressed, merge-friendly store: a perf record is an operation like a finding, so `git` merges it and a descendant wins |
| a failed tool is unknown, never clean: exit 2 | "A tool that failed outranks anything found. CI must never read "could not determine" as "determined it is fine"" |
| the lock is folded afresh by each run; the store keeps only the check's runs | the store keeps what is shown or decides reuse; the merge model is unchanged |

Left out: `rekey` (a one-off migration from the retired rolling fingerprint), `record list` (no caller), the ruff JSON
reader (ruff writes SARIF itself). `asymptote.toml` says `output = "sarif"` for a SARIF tool and nothing for an
exit-status one.

## A check

1. Find the product: the nearest directory at or above the start holding `asymptote.toml`.
2. Run each tool's `fix` under `--fix`.
3. For each tool, stamp its inputs (`git ls-files` of its `inputs`, each file's BLAKE3 digest, the lock left out) and
   reuse its last completed run under the same stamp; otherwise run it, in the product's directory, in a process
   group of its own, held to its timeout and killed with everything in its group when SIGINT or SIGTERM ends the
   command, which then ends as the signal ends it; read its SARIF or its exit status. A SARIF log not of version 2.1.0, or a result with no rule, no physical location or a coordinate below
   1, is a failed tool: no finding is dropped unjudged.
4. Key each finding: the item fingerprint of its line (the item that encloses it: a function, a method, a type),
   the whole file's for a result with no region, or the tool's `primaryLocationLineHash` when the file cannot be read.
5. Fold the lock; under `--since`, fold the lock at the
   merge base and at the revision and unite them.
6. Carry the records through git's renames since the earliest operation that accepted one, and the working tree's.
7. Judge: a key seen more often than accepted is new by the surplus; a record short at its path takes the same content
   found unaccepted elsewhere; one seen less often is resolved when its tool completed
   (unless it lists its rules and leaves this one out, which retires the record), and unjudged when its tool did not run or failed.
8. Keep the report: each run under its stamp, and the digest of every file fingerprinted.

## up and down

Both read the report and refuse when the commit or `asymptote.toml` moved since, or any file the judgement rests on
did: every tool input, every file a finding is in, and every file a record it judged is of, so a fix checked clean
and then undone is never drained. They refuse as well when a tool failed. `up` records
the new findings its selectors choose, each record counting what the lock already accepts under its key plus what
is new, in one operation whose parents are the lock's heads. `down` drops every record no finding of which occurs
and lowers one some of whose findings still occur; a rename projection the run cannot carry is dropped from its old
path. Either then appends a snapshot of the lock below every head once it holds 64 operations since its last snapshot,
so snapshots stay one in 64 operations. `-n` (`--dry-run`) says on stderr what either would write and writes nothing.

`down` also records each new finding an in-source ignore suppresses, as a suppressed record whose reason is
`ignored in the source`: a finding fixed or ignored in the source with its reason is resolved.

`up` refuses a finding outside the workspace, `cannot accept a finding outside the workspace: <path>`: a path that is
absolute, climbs out of the product, or leads out of it through a link, which `check` names by where it leads.
