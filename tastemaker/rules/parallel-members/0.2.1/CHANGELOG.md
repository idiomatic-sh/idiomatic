# parallel-members 0.2.1

Every join of the query now plans under the engine's growth check, which refuses a join keyed on the file alone unless one side is proven to hold each file once. A pair is kept when its member, or its counterpart, is among the dependent paths, a projection on `path` (one row per file), where 0.2.0 joined the dependent rows themselves. The findings and the scenarios are as in 0.2.0.

# parallel-members 0.2.0

A file beneath a test role is no member: a test group (anything under a `*_test/` directory, a `test.rs`), a fixture (`fixtures/`, `fixtures.rs`) and anything under `tests/`. 0.1.0 counted them, so a use case's test groups named after the domain concepts they prove (`use_case_test/<concept>.rs` beside `use_case_test/<concept>/`, the module-file form since 2026-11-11) read as one concept split by role. Scenario `test-groups-named-like-domain-concepts` (no finding); `split-concept` is still reported. Everything else is as in 0.1.0, which stays as published: tastemaker and asymptote lock it.
