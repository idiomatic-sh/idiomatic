# runtime-authority

## 0.3.0

The execution crates' module files moved from `<x>.rs` to `<x>/mod.rs` (carve-out batch A), so every path this rule names for a moved module file, in its query and in its scenarios' trees, names `<x>/mod.rs`; every scenario plants its violation at the new path. Nothing else changes. Module files are named for their directory (`<x>.rs`, never `<x>/mod.rs`), so every module-file path the query and the scenarios name is `<x>.rs`. The admitted input's sealed owner is `program/admission/service.rs`, where the part-namespace split placed it; the query and the scenarios name that file.

## 0.2.0

The declaration check also holds the records of other owners named in the new required `owner-records` param: each `{ record, visibility }` gives the widest visibility a field of that struct may be written with, `private` for none. Placed for the derived owner's records (`AdmittedViewInputs`, `GraphHandle`, `DomainHandle`, `DerivedStorage`, `RequestDerived` private; `CacheBackend` at the owner's own `pub(in …::derived::owner)`), the same-crate leftover of the derived-owner source-overlay probes. Fixtures `owner-records-keep-their-visibility` (pass), `owner-record-field-crate-is-not-private` and `owner-record-field-cannot-widen-past-its-owner` (violations). Everything else is as in 0.1.0.
