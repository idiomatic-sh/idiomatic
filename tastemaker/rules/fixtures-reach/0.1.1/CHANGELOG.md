# fixtures-reach

## 0.1.1

Every join of the query now plans under the engine's growth check, which refuses a join keyed on the file alone unless one side is proven to hold each file once. A fixtures mention is kept when its file is among the gated files, grouped by file (one row per file), where 0.1.0 joined the gated rows themselves. The findings and the scenarios are as in 0.1.0.

## 0.1.0

A file the `fixtures` feature builds (`fixtures.rs`, `fixtures/**`, or a module declared under a `cfg` naming the feature, without `#![cfg(test)]`) names another crate's `fixtures` only when the crate's `[features] fixtures` enables `<dep>/fixtures`.
