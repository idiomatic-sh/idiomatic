# port

## 0.2.0

Places `adapter` 0.2.0, whose adapter struct is `pub(crate)` (rustc's `unreachable_pub`); nothing else changes. A product moves by placing `nu-hex` 0.2.0.

**Store ports may exchange their flat tables together** (amended in place, owner 2026-10-29: "fix the spec yes"). A store port may load and replace its product's flat tables as one value (`load() -> Tables`, `replace(Tables)`) when every run replaces them all, in one write to a temporary file renamed over the store. A nested whole-state value (an `Image`), or a write that replaces some tables and keeps the others, is still not a store port's shape. The `no-whole-state-ports` rule shows the line with scenarios `flat-tables-replaced-whole` (passes) and `nested-image-replaced` (fails); it reads `Image` names only, so a partial replace is stated here and not found by a rule.

## 0.1.0

The first version, split from the retired `part` spec by kind. It lists its own files with their counts, shares the templates and rules the other kinds use, and is placed by the crate kind that holds it, so a product's tastemaker.toml names only the crate and its crate kind.

**.** The port places its adapters (`adapters/{implementation}`, at least one) and the `factory-name` rule moved here from the retired adapter-crate spec.

`adapters.rs`, `factory.rs` and the `adapter` parts count `min = 0`: a port the SDK exposes as an extension point has none, and the shared `extension-point-ports` rule requires at least one adapter for every other port.

**.** The contract moves from `interface_test.rs` to `fixtures/contract.rs` and the port gains `fixtures.rs` and `fixtures/fake.rs` (the fake of the port, no name rule); `types_test.rs` and `errors_test.rs` are deleted. Fixtures: `fake` and `fixtures` templates (example, extra content, missing, contract-only).

- The spec takes `exposed` from the nu-hex and passes it to the interface and fake templates and to `kind-suffix`: a listed port's trait is its plain noun, every other port's ends in `Port`. The fake declares `<Port>Fake` and its test module `fixtures/fake_test.rs`, which runs the port's contract.

- The `adapter-module` template declares the adapter's `settings`, `types`, `errors` and `schemas` when it holds them.

- **Adapters are leaves:** the `adapters` template declares each implementation as `pub(super) mod <implementation>;` instead of `pub(crate) mod`, so an adapter directory is reachable from its port (the factory) and no wider; the sibling-reach is also found by the adapter spec's `adapter-owns-no-adapter` rule. A private `mod <implementation>;` would hide the module from the port's `factory.rs`, so it is `pub(super)`.

The audit becomes rules of the port:

- `plain-port-types` (error, `types.rs`): every struct and enum derives `Clone`, `Debug` and `PartialEq` (a lifetime-parameterized view is exempt from `Clone`), and none names `File`, a sync type (`Mutex`, `RwLock`, an atomic, `Arc<..Mutex..>`) or `dyn`; an infrastructure crate in a port's types.rs is `client-at-adapter`'s (stated once).
- `no-round-trip` (error, `interface.rs` with `types.rs`): a non-plain type one method returns is not taken by another method of the same port.
- `config-in-factory` (warn, `factory.rs` with `interface.rs`): a type the factory takes is not a parameter of a port method, and no method takes a `PathBuf`, `Directory`, `Settings` or `Budget`.
- `interface-shape` is placed with its amended text: one port trait plus the traits its methods return.
- The `factory`, `interface` and `types` templates scaffold constructor injection (a factory takes its clients and returns the adapter), plain types and handle traits returned by the port; their example fixtures carry the new header text.

**Handle traits are exempt from `no-round-trip`.** A handle trait declared in the same `interface.rs` is no longer non-plain: a consuming method `fn finish(&self, workers: Vec<Box<dyn FactsWorker>>)` that takes back the handle `FactsWorker` another method returns is the approved shape and passes (fixture `handle-taken-back`, replacing the failing `handle-round-trip`). What the rule still forbids is a struct or enum of `types.rs` holding a `File`, a sync type or a `dyn` that one method returns and another takes (fixtures `shard-round-trip`, `dyn-round-trip`, `file-round-trip`).

- **`test-runners-match-rows`** is placed beside `test-shape`: a test file has a `run(CASES, Word::<word>, subject)` runner exactly for each case word its `CASES` table has rows of. The test templates (`unit-test`, `item-test`, `schemas-test`, `composition-test`, `command-test`, `entry-test`, `use-case-test`, `adapter-test`) no longer render all four runners: an empty table has no runner. The fixtures `*-without-rejects-runner` became `*-with-rejects-row-without-runner`.

A port's test support may hold further files: the `port-fixture` template (owned, open) places `fixtures/{name}.rs` for every `.rs` of `fixtures/` except `contract.rs`, `fake.rs` and `fake_test.rs`, which have templates of their own, so a fake that replays material keeps it in `script.rs`, `trees.rs`, `library.rs` and the contract's proof in `contract_test.rs`. The port's `fixtures.rs` declares them (the `fixtures` template is open: it requires `contract` and `fake` and accepts any further `pub mod`). Fixtures: the template's `example`, `extra-content`, `missing` and the spec's `port-with-fixture-files`.

A store port, as the spec's why says: its settings (the directory and the build) go to the factory, its reads are empty when the store is absent, of another build or undecodable, and each `keep_…` writes once on success through a temporary file and a rename. The `factory` template's `store-settings` scenario proves that shape of factory.
