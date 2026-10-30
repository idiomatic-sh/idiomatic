# smoke-tests-run-no-targets 0.1.0

The first release. In the distribution test target's smoke tests (`smoke_test.rs` at its root and in each registry's
directory), no expression names `target_test`: a smoke test proves the install, and the targets are the bench's, which
runs each one and holds every round to its expected findings. The `distribution` spec places it on every product.
