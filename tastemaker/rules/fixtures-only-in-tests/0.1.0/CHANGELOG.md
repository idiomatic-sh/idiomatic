# fixtures-only-in-tests

## 0.1.0

An item gated by `cfg(any(test, feature = "fixtures"))` is named only by test code (test and fixtures paths, files declared under a test or `fixtures` cfg, items gated by one): a production call of a gated method in any package, or a production path naming a gated function, constant or type of its own package, is a finding. A name also declared outside the gate is left alone.
