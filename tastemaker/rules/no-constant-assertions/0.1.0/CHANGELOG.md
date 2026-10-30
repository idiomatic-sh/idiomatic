# no-constant-assertions 0.1.0

The first release in `idiomatic.arch`. It was the benchmark rule `constant-assertions` of `idiomatic.benchmarks`, kept to
measure the engine and placed on no product; it moves here with the query and scenarios it had (Rust and Python), and the `idiomatic-rust`
spec places it on every product: no `assert!` or `debug_assert!` whose condition is the literal `true` or `false`.
