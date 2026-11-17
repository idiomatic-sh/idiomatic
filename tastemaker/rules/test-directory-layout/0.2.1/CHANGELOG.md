# test-directory-layout 0.2.1

Every join of the query now plans under the engine's growth check, which refuses a join keyed on the file alone unless one side is proven to hold each file once. The `#[path]` metadata is read from the proof files, a projection on `file` (one row per file), where 0.2.0 joined the union of proofs itself. The findings and the scenarios are as in 0.2.0, which is removed with this version: no lock reads it.

# test-directory-layout 0.2.0

A group's module file is `<group>.rs` beside its directory `<group>/`, the one module-file style (owner, 2026-11-11; `mod_module_files` denied by cargo-workspace 0.13.0). 0.1.0 accepted only `mod.rs`, `test.rs` and `fixtures.rs`, so it refused every `use_case_test/<group>.rs` that use-case 0.1.3 places, and its message and help said to use `mod.rs`. A `<name>.rs` counts as a module file only where the directory `<name>/` exists beside it; any other concept-named file is still refused. Scenarios `registered-directory-proof` (pass, in the `<dir>.rs` form) and `module-file-without-its-directory-refused`. 0.1.0 stays as published: tastemaker locks it.
