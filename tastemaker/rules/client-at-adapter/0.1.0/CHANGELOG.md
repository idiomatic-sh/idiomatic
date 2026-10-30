# client-at-adapter 0.1.0

A client or store library is named only in an adapter (`src/outbound/<port>/adapters/**`), its port's `factory.rs` and an edge's `composition.rs`. It replaces the `nu-hex` spec's `core-dependencies` rule, which let a client library into a manifest only as an optional dependency behind a cargo feature per adapter. The `libraries` param lists the libraries as path globs (`rusqlite::*`).

A port's `types.rs` and `errors.rs` are inside the rule's reach on purpose: the placement excludes only adapters, factories and compositions, never a port's own types or errors, so an infrastructure crate in a port's plain types is a finding here and `plain-port-types` does not state it a second time. Fixtures `port-types`, `port-errors` and `port-types-plain`; the rule's text names the files.

A domain file's `*_test.rs` is inside the rule's reach: the product placement excludes adapters, factories, compositions and the generated runners, never a test beside the domain, so a test of the domain cannot name the infrastructure library. Fixtures `domain-test` (violation) and `adapter-test` (pass, the adapter's test is under `adapters/`).
