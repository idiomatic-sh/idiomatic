# fixtures-reach

## 0.1.0

A file the `fixtures` feature builds (`fixtures.rs`, `fixtures/**`, or a module declared under a `cfg` naming the feature, without `#![cfg(test)]`) names another crate's `fixtures` only when the crate's `[features] fixtures` enables `<dep>/fixtures`.
