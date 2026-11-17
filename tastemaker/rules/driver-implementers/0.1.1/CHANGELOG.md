# driver-implementers 0.1.1

Every join of the query now plans under the engine's growth check, which refuses a join keyed on the file alone unless one side is proven to hold each file once. An implementation is matched to an import of the same file by the imported binding as well as the file, as one key, where 0.1.0 joined every import of the file and then compared the binding. The findings and the scenarios are as in 0.1.0.

# driver-implementers 0.1.0

A driver port, a trait of `ports` whose name matches `traits`, is implemented only in the files `implementers` lists, written by its name, a path or an import alias. It replaces the `DriversAreImplementedByStrategiesOnly` scenario of the execution operator service test, which scanned the engine's sources as text for `impl … Driver for`. Fixtures `the-strategies-implement-the-ports` (pass), `a-shortcut-implements-a-port` and `an-alias-does-not-escape` (violations).
