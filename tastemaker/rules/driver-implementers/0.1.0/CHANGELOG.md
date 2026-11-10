# driver-implementers 0.1.0

A driver port, a trait of `ports` whose name matches `traits`, is implemented only in the files `implementers` lists, written by its name, a path or an import alias. It replaces the `DriversAreImplementedByStrategiesOnly` scenario of the execution operator service test, which scanned the engine's sources as text for `impl … Driver for`. Fixtures `the-strategies-implement-the-ports` (pass), `a-shortcut-implements-a-port` and `an-alias-does-not-escape` (violations).
