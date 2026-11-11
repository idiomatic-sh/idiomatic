# cargo-workspace

## 0.13.0

`pub_use` is no longer denied: re-export policy is owned by Tastemaker's `idiomatic.arch/no-reexports` rule, which allows the
one-item seam forwards and the port-factory resource-handle export and refuses a re-export outside an SDK surface, so clippy
holds no second, blunter definition of it. Every scenario is unchanged apart from the line.

`mod_module_files` is denied, edited in place: a directory's module file is `<dir>.rs` beside the directory, never `<dir>/mod.rs`, one style for every crate (owner, 2026-11-11), held by the compiler, the cheapest tool for it. The description no longer says Tastemaker's placements own the choice: no placement names `mod.rs`. Scenario `no-mod-module-files-deny`; every other scenario carries the line.
