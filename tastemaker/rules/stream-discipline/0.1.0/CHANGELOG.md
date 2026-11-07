# stream-discipline 0.1.0

The first version. A `tracing_chrome::ChromeLayerBuilder` built in a file that names no `file` is a finding: left to
its default, it writes `./trace-<time>.json` into the working directory, the tree the command reads. The trace is
written to a path the presentation layer gives it instead. The standard streams are held by `stdio-at-presentation`,
`print-in-library` and clippy's print lints.

## Fixtures

`trace-in-the-working-directory` (failing), `trace-to-a-given-file` (passing).
