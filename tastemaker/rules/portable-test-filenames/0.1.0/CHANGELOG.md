# portable-test-filenames 0.1.0

The first release. A test function that creates files names none with a character Windows refuses (`?`, `<`, `>`, `|`, or a
`*` inside a name), unless it is decorated `#[cfg(unix)]` or `#[cfg(not(windows))]`. The `idiomatic-rust` spec places it on
the tests, benches and test support of every product.
