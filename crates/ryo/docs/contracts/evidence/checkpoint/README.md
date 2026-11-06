# Checkpoint evidence

## Validation set

`commands.json` is the list of `[name, argv]` validation commands (libraries,
boundary, ownership edges, and the rest) that the matrix's numbers rest on.
`run.py` takes the build locks, refuses a dirty tree, writes `identity.json`
(source revision plus the command list), runs each command in order, and writes
`<name>.log` and `<name>.json` (exit code, elapsed seconds) beside the set.
Run it from a clean checkout; the output directory is set by `p` at the top of
`run.py` (the committed copy points at `/tmp/ryo-contract-validation310-20261106`).

The matrix's numbers come from `validation.json` in this directory, which
records the exact sources and approved reuse of those runs; the raw logs stay
outside the repository. The set is committed under bead ryo-607o.
