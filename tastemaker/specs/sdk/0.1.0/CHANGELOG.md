# sdk

## 0.1.0

The SDK's own domain, edited in place: a value more than one function shares (a relation's name, a file nothing read, the threads a run reads on, a typed cell) is declared once, below the functions, as the SDK's one edge copy of its concept. Four templates place it, each `count = { min = 0 }`: `domain` (`src/domain.rs`, one `pub(crate) mod` per concept), `domain-concept` (`src/domain/{concept}.rs`, declaring its `types` and its `errors`), `domain-types` (`src/domain/{concept}/types.rs`) and `domain-errors` (`src/domain/{concept}/errors.rs`). A function imports such a value from `crate::domain::<concept>::types`, never from a sibling function or its resource (`crate::resources`, `super::super`, which `import-directions` refuses), and its resource module exposes it under its one name, with no `as`. Fixtures: passing `function-imports-a-domain-value`; failing `domain-file-outside-the-placed-set` (`sdk#closed`: a `service.rs` in the domain, which no template places).

The SDK's lock: connections, schemas and one test shape, edited in place, as `cli` 0.10.0 has it.

- **`connections.rs` beside `composition.rs`** in each function (`entry-connections`): it opens the stores and returns their clients, the only I/O of the edge; `composition.rs` takes the clients and the parts and only constructs (`compose(connections, parts)`; `build()` calls `connections::open()`), and `composition_test.rs` runs the real wiring over fake connections. `entry-composition`, `entry-module`, the shared `composition-test` and `composition-parts` follow.
- **`schemas.rs` and `schemas_test.rs` at each function** (the shared `schemas` and `schemas-test`): the shape of the request a user gives and its checked conversion into the product's types. `validation-at-edges` is placed over `src/**` but the schemas, the tests and the extension points (a `hex` entry of the product's, which keeps the `hex` spec's values).
- **`entry-test` is the one test shape**: a `CASES` table and the four template-owned runners `succeeds`, `refuses`, `fails` and `rejects` calling `crate::cases::run`, the aliases `Input`, `Value` and `Failure` standing for the table's types. It no longer writes `<verb>_succeeds`; the subject is `todo!` until a row states the call. `test-shape`, `case-expectation` and `unit-cases` are placed over `src/entries/**/*_test.rs`; `case-vocabulary` is not, because it reports the bare `fails` of the runner.
- The imports allow-list drops `*::value` (there is no value split) and the `entry` entry may import its `schemas`. The description states the principle of one alphabet of primitives, each file one of them. The extension points keep their `hex` entries.

Moving up from the previous edit: add `connections.rs` and `schemas.rs` with its `schemas_test.rs` to each function and declare them in its module file; give `composition.rs` the `compose(connections, parts)` shape; rewrite each `entry_test.rs` and `composition_test.rs` as a table and the four runners. Fixtures: `complete` and the rest follow; failing `missing-connections`, `missing-schemas`, `validation-outside-schemas`, `schemas-test-without-a-success`, `missing-runner` and `extra-entry-test` (it replaces `misnamed-case`).

The imports allow-list names `*::value` beside `*::types`: a value with behaviour is
`types/<value>/value.rs` of its hex.

Entry compositions now expose crate-private typed `Parts` and `compose`, with
production choices in `build`. The active composition rule and required native gate
enforce the construction boundary. Optional `types.rs` and `composition_test.rs`
owners carry dependency declarations and same-wiring substitution proofs; SDK internals
remain private. All interface-bearing intermediate and returned services are supplied
as ports or constructor callbacks, with typed settings forwarded by the owning root.

Every entry and rule carries a `description`, and the fixtures whose root holds a Cargo workspace manifest no entry places say `closed = false`.

An SDK is an edge, like a command line: a user sees only what the user is aware of, and the edge the SDK owns composes the internals, as a command line's user never composes a command. The SDK crate does what the command line crate does.

An entry's composition declares `fn build()` without a visibility: the entry calls it, and
the entries' compositions are the crate's, so `pub(crate)` builds for the entry alone
where `pub` in a crate-private module drew an `unreachable_pub` expectation. Fixture:
passing `crate-build`.

The spec is `cli` 0.10.0's shape, applied beside the crate's `crate` entry, with the same
rules placed the same way:

| cli 0.10.0 | sdk 0.1.0 |
|---|---|
| `src/commands.rs` (`commands`) | `src/entries.rs` (`entries`) |
| `src/commands/{resource}.rs` (`resource`) | `src/entries/{resource}.rs` (`resource`) |
| `src/commands/{resource}/{command}.rs` (`command-module`) | `src/entries/{resource}/{verb}.rs` (`entry-module`) |
| `.../command.rs` (`command`), `fn run` | `.../entry.rs` (`entry`), a public function named for its verb |
| `.../composition.rs` (`command-composition`), `pub fn build` | `.../composition.rs` (`entry-composition`), `pub fn build` |
| `.../command_test.rs` (`command-test`) | `.../entry_test.rs` (`entry-test`), `<verb>_succeeds` |
| `case-vocabulary` over `src/commands/**/*_test.rs` | the same over `src/entries/**/*_test.rs` |
| `no-root-composition` on `src/composition.rs` | the same (its own copy: an owned rule is private to its spec) |
| `factory-calls` over `src/**` but each command's `composition.rs` and the tests | the same, but each function's `composition.rs` |
| `no-runtime-fixtures` over `src/**` but the tests | the same |
| `command`'s `imports`: deny by default; clap, std, its composition, `crate::outcome`, `crate::output`, `*::interface`, `*::types`, `*::errors` | `entry`'s `imports`: deny by default; std, core, alloc, its composition, the SDK's vocabulary, `*::interface`, `*::types`, `*::errors` |

What differs from `cli`, each because an SDK is a library and not a process:

- No `src/main.rs`, `src/outcome.rs` or `src/output.rs`: a function returns the SDK's own
 types and prints nothing. One left in an SDK crate is a closed finding.
- No `benches/performance/{resource}/{command}/bench.rs` per function: the SDK keeps the
 `crate` spec's `benches/performance.rs` and its workloads.
- `command-refuses` is not placed: what a function refuses is its error type, proved in
 `entry_test.rs` by a test named with the case vocabulary.
- `thin-command` is unchanged and placed by the product's shared rule block, which adds
 `**/src/entries/**/entry.rs` beside the commands: no loop, no branch, at most one service
 call. Its `run` returns `Outcome` check reads only a function named `run`, and a function
 here is named for its verb.
- `hollow-services` 0.1.0 over each function's `composition.rs`, as
 `cli` 0.10.0 now places it over each command's.

What is special to an SDK:

- **Extension points**, which a user implements and the product calls. Each is a `hex` the
 product's `tastemaker.toml` places at `src/extension_points/{point}` (a spec holds no
 other spec), with its `domain-module` at
 `src/extension_points/{point}.rs`; this spec places `src/extension_points.rs`, which
 declares each point (`extension-points`, optional: an SDK without a point has no such
 module). That a point's code names only the SDK's own modules is the `imports`
 allow-list on that `hex` entry (`crate::*`, `self::*`, `super::*`, `std::*`, `core::*`,
 `alloc::*`): a `use` of an internal crate in a point's `types.rs` is a
 `hex(<entry>)#imports` finding, and its `interface.rs` is already held to its own
 `types` and `errors` by the hex spec.
- **The vocabulary**, `src/vocabulary.rs` (`vocabulary`, optional): the facade through which
 the SDK re-exports the types of the product's other crates that its surface names, so a
 user depends on the SDK alone. The re-export allowance is the SDK crate's and nothing else's: this spec places `no-reexports` 0.3.0 over `src/**`
 but `src/vocabulary.rs` and `src/lib.rs`, so a re-export anywhere else in the SDK is a
 finding; the product's own `no-reexports` entry leaves the crate's `src/**` to it, and
 the root is the SDK's surface. Which crates' types may be
 re-exported is the product's API check, not this spec.
- `crate` 0.12.0's `crate-root` declares `pub mod entries;` and `pub mod vocabulary;` for
 an SDK crate, and each extension point's `pub mod extension_points;` as it declares any
 hex's first directory.

Fixtures: passing `complete` and `extension-point`; failing `missing-entry-test`,
`missing-composition`, `missing-resource-module`, `entry-calls-factory`,
`entry-names-an-adapter`, `root-composition`, `stray-file`, `main-left`, `misnamed-case`,
`hollow-services`, `extension-point-undeclared`, `extension-point-names-internal` and
`reexport-outside-vocabulary`.

Edited in place. An entry is a part of kind `entry`, placed by the product's entries; the spec places the `entries` list and each entry's module file.

the `part` spec is retired for use-case, concept, port, item, adapter, command, entry and collection, and this version places the specs of its own parts, so a product's tastemaker.toml names only the crate and its crate kind.

**.** The `extension-points` template re-exports the core ports the SDK exposes: the sdk spec takes `extension_points`, the list of `pub use` paths (a port's trait and its types), which the template writes into `extension_points.rs`.

**and.** There is no extension_points.rs and no vocabulary.rs, no `extension_points` param and no extension-point hexes: `src/resources.rs` lists the resources, `src/resources/<resource>.rs` declares the verbs and exposes the types and traits they use, and the entries sit at `src/resources/<resource>/<verb>/` (the `entries/` level is gone). The crate spec detects an SDK by `src/resources.rs`.

- The `entry-module` template declares `connections`, `composition_test` and `fixtures` when the verb holds them.

- The `entry` template calls `compose` with the connections only when the verb has a `connections.rs`; the `entries` list declares every resource file the product places, a resource with no entry included (`languages.rs`).

**.** The spec takes a required `reexports` param, a list of `{ resource, path, as }` (`[]` when none): each resource module writes `pub use <path> [as <as>];` for the items listed for it. A new `contract` template places `src/resources/<resource>/contract.rs` (optional), which the resource module declares as `#[cfg(feature = "fixtures")] pub mod contract;` when the file is there. The `entry-module` template leaves `contract.rs` out.

**.** A resource's `contract.rs` is compiled only under the `fixtures` feature and exists to call the contract another crate keeps in its `fixtures`, so the `no-runtime-fixtures` rule leaves `src/resources/*/contract.rs` out.

A resource module holds exactly what its template renders: the `resource` template is `closed`. A `pub` item the product's `reexports` does not list, a stray module, or any other declaration is a finding, so the surface a user can call is the list and nothing else. The module files carry no `#[expect]` attribute on a re-export; the fixtures follow.

**.** The rule `verb-resolves`: a verb is a callable function at `<resource>::<verb>`, so the resource module binds a name for every verb whose `entry.rs` defines the function. The `entry-module` template declares `errors` and `types` when the entry holds them (the entry spec places them, for the types the SDK owns). `list-matches-source` (the `reexports` list equals the `pub use` items of the resource modules) is not written: a rule's params cannot be a table list, so no query can read the list (the engine feature it needs is in the batch report).

**.** A resource may keep a type of its own and macros for the programs that use it: the open templates `resource-types` (`src/resources/<resource>/types.rs`), `resource-macros` (`macros.rs`) and `resource-macro` (`macros/<macro>.rs`), each `count = { min = 0 }`; the `resource` template declares `mod types;` and `mod macros;` when the files are present (`present["resource-types"]`, `present["resource-macros"]`); the `entry` placement excludes `src/resources/*/macros` and the `entry-module` placement leaves out `types.rs` and `macros.rs`, so none of them is mistaken for a function. This clears the languages resource's `types.rs`, `macros.rs` and `macros/*.rs`. Fixtures: `resource-with-types-and-macros`, `resource-types-not-declared` and the example, extra-content and missing cases of each template.

`no-reexports` leaves out `src/lib.rs`, `src/resources.rs`, `src/resources/*.rs` and `src/resources/*/contract.rs` (the SDK root, its resource modules and their contracts), the same in the sdk spec and, prefixed `**/`, in idiomatic-rust.

**A resource's macros may write re-exports.** `no-reexports` leaves out `src/resources/*/macros/*.rs`: a macro there expands in the resource module that calls it (the `languages` resource's `catalog_names`, called over the catalog's one list), and names itself to that module with a `use`.

The fixtures' bench files no longer carry divan or a budget helper (`fn main` only), as the `crate` spec asks.

Scenarios `verb-resolves-reaches-an-entry-part` and `validation-at-edges-skips-an-entry-part` prove `verb-resolves` reads the entry parts and the own-file `validation-at-edges` rule leaves them to the `entry` spec.
