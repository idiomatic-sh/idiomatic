# no-path-exists 0.1.0

The first release. It reports every `exists()` and `try_exists()` with no argument, and every `Path::exists`,
`PathBuf::exists` and `fs::exists` (and their `try_exists`) called or passed: absence is the `ErrorKind::NotFound` of the
operation itself, never a separate question whose `false` also means "cannot tell". The `idiomatic-rust` spec places it on
production code; tests and their data are left out.
