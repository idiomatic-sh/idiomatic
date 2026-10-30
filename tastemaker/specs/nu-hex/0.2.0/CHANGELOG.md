# nu-hex

## 0.2.0

Places `port` 0.2.0, and through it `adapter` 0.2.0, whose adapter struct is `pub(crate)` (rustc's `unreachable_pub`); nothing else changes. The scenarios place `nu-hex` 0.2.0 and their adapters are `pub(crate)`. A product moves by changing the `version` of its `nu-hex` placements to 0.2.0 and writing each adapter `pub(crate) struct`.

Scenarios `io-at-boundary-reaches-a-port-part`, `core-fakes-reaches-a-port-part`, `plugin-schemas-reaches-a-port-part` and `fixture-location-reaches-a-port-part` prove the spec's rules read the files of the port parts it places.

## 0.1.0

The first version. One of the two specs that replace the `hex` spec's
use-case, adapter and plugin conditions. A nu-hex is one crate holding one product core, applied beside its `crate`
entry at the crate's directory, as a command line is `crate` plus `cli`.

**The principle.** One small alphabet of primitives, the same at every level: interface,
use_case, service, item, adapter, schemas, composition, command and entry, factory, settings, fixture,
types, errors, the test beside each logic file, the feature file and its runner, and the module file
beside each directory. A part has the same five at every level: a body, its test, its types, its
errors and an interface where something else implements it. Every template writes exactly one
primitive, and a file that matches no primitive is a finding under the closed placement.

**What it places**, under the crate's `src/`:

| path | template | note |
|---|---|---|
| `inbound.rs`, `inbound/use_cases.rs`, `inbound/use_cases/{use_case}.rs` | `inbound`, `use-cases`, `use-case-module` | one core, several use cases; at least one; the module file declares the use case's types and errors with their tests when it has them |
| `inbound/use_cases/{use_case}/{interface.rs, use_case.rs, use_case_test.rs}` | `use-case-interface`, `use-case`, `use-case-test` | the use case holds only its ports, each as an `Arc<dyn <Name>Interface>` field (`use-case-ports`) |
| `inbound/use_cases/{use_case}/{types.rs, types_test.rs, errors.rs, errors_test.rs}` | `use-case-types`, `use-case-types-test`, `use-case-errors`, `use-case-errors-test` | optional; the use case's request and answers, and the error that wraps the ports' errors it can meet |
| `domain.rs`, `domain/{concept}.rs`, `domain/{concept}/{service.rs, service_test.rs}` | shared `domain-concepts`, `concept-module`, `service`, `service-test` | pure functions over plain types, no port |
| `domain/{concept}/{types.rs, types_test.rs, errors.rs, errors_test.rs}` | shared `concept-types`, `concept-types-test`, `concept-errors`, `concept-errors-test` | optional |
| `outbound.rs`, `outbound/{port}.rs`, `outbound/{port}/{interface.rs, interface_test.rs}` | `outbound`, `port-module`, `interface`, `interface-test` | the core's own ports; the contract exports its rows `CASES` and `check`, `pub` under the `conformance` feature, because the adapters are in other crates |
| `outbound/{port}/{types.rs, types_test.rs, errors.rs, errors_test.rs}` | `port-types`, `port-types-test`, `port-errors`, `port-errors-test` | optional; what a port speaks lives with the port, because its adapter constructs it |
| `outbound/{port}/{schemas.rs, schemas_test.rs}` | shared `schemas`, `schemas-test` | at exactly the plugin ports the entry lists in `plugins` |
| `collections.rs`, `collections/{collection}.rs`, `collections/{collection}/{interface.rs, interface_test.rs, types.rs, types_test.rs, errors.rs, errors_test.rs}`, `collections/{collection}/{item}.rs`, `{item}/{item.rs, item_test.rs}`, and per concept | shared `collections`, `collection`, `collection-interface`, `collection-interface-test`, `collection-types`, `collection-types-test`, `collection-errors`, `collection-errors-test`, `item-module`, `item`, `item-test` | N items of one shape, stated once in the collection's interface, at the core and at one domain concept; the item names `interface`, `types` and `errors` are the collection's |
| `fixtures.rs`, `fixtures/{fixture}.rs`, `fixtures/{fixture}/{fixture.rs, fixture_test.rs}` | shared `fixtures`, `fixture-module`, `fixture`, owned `fixture-test` | the fakes of the core's ports; a fake of an outbound port runs its contract |

There is no crate-level `types.rs`, `types_test.rs`, `errors.rs` or `errors_test.rs`, no
`types/<value>/`, no `steps/`, no `adapters/`, no `factory.rs`, no `dtos.rs` and no
`interface.rs` at the root. Closed placement refuses each as an unplaced file (fixtures
`crate-level-types`, `crate-level-errors`, `value-split`, `steps`, `adapter-in-core`, `factory-in-core`,
`interface-at-root`, `dtos-in-core`). A definition lives with the part whose code constructs it, an error
with the part that produces it, and every other part imports it: the entries' `imports` lists let a use
case import its own and a port's types and errors and a concept's types, a concept a port's types and
another concept's, and an interface only vocabulary.

**The test shape.** Every unit test is a table `CASES` of rows, typed by the three aliases
`Input`, `Value` and `Failure`, and the four runners `succeeds`, `refuses`, `fails` and `rejects`; the
templates pin those names and the alias names, not what the aliases stand for. The contract's `CASES`
is `pub` and its three aliases stay private (a public constant may name a private alias, and `no-reexports` refuses a public one).
`fixture-location` lets the contract file hold the rows and their aliases.

**The three kinds of test.** The unit test beside its primitive (the table above), the contract a port
exports and every adapter runs (`interface_test.rs`, `CASES` and `check`), and the crate's behaviour through its
surface: one `tests/<use_case>.feature` per use case with its steps in `tests/bdd/<use_case>.rs`, run by
`tests/bdd.rs` and faking only the stores' connections. The crate spec places the three files and its `bdd-members`
refuses a feature or steps file that names no use case; a core's use cases are required, so this spec's `hex-members`
says the other way: a use case with no feature or no steps file is a finding at its module file (fixtures
`use-case-without-feature`, `use-case-without-steps`).

**Rules**, placed over `src/` (`hex-members` also over `tests/*.feature` and `tests/bdd/*.rs`): `use-case-methods`, `use-case-ports`, `use-case-modules`, `contract-methods`,
`fake-contract`, `fixture-location`, `fixture-fakes`, `hex-members`, `plugin-schemas` and `core-dependencies`,
owned; `interface-suffix`, `interface-shape`, `hollow-types`, `no-domain-aliases`, `no-inherent-impls`,
`validation-at-edges`, `domain-modules`, `hollow-artifacts` and `no-runtime-fixtures`, shared by both specs; and
`test-shape` and `case-expectation` over `src/**/*_test.rs`, `unit-cases` over those and `src/**/errors.rs`, and
`item-contract` over `src/**/*.rs`, the shared rules of the one test shape, which this spec places over its own tests as
the cli and sdk specs place them over theirs (the crate spec places none of them). `case-vocabulary` is no longer placed: the
four runners are named by the case words, which `test-shape` holds.

| rule | what changed from the `hex` spec's |
|---|---|
| `use-case-methods`, `use-case-modules`, `contract-methods`, `fake-contract`, `fixture-fakes` | the same queries (they read declarations and the call of `check`, not the layout); the fixtures of `fake-contract` and `contract-methods` show the tests as the templates write them: a fake's test is the `CASES` table and the four runners with the contract's `check` in the success runner, and a contract exports `CASES` and `check` |
| `fixture-location` | the contract file may also hold `CASES` and the aliases `Input`, `Value` and `Failure` that type it |
| `use-case-ports` | new: a use-case struct holds each port as an `Arc<dyn <Name>Interface>`; a field whose type mentions one of the struct's own generic parameters is a generic port, and a finding. Ports are dynamic, data is static |
| `hex-members` | pairs the core's ports (module file, port, contract, line in `outbound.rs`), domain concepts (module file, service, test, line in `domain.rs`), plugin schemas with their test, each collection (interface, module file, items), each use case's feature and steps file, and, for every kind of part, a types.rs with its test exactly when it holds a hand-written trait impl and an errors.rs exactly when it has a function body; the use cases' own files are paired by the spec's own counts (`min = 1`) |
| `plugin-schemas` | new: `schemas.rs` and its test stand at exactly the ports the entry lists in `plugins` |
| `fake-contract` | a fake of an outbound port runs that port's own contract from its `fixture_test.rs`: the test calls `interface_test::check` and depends, as the provider resolves it, on `outbound/<port>/interface_test.rs` of the same port; a fake of a use case's port needs none |
| `fixture-fakes` | the fixtures fake exactly the core's own ports, every outbound port and every use case's port, and no other crate's |
| `core-dependencies` | new: the manifest's `[dependencies]` and `[build-dependencies]` name only the entry's `vocabulary` and the std-level crates serde, thiserror and tracing; a renamed dependency is read by its `package`, an inherited one by its key, which is its package while the `no-registry-renames` rule holds for the workspace table |
| shared `no-inherent-impls`, `validation-at-edges`, `hollow-types` | and a types.rs holds no `impl Type { }`, a `TryFrom`, `FromStr` or fallible constructor stands only in a schemas.rs, and a types.rs declares a type |
| shared `test-shape`, `case-expectation`, `unit-cases`, `item-contract` | new here: every test is a `CASES` table and the four runners, a row expects a value or an error variant, a part's test has a `succeeds` row and, where the errors.rs beside it has variants, a `refuses` row, and each item of a collection runs the collection's shared cases |
| dropped | `behaviour` and the `errors-behaviour` condition (no crate-level errors.rs), `value-placement` and `private-checked-fields` (no value split, no validated value), `case-vocabulary` (the runners are the case words), `hex-core`, `contract-required`, `no-orchestration`, `adapter-*`, and the owned `interface-suffix` and `interface-shape`, which are shared now |

**Entry params.** `vocabulary` lists the product's vocabulary crates the manifest may name beside the
std-level ones; `plugins` lists the outbound ports whose data a plugin emits, each with a `schemas.rs`.

**The shared assets.** The templates and rules both new specs use are shared (`tastemaker/templates/`,
`tastemaker/rules/`, 0.1.0). Findings of a shared asset carry its own name
(`idiomatic.arch/no-inherent-impls`), those of an owned one the spec's (`idiomatic.arch/nu-hex/hex-members`).

Fixtures: `complete` (one use case over one port, with the port's fake and contract) and `missing` (a use
case without its test). Failing, each one violation:

`missing-inbound`, `use-case-without-feature`, `use-case-without-steps`, `stray-file`, `steps`,
`adapter-in-core`, `factory-in-core`, `interface-at-root`, `crate-level-types`,
`crate-level-errors`, `value-split`, `dtos-in-core`, `lib-undeclared`, `manifest-without-thiserror`,
`manifest-without-conformance`, `client-dependency`, `domain-calls-a-port`,
`use-case-names-a-library`, `port-names-a-library`, `types-name-a-library`, `generic-use-case-port`,
`validation-outside-schemas`, `inherent-impl-in-types`, `port-not-named-interface`,
`port-with-a-free-function`, `use-case-with-another-method`, `use-cases-list-holds-code`,
`types-without-a-type`, `types-with-an-alias`, `domain-list-holds-a-rule`,
`contract-skips-a-method`, `contract-holds-a-struct`, `fake-without-contract-run`,
`use-case-port-without-a-fake`, `production-reads-fixtures`, `placeholder-type`,
`test-beside-the-runners`, `is-ok-expectation`, `use-case-without-succeeds`,
`use-case-without-refuses`, `item-without-contract`, `plugin-without-schemas`,
`schemas-at-a-plain-port`, `concept-collection-without-interface`, `port-errors-without-test`.

Passing, each a shape a core may hold:

`two-use-cases`, `several-ports`, `concept-complete`, `collection-complete`,
`concept-collection-complete`, `concept-collection-with-tests`, `collection-with-tests`,
`use-case-with-error-behaviour`, `use-case-with-types`, `plugin-with-schemas`,
`port-errors-with-behaviour`.

Every owned rule and template has fixtures of its own.

Edited in place. The spec places only the core's namespaces (the inbound, domain and outbound lists with each part's module file, the collections, the fixtures and their fakes); every directory that holds code is a part of the `part` spec, placed by the product's entries. What moved to the part spec: the use case, port, concept and item templates and their rules. `fake-contract`, `plugin-schemas` and `fixture-location` are shared rules the starter places, and `module-declares`, `collection-members` and `item-contract` replace `use-case-modules` and `hex-members`. `fake-name` names a fake `<Port>Fake`. The `plugins` parameter is the `plugin-schemas` entry's.

the `part` spec is retired for use-case, concept, port, item, adapter, command, entry and collection, and this version places the specs of its own parts, so a product's tastemaker.toml names only the crate and its crate kind.

**and.** The crate kind is `nu-hex` (renamed from `core-crate`, to be renamed `hex` when the old `hex` spec retires) and it is the one crate kind: the adapter-crate spec is retired. A port holds its adapters: `src/outbound/<port>/{adapters.rs, factory.rs}` beside its interface, and `src/outbound/<port>/adapters/<implementation>/{adapter.rs, adapter_test.rs}` with an optional `settings.rs` (the `adapter` part; the `adapters`, `factory` and `adapter-module` templates moved here). `core-dependencies` lets a client or store library in `[dependencies]` only as `optional = true`, behind an adapter's feature.

There is no cargo feature per adapter. An adapter's client library is a normal dependency of the core; the `core-dependencies` rule and the `vocabulary` param are removed, and the shared `client-at-adapter` rule (placed in the product's shared block) confines the library to an adapter, its port's `factory.rs` and an edge's `composition.rs`.

**.** A port's module file declares `adapters` and `factory` only when the port has them (`port-without-adapters`).

The crate-level `src/fixtures.rs` and `src/fixtures/`, the `fixtures`, `fixture-module`, `fake`, `fixture` and `fixture-test` templates, the `fake-name` rule and the `<Port>Fake` name rule are gone: a port keeps its contract and its fake in its own `fixtures/` (the `port` spec), gated by `#[cfg(any(test, feature = "fixtures"))]` in the port's module file. `types_test.rs`, `errors_test.rs` and `fixture_test.rs` are deleted from every spec. A collection lives only in `domain/<concept>/collections/`: the crate-level `src/collections/` is removed. Fixtures: `collection-at-crate-root` and `crate-level-fixtures` refuse the two retired places; `fixtures-not-gated` refuses a port's `fixtures` module without the gate.

- The spec takes `exposed = list<string>`, the ports the product's SDK exposes, and passes it to the port spec it places; the `conformance` feature is gone: the port's contract and fake are compiled under `fixtures`.

- The `implements` param (required, `[]` for a normal core) is passed to the use-case spec. The module templates declare the optional members the part holds (`types`, `errors`, `collections`, `schemas`, `adapters`, `factory`, `settings`, the interface unless implemented), read through `present`, which now reaches the nested part's entries.

- **`item-trait`**: a required param passed to every concept (and so to every collection) of the crate; `absent` when each collection declares its own `<Collection>Item`. Open: one value serves every collection of a crate, so a crate whose collections implement different traits (a language: `ReadItem` for relations, `ManifestItem` for manifests, its own trait for refinements) states `absent` until the param can name a trait for each collection.

- **No use case required**: a language's `parse` and `project` are contracts of the `contract` spec (idiomatic.ryo), not use cases, so the counts of `use-case`, `use-cases` and `use-case-module` are `min = 0` and the `inbound` template declares `mod use_cases;` only when the list is present. The pairing the module file's count held (a use case directory has its module file) is the shared rule `use-case-module-beside`, which a product places over `src/inbound/use_cases/**`, because a spec's rule does not read the directories of the use cases nested in it and a count of `min = 1` cannot be satisfied by a core with no use case; the fixture `missing-use-case-module` moved to that rule. Open: a core other than a language that has no use case is no longer a finding of this spec. Fixture: `core-without-use-cases`.

**The `kernel` param**. A kernel crate (the values crate and the language crate) names its concept modules in `src/domain.rs` as `pub mod <concept>;`, every other core as `pub(crate) mod <concept>;`; the `crate` spec's `kernel` param of the same crate renders the matching `pub mod domain;` in `src/lib.rs`, so a product states `kernel = "kernel"` in the crate and the nu-hex placements of its kernel crates. Required (`kernel` or `absent`). Fixtures `kernel-complete`, `kernel-private-domain` (lib.rs private) and `kernel-private-concepts` (concept modules `pub(crate)`).

**A kernel crate needs no use-case surface**. The `inbound` template's count is 0 when the crate's library root declares its domain public and no inbound module (the owned rule `domain-only`, the spec's `domain-only` condition), and still exactly 1 for every other core, the language crate (a kernel crate with use cases) included: the values crate has a domain and no use case. The `concept-module` template takes the `kernel` param (`pub mod service;` and the optional members public in a kernel crate, `pub(crate)` otherwise), passed from the spec's `concept-module` entry. Fixtures `kernel-without-inbound` (passes), `missing-inbound` (an ordinary core still fails), the template fixtures `kernel-public`, `kernel-crate-private`, `ordinary-public`, and the rule's `public`, `private`, `restricted`, `with-inbound`; `kernel-complete` declares its concept's members public.

**A kernel crate without an inbound module still declares `pub mod domain;`**. The `crate-root` template of the crate spec 0.12.0 now renders `pub mod domain;` for a kernel crate whenever its domain list exists, not only beside `pub mod inbound;`; fixtures `kernel-without-inbound` (passes with it) and `kernel-domain-undeclared` (omits it, fails).

Provider role: the `kernel` param takes a third value, `provider`, for a language provider crate (`nu-hex-languages`). Its lib renders `pub(crate) mod inbound;` and `pub(crate) mod outbound;` (private `mod domain;`) and its `src/outbound.rs` renders `pub(crate) mod <port>;`, since only the contracts it links reach the SDK; concept members stay `pub(crate)`, and the crate keeps its bench `cpu-time`. Fixtures `provider-complete` and `provider-public-modules`.

- **`item-trait` is a list of `<collection>=<trait path>` pairs**: it closes the open point of the entry above. `[]` when every collection declares its own `<Collection>Item`; otherwise a pair for each collection whose items implement a trait of another crate, so a crate whose collections implement different traits is expressible (the language provider: `relations=ryo::languages::ReadItem`, `links=ryo::languages::ReadItem`, `manifests=ryo::languages::ManifestItem`). Passed to every concept with the crate's `kernel`, and from there to the collections and the items, which read their own collection's pair. The kernel and provider roles now also decide the visibility of the collection and item modules and the name of the item struct (item, collection and concept specs). The `complete`, `kernel-*` and `provider-*` fixtures pass `item-trait = []`; `crates/ryo/tastemaker.toml` states the pairs for `nu-hex-languages` and `[]` for every other entry.

- **The `kernel` param is gone, and nothing replaces it** `nu-hex` places the same entries for every crate. The `domain-only` condition (the library root declares `pub mod domain;` and no inbound module) keeps the values crate legal without `src/inbound.rs`; the language crate, with the same declaration and an inbound, has one. `inbound` also declares `pub(crate) mod contracts;` when the crate holds contracts, the `contract` part is nested at `src/inbound/contracts/{contract}` with its `contracts` and `contract-module` entries (a language provider's inbound surface), and the shared `allocation-test` is placed at `tests/allocation.rs` (`count = { min = 0 }`). `domain-concepts`, `concept-module` and `outbound` declare their modules without a visibility. Fixtures `kernel-complete`, `kernel-without-inbound`, `kernel-with-allocation-test`, `kernel-allocation-test-without-allocator`, `provider-complete` and `provider-feature-names-no-contract` hold the kernel and provider shapes; `concept-public` is gone.

`no-runtime-fixtures` over the namespace files leaves out `src/fixtures.rs`, the fixtures module that a spec beside this one places (the `language` spec places it for a language); fixture `language-fixtures-module`.

**A language's modules are crate-private, and the scaffold says so.** The `domain-concepts`, `outbound` and `concept-module` templates declare their members `pub(crate) mod` when the `language` spec is placed beside the core (`present["language-conformance-test"]`), because the contracts, the library root and the items of a language reach each other by those paths; a module declared without a visibility is private to its parent, and a scaffold that stops compiling at its first cross-module `use` is not a scaffold. Other crates declare their modules as before. Fixture `concept-modules-private` (in the `language` spec).

The fixtures' bench files no longer carry divan or a budget helper (`fn main` only), as the `crate` spec asks.
