# benchmark-linter

## 0.2.5

A linter may hold one `cost_test.rs`: the pack's cost check, which compiles each rule's query with the query language and holds its cost against the `[cost]` its manifest declares (the ceiling, and each statement above linear with sorts). It is the one file of a linter that may name the query language: the SDK, which every other proof of a linter goes through, does not carry a query's cost. Unreleased, joined with the 0.2.5 below.

The spec's reason says what the shared application publishes: PASS, FAIL and DEFERRED, where a default the profile defers under a named capability, with no authored query, is DEFERRED and runs nothing, and a run passes when no row is FAIL. It said PASS/FAIL. This joins the 0.2.4 below with the reason change a 0.2.4 made beside it (both unreleased); nothing else changes.

## 0.2.4

The Ruff bench moves its Python implementation into the shared harness (ryo-b1gq.40).

No bench has a `host.rs` or `host_policy.rs`: the shared harness observes the host and decides (`outbound/host`, `domain/host_applicability`). The roles `host-probe` and `host-policy`, the `import-directions` entry on `host_policy.rs`, the scenarios `host-probe-and-policy-have-shared-roles` and `host-policy-cannot-probe-environment`, and the `host-probe`/`host-policy` branches of the `bench-entry` template are removed; the template renders `bench.rs` without them.

The suppression policy every suppressing rule's `codes` argument is rendered from moves from `results/standard-suppression-policy.json` to `profiles/pinned-suppression-policy.json`, role `registry-pinned-suppression-policy-json`, beside the pinned noqa code catalog; the Ruff schema test checks each query against both.

No Python program is placed outside a bench's evidence. The Ruff bench's authoring tools (`authoring/`, `verify_profiles.py`), its generated `RULES.md`, its `results/` captures (but the 25 UP031 process captures its cases declare), the frozen-manifest profiles (`profiles/prepared.json`, `native-standalone-*`, `archived-inputs/`, `oracle.json`, the variant and combined profiles), the upstream copies its cases inline and the evidence no case declares are deleted, with every role only they used: the `results/` roles but `captured-stdout-bin`, `captured-stderr-bin`, `captured-invocation-json` and `captured-stdout-json` (which place the UP031 process captures its cases declare, kept), `authoring-source`, `verify_profiles.py` (and its `file-length` include), `run-certifier`, `run-program-*`, `registry-native-standalone-*`, `acceptance-profiles`, `addressed-*`, `rule-inventory-guide`, `rule-upstream*`, the variant case-file roles and the profile snapshot roles. The per-name evidence program roles collapse into `evidence-program`, `evidence-phase-program`, `rule-evidence-program` and `fragment-evidence-program`: a `.py` a rule's cases declare is data the harness stages and hashes, never runs. Scenarios `named-evidence-is-placed`, `unreviewed-dated-evidence-is-refused`, `acceptance-profile-inputs-are-placed`, `frozen-combined-source-snapshots-are-placed`, `shape-combined-evidence-is-placed`, `unplaced-expanded-report-is-refused`, `unplaced-shape-combined-evidence-is-refused`, `numbered-process-records-have-protocol-roles` and `captured-python-input-is-data` go with their roles; `evidence-cannot-admit-arbitrary-oracle-program` and `exact-protocol-rejects-suffix-escape-0`, `-1` and `-8` go because any program under evidence is now evidence data; `dated-evidence-has-explicit-roles` and `typed-archive-and-authoring-roles` keep only their retained roles; `evidence-program-is-placed` and `a-python-program-outside-evidence-is-refused` are new. The `companions` rule no longer exempts `results/**`.

## 0.2.3

Every linter bench gets the harness's heap and CPU profile run with no line of its own: `bench.rs` is the verbatim `bench-entry` template (its `mod` lines rendered from the bench's optional `frontend_projection.rs`, `host.rs` and `host_policy.rs`), a new verbatim `composition-profile` template places `composition/profile.rs`, which wires the harness's `profile` use case at the bench's edge, and `composition.rs` (template `composition-root`) declares `pub(crate) mod profile;`. The Ruff-only Python profile driver's placements (`profile_ryo.py`, `worker_profile_contract.py`, `test_worker_profile_contract.py`) are removed. Everything else is as in 0.2.2.

## 0.2.2

Places `Cargo.toml`, the linter's own unpublished package: its `bench.rs` and `tests.rs` targets move out of the language provider's manifest, so the provider's published package carries no linter rule set or harness. Nothing else changes.

Edited in place: the module file of a directory is `<dir>.rs` beside it, never `<dir>/mod.rs` (owner, 2026-11-11; `mod_module_files` denied by cargo-workspace 0.13.0): `sharing.rs`, `sharing/fixtures.rs` and `frontend_projection.rs`; the test boundary reads them.

## 0.2.1

Places `sharing/fixtures/{part}.rs`, one file per concept of the inventory proof's fixtures, beside `sharing/fixtures/mod.rs`, which the 400-line limit otherwise forces to hold all of them. A nested directory or a file outside `sharing/fixtures/` is still refused. Nothing else changes.

## 0.2.0

`composition.rs`, `composition/*.rs`, `composition/constants.rs` and `worker_request_input_constants.rs` may import `ryo_benchmark_contracts::*` and the language's own `ryo_python::settings::*`: the contract types moved out of `ryo_benchmark` into `ryo_benchmark_contracts`, which 0.1.0 predates. Shared application policy (`ryo_benchmark::application::*`) stays refused. 0.1.0 stays as published.
