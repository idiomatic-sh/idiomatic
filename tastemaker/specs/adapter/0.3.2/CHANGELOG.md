# adapter 0.3.2

Places `no-runtime-fixtures@0.2.0` and `public-fields@0.3.0` where 0.3.1 named their earlier versions, so a product that places it locks one version of each rule: two versions of one rule share its id, and the earlier entries ran the later query. Nothing else changes: `adapter_test.rs` is the spec-local `adapter-test` template, which holds the contract import and the `CASES` table, and an adapter places no nested part and no group of cases; scenario `adapter-with-a-nested-part` holds a nested part's files closed. A part too long for one file is split by the adapter's own roles (`types.rs`, `errors.rs`, `schemas.rs`, `{document}/schemas.rs`). 0.3.1 is removed with this version, not kept as published: no lock names it, nothing has been released.

Edited in place: the module file of a directory is `<dir>.rs` beside it, never `<dir>/mod.rs` (owner, 2026-11-11; `mod_module_files` denied by cargo-workspace 0.13.0); scenario `adapter-with-a-nested-part` names `decode.rs`, which the adapter reads as a document module.

Edited in place: places `unit-cases@0.2.0`, which reads a namespace `*_test.rs` with the tables of its groups.

# adapter 0.3.1

Places the corrected rules (`public-fields@0.2.0`, `constants-once@0.2.0`, `mirrored-types@0.2.0`, `no-json-in-process@0.4.0`) and the specs that place them at their corrected versions, where 0.3.0 named the earlier ones. Nothing else changes.
