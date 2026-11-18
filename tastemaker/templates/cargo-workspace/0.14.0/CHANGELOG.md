# cargo-workspace

## 0.14.0

`ignore`, `sysinfo` and `cpu-time` leave the workspace's dependencies, and `[profile.test.package.sysinfo]` the profiles: they served the distribution benches and the `commands` bench every product carried, which distribution 0.4.0 and cli 0.13.0 delete; Cargo refuses a workspace dependency no member uses. A product whose own crates use them (tastemaker's memory bench) keeps declaring them; the template holds what every product needs.

### Moving up from 0.13.0

- Delete `ignore`, `sysinfo` and `cpu-time` from `[workspace.dependencies]` where no member uses them, and `[profile.test.package.sysinfo]` where nothing builds `sysinfo`.

## 0.13.0

`pub_use` is no longer denied: re-export policy is owned by Tastemaker's `idiomatic.arch/no-reexports` rule, which allows the
one-item seam forwards and the port-factory resource-handle export and refuses a re-export outside an SDK surface, so clippy
holds no second, blunter definition of it. Every scenario is unchanged apart from the line.

`mod_module_files` is denied, edited in place: a directory's module file is `<dir>.rs` beside the directory, never `<dir>/mod.rs`, one style for every crate (owner, 2026-11-11), held by the compiler, the cheapest tool for it. The description no longer says Tastemaker's placements own the choice: no placement names `mod.rs`. Scenario `no-mod-module-files-deny`; every other scenario carries the line.
