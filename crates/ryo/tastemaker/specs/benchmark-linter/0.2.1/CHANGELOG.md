# benchmark-linter

## 0.2.1

Places `sharing/fixtures/{part}.rs`, one file per concept of the inventory proof's fixtures, beside `sharing/fixtures/mod.rs`, which the 400-line limit otherwise forces to hold all of them. A nested directory or a file outside `sharing/fixtures/` is still refused. Nothing else changes.

## 0.2.0

`composition.rs`, `composition/*.rs`, `composition/constants.rs` and `worker_request_input_constants.rs` may import `ryo_benchmark_contracts::*` and the language's own `ryo_python::settings::*`: the contract types moved out of `ryo_benchmark` into `ryo_benchmark_contracts`, which 0.1.0 predates. Shared application policy (`ryo_benchmark::application::*`) stays refused. 0.1.0 stays as published.
