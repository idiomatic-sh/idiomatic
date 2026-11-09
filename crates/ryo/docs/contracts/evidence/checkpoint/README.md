# Checkpoint evidence

## Validation set

`asymptote.json` is the integrator's record of the quick checks for the landed sha, in the [review evidence schema](../../review/README.md#review-evidence-schema) with `reviewer` `integrator` and one check per group (`lint`, `check`, `clippy`); the TM steward reads only this file.

`commands.json` is the list of `[name, argv]` validation commands the matrix's
numbers rest on: `checkout_tools` (this checkout's `tm` and `ryo`, built into the run's target directory, which the commands over
`crates/ryo`, pinned to the unreleased Tastemaker 0.1.2, and `ownership_edges` run; the root and `crates/tastemaker`, pinned to
0.1.0, keep the installed `tm`), library tests, `boundary_contract`, `project_boundary`,
`activation`, `bdd`, `core_bdd`, `core_allocation`, the execution program
authority targets, the doctests (execution, its domain, store and runtime, values, core, sdk, derived),
`ruff` (the python target), the ten language `conformance`
targets, the command line's `cli_tests` (`replay`, `blocks`, `call_count`,
`command_line`, `command_tree`, `readme`), `bench_build` (the command line's and
the core's benches compile), `no_print` (the two print lints over the execution,
execution domain, execution store, execution runtime and core libraries), the Tastemaker checks (`tm_import_cli`, `tm_cli_source`,
`tm_source` over the scoped directories, `tm_guards`, and
`tm_validate_no_heap_accounting`, the `no-heap-accounting@0.2.0` validation), `ownership_edges` and `checkpoint_check` (the fixture cases of `run.py --check` in `run_test.py`).

At checkpoint 318 two commands are red by design and the rest are green:
`core_allocation` (the Phase 7 measurement) and `cli_tests`,
whose `replay` target has six red tests, each named in `validation.json`
(`results.cli_replay_reds`): the four `(b)` behaviour tests and the cache store
size and warm peak tests, which wait for the coverage missing-kinds field. `tm_source` exits 1 because it reports its
findings; `run.py` records its total, its suppressed count and the production/test
split per scoped directory in `tm_source.json` (`findings`), and `validation.json`
holds the same for the validation318 run. The whole-tree `tm check --root crates/ryo` count, its suppressed
count and the file-level ownership cycle count (computed from
`ownership_edges.log`) are recorded in `validation.json` beside the commands.

`run.py` takes the build locks, refuses a dirty tree, replaces `{worktree}` in
each command's target directory (`idiomatic-target/{worktree}` in the workspace
mount) with the worktree's name (one cargo target per worktree, so no command
runs units another tree built), writes `identity.json`
(source revision plus the expanded command list) beside itself, runs each command in
order, and writes `<name>.log` and `<name>.json` (exit code, elapsed seconds) to
the output directory. That directory is the first argument; the default is a
directory named for the source head and the date under the system temporary directory. Raw outputs are never
committed; `validation.json` and `identity.json` are.

The matrix's numbers come from `validation.json`, which records the exact
sources and approved reuse of those runs.

On GitHub Actions the checkpoint workflow (`.github/workflows/checkpoint.yml`), started by hand on a ref, runs `run.py` through the
root `asymptote.toml`'s `checkpoint` tool with `target/checks/checkpoint` as the output directory, and uploads `validation.json`,
`identity.json`, `tests.json` and every command's `.log` and `.json` as the `checkpoint-validation` artifact.

## What `run.py` writes and checks

After the commands, `run.py` raises `validation` by one, records the source and
the raw directory, and writes `tm_targets` into `validation.json`: for each
`tm check` command (`tm_source`, `tm_cli_source`) the finding count, the
suppressed count, the production/test split (a path containing `/test` or
`_test.rs` counts as test) and `scoped_directories`, each scoped path with its
finding count. It writes `tests.json` beside `validation.json` (which keeps its line budget): for each
cargo test command, per test binary, the passed and ignored tests and each failed
test as `[collected or aborted, red row...]`, the case rows its runner names
(`collected` when the runner ran every row of the word and named every red one). It then rewrites the matrix's "Current ownership checkpoint"
block (heading to the first bullet) from `validation.json`: source, validation
number, per-target pass/fail counts, TM findings and suppressed, cycles. Nobody
types those cells.

`python3 crates/ryo/docs/contracts/evidence/checkpoint/run.py --check` runs no
cargo and takes no lock. It rewrites the block from the committed
`validation.json`, and when a file changed it prints `STALE RECORD` with the files and exits 1 (stage them and run again),
so a stale block cannot pass. A full run rewrites the block every time (its validation number is new), which is not a failure:
it exits on the validation's results alone.

Both modes exit 1 for a red command no README row owns, and print every red command with its owning rows: its own `OWNERS` row,
or the rows of every red test it recorded (a test by its name, a library test binary `ryo_<x>` by `<x>_library`). A command that
cannot run on the machine prints `not available: <why>` as its first line and exits non-zero (`ruff_parity`, whose pinned Django
request stays outside the repository); `run.py` records the reason in its check, and only that refusal is owned, by its
`UNAVAILABLE_OWNERS` row. Both modes also exit 1 when a ticked row (`☑`,
`[x]`) of the matrix cites no validation (`validationNNN`, `proofNNN`,
`reviewNNN`, `passNNN`, `runtimeNNN`, `reuseNNN`, `inNNN`, `atNNN`) or cites one
greater than `validation`; each offending row is printed with its line number.
Both modes un-tick (and print) every ticked row that cites a test, a test module
or a case row (a backticked lowercase `::` path through a test module or ending
in a sentence-like name, or a bare sentence-like name) that `tests.json` records
red, as not run (its table stopped at an earlier red row) or not at all, and exit 1
until the un-ticked file is staged. A ticked row or a result note that cites a
narrow run or a rerun must name its raw `.log`, present in the validation
directory, or the check exits 1. Unticked rows are not checked. The block lists
each red test with its red rows.

Both modes print the scope delta against `validation.json` at `HEAD`: each
newly scoped directory with its finding count, and each directory no longer
scoped. The checkpoint commit message carries these lines.

## Red-then-green record
For each fix (a change to production behaviour) the integrator appends to the `red_green` list in [validation.json](validation.json) an entry `{test, base_sha, merge_sha}`: the test id the builder named, the 40-hex base sha where it fails (red) and the 40-hex merge sha where it passes (green). `run.py --check` refuses an entry of any other shape. Docs-only, evidence-only and perf changes are exempt. See the [Builder test rule](../../review/README.md#builder-test-rule).
