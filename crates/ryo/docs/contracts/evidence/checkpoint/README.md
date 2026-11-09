# Checkpoint evidence

## Validation set

`asymptote.json` is the integrator's record of the quick checks for the landed sha, in the [review evidence schema](../../review/README.md#review-evidence-schema) with `reviewer` `integrator` and one check per group (`lint`, `check`, `clippy`); the TM steward reads only this file.

`commands.json` is the list of `[name, argv]` validation commands the matrix's
numbers rest on: library tests, `boundary_contract`, `project_boundary`,
`activation`, `bdd`, `core_bdd`, `core_allocation`, the execution program
authority targets, the doctests (execution, its domain, store and runtime, values, core, sdk, derived),
`ruff` (the python target), the ten language `conformance`
targets, the command line's `cli_tests` (`replay`, `blocks`, `call_count`,
`command_line`, `command_tree`, `readme`), `bench_build` (the command line's and
the core's benches compile), `no_print` (the two print lints over the execution,
execution domain, execution store, execution runtime and core libraries), the Tastemaker checks (`tm_import_cli`, `tm_cli_source`,
`tm_source` over the scoped directories, `tm_guards`, and
`tm_validate_no_heap_accounting`, the `no-heap-accounting@0.2.0` validation), `ownership_edges`, `checkpoint_check` (the fixture cases of `run.py --check` in `run_test.py`) and `census` (`run.py --census`, the change-amplification counts below).

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
`validation.json` and exits 1 when the file changed (stage it and run again),
so a stale block cannot pass. Both modes also exit 1 when a ticked row (`☑`,
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

## Change-amplification census
`run.py --census` reads the product's tracked Rust files (no build) and prints four counts, each as files and crates; the `census` command's log is recorded as `census` in `validation.json`, and the numbers block shows it. A production file is under `crates/*/src`, `cli/src` or `languages/*/src` and is no test file by `is_test_file`, the one classifier `run.py` uses for tm splits, `FIX`, `DOC` and the census; each `#[cfg(test)]` item of a production file (its own extent) counts as test.
- `replace_the_evaluator`: files outside `crates/execution_runtime` that name the evaluator's vocabulary (every `pub` struct, enum, trait or type it declares; a name counts unless the file's own crate or the file declares it, every `use` that brings it in starts at another crate than `ryo_execution_runtime` or `ryo_execution` or at this one, or it follows `Type::` as a variant) or `ryo_execution_runtime`, production and test apart; a file naming only the seam's ports (`DigestPort`, `DigestBytes`, `FactsPort`, `OperatorsPort`, `FactsError`) is not counted.
- `operation_exhaustive_matches`: production files naming eight or more distinct `Operation` variants, through `Operation::`, an alias (`use ...Operation as X`, `type X = ...Operation;`) or a glob import of its variants.
- `per_file_fact_touch_points`: production files naming the per-file fact precedent, `binding_read` in any spelling.
- `cross_crate_tests_naming_runtime_internals`: test files outside the runtime and store crates naming a runtime internal type (`FileRunInput`, `FileSession`, `RelationOwner`, `InvocationWorkspace` and the rest of `RUNTIME_INTERNALS`).

The contract review's isolation grade reads these numbers against the previous validation's.

## Checkpoint diff checks
Both modes check the checkpoint's own commits, from the merge base of `HEAD` and `origin/claude/ryo-consolidation`, and exit 1 with one line per problem:
- `FIX`: a commit that changes production code (a Rust source, or the Python of the harness: `.claude/hooks/`, `ci/` and this directory, none of them a test, fixture or bench) is a fix whatever its type; a merge counts by its own changes (its combined diff against every parent). A fix names a test in its message, and a `red_green` entry whose range holds it (its `base_sha` excluded, its `merge_sha` included) proves that test: the entry's `test` names it with `(red <name>.log, green <name>.log)`, two distinct files in the validation directory, each framed by its test runner's own header and summary lines (cargo test `running N tests` and `test result:`, nextest `Starting N tests across` and `Summary [`, unittest `Ran N tests in` and `OK`/`FAILED (`), the red log showing the test fail and the green log showing it pass. The frame catches a typed result, not a forged one. A `perf` commit instead needs an `evidence/perf/*/after.json` added or changed in the checkpoint with `identical_output` true.
- `DOC`: each `pub` function, struct, enum, trait, type or union a published crate (no `publish = false`) gains has an `# Examples` section holding a fenced code block, and a function returning a `Result` an `# Errors` section. An item is new when no item of the same module path (from its file), owner (its impl type, module or trait), kind and name is `pub` in that crate's production files at the base; renames count as additions. Test files (the one `is_test_file`: `tests/`, `test.rs`, `tests.rs`, `*_test.rs`, `*_tests.rs`, fixtures, benches, examples), `fixtures.rs` and a `Sealed` seal trait are exempt.

Without a base (the consolidation branch is not fetched), both checks refuse.

## Red-then-green record
For each fix (a change to production behaviour) the integrator appends to the `red_green` list in [validation.json](validation.json) an entry `{test, base_sha, merge_sha}`: the test id the builder named followed by its red and green logs (`(red <name>.log, green <name>.log)`, files in the validation directory), the 40-hex base sha where it fails (red) and the 40-hex merge sha where it passes (green); the fix commit lies between the two shas. `run.py --check` refuses an entry of any other shape. Docs-only, evidence-only and perf changes are exempt. See the [Builder test rule](../../review/README.md#builder-test-rule).
