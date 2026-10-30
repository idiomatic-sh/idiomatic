# cases-not-empty 0.1.0

The first release. In test code, a `read_dir(…)` whose failure is swallowed (`into_iter`, `ok`, `flatten`,
`unwrap_or_default`, `unwrap_or`, `unwrap_or_else`, `let Ok(…) = … else`, `if let Ok(…) = …`) is reported: a fixture
directory that cannot be read never becomes zero cases. Case tables written in code are held by each crate's `cases::run`,
which refuses a case word with no row. The `idiomatic-rust` spec places it on the tests, benches and test support of every
product.
