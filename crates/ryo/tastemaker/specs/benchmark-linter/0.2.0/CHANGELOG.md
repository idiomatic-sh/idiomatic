# benchmark-linter

## 0.2.0

`composition.rs`, `composition/*.rs`, `composition/constants.rs` and `worker_request_input_constants.rs` may import `ryo_benchmark_contracts::*` and the language's own `ryo_python::settings::*`: the contract types moved out of `ryo_benchmark` into `ryo_benchmark_contracts`, which 0.1.0 predates. Shared application policy (`ryo_benchmark::application::*`) stays refused. 0.1.0 stays as published.
