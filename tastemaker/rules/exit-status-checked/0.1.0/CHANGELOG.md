# exit-status-checked 0.1.0

The first release. In test code, every `output()` and `status()` with no argument has its exit read: through
`success()`, `code()` or `.status`, directly or by the name it is bound to (in code or in an assertion's tokens), or it is
handed back to the caller. A run that is dropped, bound and never checked, or read only for its stdout or for whether it
started is reported, which also catches a test that passes over a tool that is not installed. The `idiomatic-rust` spec
places it on the tests, benches and test support of every product.
