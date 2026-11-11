# program-implementation-boundary

## 0.4.0

A test double of a guarded contract (`KernelInvocation`, `Inputs`, `PrimitiveExecutor`, `PrimitiveSemantics`, an operator or representation item) is accepted in a fixtures role as well as in a proof-placed file: a `fixtures.rs` or a file beneath `fixtures/` whose module, or a module above it, is declared under a `cfg` naming `test` or the `fixtures` feature. A production implementation outside the registered set is still refused, and so is the same double in a fixtures file declared without the gate, so a test double needs no per-file `program-runtime-proof` placement. Scenarios `domain-double-is-production` and `ungated-fixtures-double` (refused), `gated-fixtures-double` and `gated-group-fixtures-double` (pass). Everything else is as in 0.3.0.

## 0.3.0

The execution crates' module files moved from `<x>.rs` to `<x>/mod.rs` (carve-out batch A), so every path this rule names for a moved module file, in its query and in its scenarios' trees, names `<x>/mod.rs`; every scenario plants its violation at the new path. The derived owner's cache parts are read in both forms, `<part>.rs` and `<part>/mod.rs` (two captures and their union), since `leases` moved and `acquisition` was already a directory module; the scenario tree places `acquisition/mod.rs`, so `peer-cannot-acquire-cache` plants its violation through the directory form. Nothing else changes.

## 0.2.0

The Rust provider's source-dependency keeps every `use` alias (73d5a6bb55), so an aliased module or type gives two edges on its line, the import and the use through the alias: the scenarios `neutral-protocol-cannot-hold-runtime-owner`, `preparation-cannot-call-file-workflow`, `source-proofs-cannot-validate-interpreter-object` and `runtime-types-cannot-import-runtime-interpreter` expect two findings on line 1. The query is unchanged.

## 0.1.0

Edited in place: the interface and types of each handle concept (and the outer service of a prepared program and of a request execution) are outer entries beside preparation and the outcomes; `protocol` is dissolved into `rule_outcome` and `observed_publication`; the fixtures state an accepted caller and the service of a handle concept that stays protected.
