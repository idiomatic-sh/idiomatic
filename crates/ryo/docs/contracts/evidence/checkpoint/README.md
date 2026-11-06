# Checkpoint evidence

## Validation set

`commands.json` is the list of `[name, argv]` validation commands the matrix's
numbers rest on: library tests, `boundary_contract`, `project_boundary`,
`activation`, `bdd`, `core_bdd`, `core_allocation`, the execution program
authority targets, the doctests (execution, values, core, sdk, derived),
`ruff_sharing` and `ruff` (the python target), the Tastemaker checks
(`tm_import_cli`, `tm_cli_source`, `tm_source`, `tm_guards`) and `ownership_edges`.

`run.py` takes the build locks, refuses a dirty tree, writes `identity.json`
(source revision plus the command list) beside itself, runs each command in
order, and writes `<name>.log` and `<name>.json` (exit code, elapsed seconds) to
the output directory. That directory is the first argument; the default is
`/tmp/ryo-contract-validation-<source head>-<date>`. Raw outputs are never
committed; `validation.json` and `identity.json` are.

The matrix's numbers come from `validation.json`, which records the exact
sources and approved reuse of those runs.
