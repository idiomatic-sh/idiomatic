# Checkpoint evidence

## Validation set

`commands.json` is the list of `[name, argv]` validation commands the matrix's
numbers rest on: library tests, `boundary_contract`, `project_boundary`,
`activation`, `bdd`, `core_bdd`, `core_allocation`, the execution program
authority targets, the doctests (execution, values, core, sdk, derived),
`ruff_sharing` and `ruff` (the python target), the ten language `conformance`
targets, the command line's `cli_tests` (`replay`, `blocks`, `call_count`,
`command_line`, `command_tree`, `readme`), `bench_build` (the command line's and
the core's benches compile), `no_print` (the two print lints over the execution
and core libraries), the Tastemaker checks (`tm_import_cli`, `tm_cli_source`,
`tm_source` over the scoped directories, `tm_guards`, and
`tm_validate_no_heap_accounting`, the `no-heap-accounting@0.2.0` validation) and `ownership_edges`.

At checkpoint 318 two commands are red by design and the rest are green:
`core_allocation` (the Phase 7 measurement, bead ryo-l3mj) and `cli_tests`,
whose `replay` target has six red tests, each named in `validation.json`
(`results.cli_replay_reds`): the four `(b)` behaviour tests and the cache store
size and warm peak tests, which wait for the coverage missing-kinds field
(beads ryo-xqxb and ryo-7fqz). `tm_source` exits 1 because it reports its
findings; `run.py` records its total, its suppressed count and the production/test
split per scoped directory in `tm_source.json` (`findings`), and `validation.json`
holds the same for the validation318 run. The whole-tree `tm check --root crates/ryo` count, its suppressed
count and the file-level ownership cycle count (computed from
`ownership_edges.log`) are recorded in `validation.json` beside the commands.

`run.py` takes the build locks, refuses a dirty tree, writes `identity.json`
(source revision plus the command list) beside itself, runs each command in
order, and writes `<name>.log` and `<name>.json` (exit code, elapsed seconds) to
the output directory. That directory is the first argument; the default is
`/tmp/ryo-contract-validation-<source head>-<date>`. Raw outputs are never
committed; `validation.json` and `identity.json` are.

The matrix's numbers come from `validation.json`, which records the exact
sources and approved reuse of those runs.
