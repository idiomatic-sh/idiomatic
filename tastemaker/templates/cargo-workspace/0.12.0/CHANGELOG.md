# cargo-workspace

## 0.12.0

`mod_module_files` is no longer denied: file layout is owned by Tastemaker's placement rules (`test-directory-layout` and the
templates that place `mod.rs` files), so clippy holds no second definition of it. Clippy's renames of those files had broken
69 placements. The scenario 'Mod.rs files are not denied.' is gone with the deny; every other scenario is unchanged apart from
the line.
