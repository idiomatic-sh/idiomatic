# shared-walk-owner

## 0.3.0

The execution crates' module files moved from `<x>.rs` to `<x>/mod.rs` (carve-out batch A), so every path this rule names for a moved module file, in its query and in its scenarios' trees, names `<x>/mod.rs`; every scenario plants its violation at the new path. Nothing else changes. Module files are named for their directory (`<x>.rs`, never `<x>/mod.rs`), so every module-file path the query and the scenarios name is `<x>.rs`.
