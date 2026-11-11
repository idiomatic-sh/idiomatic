# benchmark-linter

## 0.2.2

Places `Cargo.toml`, the linter's own unpublished package: its `bench.rs` and `tests.rs` targets move out of the language provider's manifest, so the provider's published package carries no linter rule set or harness. Nothing else changes.

Edited in place: the module file of a directory is `<dir>.rs` beside it, never `<dir>/mod.rs` (owner, 2026-11-11; `mod_module_files` denied by cargo-workspace 0.13.0): `sharing.rs`, `sharing/fixtures.rs` and `frontend_projection.rs`; the test boundary reads them.

## 0.2.1

Places `sharing/fixtures/{part}.rs`, one file per concept of the inventory proof's fixtures, beside `sharing/fixtures/mod.rs`, which the 400-line limit otherwise forces to hold all of them. A nested directory or a file outside `sharing/fixtures/` is still refused. Nothing else changes.

## 0.2.0

`composition.rs`, `composition/*.rs`, `composition/constants.rs` and `worker_request_input_constants.rs` may import `ryo_benchmark_contracts::*` and the language's own `ryo_python::settings::*`: the contract types moved out of `ryo_benchmark` into `ryo_benchmark_contracts`, which 0.1.0 predates. Shared application policy (`ryo_benchmark::application::*`) stays refused. 0.1.0 stays as published.
