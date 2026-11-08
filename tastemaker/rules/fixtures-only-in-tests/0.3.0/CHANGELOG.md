# fixtures-only-in-tests

## 0.3.0

A gated method called by its path (`Trait::method(&value, ..)`, the 324 review's note) is a finding as a method call is, under the same seal: matched by the path's last segment when the path's qualifier ends in a capitalised name (a type or a trait), so `crate::open_file` and `module::open_file` stay functions. Scenarios `production-calls-a-gated-method-by-its-path` (violation, beside a production function of the same name) and `a-path-named-like-a-production-function` (pass); the 0.2.0 scenarios are kept.

## 0.2.0

A method declared without the gate in a test or fixtures file (a test witness implementing the gated trait method) no longer unseals the gated method: only an ungated declaration of the same name in a production file does. A production call of `open_file` with such a witness present is a finding.

## 0.1.0

An item gated by `cfg(any(test, feature = "fixtures"))` is named only by test code (test and fixtures paths, files declared under a test or `fixtures` cfg, items gated by one): a production call of a gated method in any package, or a production path naming a gated function, constant or type of its own package, is a finding. A name also declared outside the gate is left alone.
