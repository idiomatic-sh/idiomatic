# item-fixtures

## 0.1.0

Each relation and operator item has `tests/fixtures/<collection>/<item>/` with an `input` file and an `expected.rs`. The rule counts files and reads none of them: an `input` with an extension and an `input` without one both count (fixtures `bare-input`, `empty-files`).

The input of a `relations` item is no longer counted here: the workspace keeps rules off the source of a syntax error, so the rule could not see it, and the `language` spec now places each relation fixture's input beside its expected rows (paired, so either alone is a finding). An `operators` item still needs its input. Fixtures `relation-input-held-by-placement`, `relation-without-expected`.
