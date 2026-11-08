# fixtures-only-in-tests

## 0.2.0

A method declared without the gate in a test or fixtures file (a test witness implementing the gated trait method) no longer unseals the gated method: only an ungated declaration of the same name in a production file does. A production call of `open_file` with such a witness present is a finding.

## 0.1.0

An item gated by `cfg(any(test, feature = "fixtures"))` is named only by test code (test and fixtures paths, files declared under a test or `fixtures` cfg, items gated by one): a production call of a gated method in any package, or a production path naming a gated function, constant or type of its own package, is a finding. A name also declared outside the gate is left alone.
