# Checkpoint evidence

## Validation set

`commands.json` is the list of `[name, argv]` validation commands (libraries,
boundary, ownership edges, and the rest) that the matrix's numbers rest on.
`run.py` takes the build locks, refuses a dirty tree, writes `identity.json`
(source revision plus the command list), runs each command in order, and writes
`<name>.log` and `<name>.json` (exit code, elapsed seconds) beside the set.
Run it from a clean checkout, from a copy of `commands.json` and `run.py` placed in the
output directory: `run.py` writes beside itself.

The matrix's numbers come from `validation.json` in this directory, which
records the exact sources and approved reuse of those runs; the raw logs stay
outside the repository. The set is committed with the matrix.
