# no-silent-skip 0.1.0

The first release. In test code, an `if` branch or a `let … else` block that prints and then goes on with `continue`, or
with a `return` that carries no error and no panic, is reported: a case the test cannot run is a failure, never a line on
stderr. The `idiomatic-rust` spec places it on the tests, benches and test support of every product.
