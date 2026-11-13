# operator-meaning-once 0.2.0

Adds the function arm and its param `functions`: a free function, or an associated function of an inherent `impl`, declared in a file under `functions` and named after a kind is a finding. A method of a trait, declared in the trait or written in an `impl` of it, is not read: its name is the trait's (`FactsWorker::rows`). `engine`, `modules`, `calls`, `engine-calls` and `kinds` are unchanged. Fixtures `a-kind-named-function-in-the-pass`, `an-inherent-function-named-after-a-kind` and `the-spike-batch-operators` (violations: the fused-evaluation spike's `batch/service.rs`, nine kinds), `trait-methods-are-the-traits`, `a-function-outside-the-pass-may-share-a-name` and `test-helpers-are-left-out-by-the-entry` (pass, the last through the entry's `exclude`).

# operator-meaning-once 0.1.0

An operation kind's meaning is implemented once, in its operators: an engine file names a variant of the operators' call only when `engine-calls` lists it, and no module under `modules` is named after a kind, read from the macro call of the `kinds` files. It replaces the `NoSecondImplementation` scenario of the execution operator service test, which scanned the engine's sources as text. Fixtures `the-engine-makes-its-calls` and `a-module-elsewhere-may-share-a-name` (pass), `the-engine-matches-a-kind-of-its-own` and `a-module-named-after-a-kind` (violations).
