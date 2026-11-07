# operator-meaning-once 0.1.0

An operation kind's meaning is implemented once, in its operators: an engine file names a variant of the operators' call only when `engine-calls` lists it, and no module under `modules` is named after a kind, read from the macro call of the `kinds` files. It replaces the `NoSecondImplementation` scenario of the execution operator service test, which scanned the engine's sources as text. Fixtures `the-engine-makes-its-calls` and `a-module-elsewhere-may-share-a-name` (pass), `the-engine-matches-a-kind-of-its-own` and `a-module-named-after-a-kind` (violations).
