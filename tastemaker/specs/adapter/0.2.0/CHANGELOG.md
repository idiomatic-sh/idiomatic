# adapter

## 0.2.0

The `adapter` template declares the adapter `pub(crate) struct <Port><Part>Adapter`, and its settings constructor `pub(crate) fn new`, where they were `pub`: the adapter lives in a private `adapters` module and only its port's factory, in the same crate, names it, so `pub` was unreachable and rustc's `unreachable_pub` (on in `cargo-workspace` 0.11.0) reported it. The template names the visibility, so a placed adapter is held to `pub(crate)` exactly. The scenarios' adapters follow. A product moves by placing `port` 0.2.0 (through `nu-hex` 0.2.0) and writing its adapters `pub(crate)`.

## 0.1.0

The first version, split from the retired `part` spec by kind. It lists its own files with their counts, shares the templates and rules the other kinds use, and is placed by the crate kind that holds it, so a product's tastemaker.toml names only the crate and its crate kind.

**.** The adapter lives in the same crate as its port, so the `core` param is gone: the templates import `crate::outbound::<port>::...` and run the contract rows with the crate's own `run`.

`types_test.rs` and `errors_test.rs` are deleted; the adapter's collections are gone and `adapter-scope` is this spec's own rule; `adapter_test.rs` runs `crate::outbound::<port>::fixtures::contract::CASES`; a `schemas.rs` may hold the decode function (`schemas-with-decode-function`).

**Adapters are leaves.** New rule `adapter-owns-no-adapter`, so that no adapter injects another: an adapter reaches no other adapter directory (a), names no other `*Adapter` type (b), names no `*Port` trait but the one it implements (c) and names no `factory` (d). `stateless-adapters` v3 bans only process-global state (`static` items, `thread_local!`, `OnceLock`, `LazyLock`, `lazy_static`, `once_cell`); a `Mutex` or atomic field is the state of one instance and passes, and the five in-source ignores of the old fields-based check are removed.

New rule `no-wildcard-into-domain-enum` (adapter.rs, schemas.rs): a `match` has no `_ =>` arm, so a variant added to a domain enum is a compile error in every adapter that maps it. Fixtures: `every-variant`, `wildcard-outside-match` (pass), `adapter-wildcard`, `schemas-wildcard` (violation).

- **`test-runners-match-rows`** is placed beside `test-shape`: a test file has a `run(CASES, Word::<word>, subject)` runner exactly for each case word its `CASES` table has rows of. The test templates (`unit-test`, `item-test`, `schemas-test`, `composition-test`, `command-test`, `entry-test`, `use-case-test`, `adapter-test`) no longer render all four runners: an empty table has no runner. The fixtures `*-without-rejects-runner` became `*-with-rejects-row-without-runner`.

### Test support of an adapter

An adapter's `fixtures/` holds its test-only files (samples, oracles, expectations) at any depth: the owned open `adapter-fixture` template places `fixtures/{*fixture}` (count min 0, its `fixture` param the path beneath `fixtures/`), so `fixtures/walk.rs` is placed like `fixtures.rs` is elsewhere and no longer needs its own `adapter-file` placement. `adapter-owns-no-adapter`, `no-runtime-fixtures`, `factory-calls` and `validation-at-edges` exclude `fixtures/**` where they already excluded `fixtures.rs` or `*_test.rs`. Fixtures: `adapter-with-fixtures-files` (spec), `example`, `missing`, `nested-file` (template).

The `adapter-test` template has no `todo!`: its subject answers a failure until the author states it, and its runner runs the contract's rows, so a scaffolded adapter compiles and its test fails by name until rows exist.


No hollow artifacts: `schemas-members` owes a `schemas.rs` only where the adapter parses or writes an external format, read mechanically: a `.rs` file directly in the adapter directory names `Serialize`, `Deserialize`, `serde`, `serde_json`, `postcard`, `toml`, `winnow`, `ra_ap_syntax` or `ra_ap_parser`, and the port carries data. An in-memory adapter owes none. The rule reads every `.rs` file of the adapter (`include = ["*.rs"]`), a `schemas.rs` beside a port that carries no data stays a finding, and `schema-name` accepts `impl From<<Name>Schema>` beside `impl TryFrom<<Name>Schema>`. Edited in place: the package is frozen.

A store adapter, as the `adapter` and `settings` templates' why says: built from its settings (the directory and the build), empty when its file is absent, of another build or undecodable, and kept once on success through a temporary file and a rename. The `adapter` template's `store-adapter` scenario proves that shape.
