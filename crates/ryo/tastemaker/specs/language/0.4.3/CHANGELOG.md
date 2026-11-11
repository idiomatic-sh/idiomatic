# language 0.4.3

Places `benchmark` 0.2.2 (and through it `benchmark-linter` 0.2.2) and holds `benchmark-targets` 0.2.0: a linter's bench and test targets are declared by its own package under `benchmarks/linters/<linter>/`, and refused in the language's `Cargo.toml`.

Edited in place: places `no-runtime-fixtures@0.2.0`, `public-fields@0.3.0`, `io-at-boundary@0.2.0`, where it named their earlier versions, so a product that places it locks one version of each rule: two versions of one rule share its id, and the earlier entries ran the later query.

Edited in place: the `language-tastemaker` template writes `client-at-adapter` as `include = ["**"]` with the test-role excludes `idiomatic-rust` names for the same rule, `**/*_test.rs`, `**/test.rs`, `**/*_test/**`, `**/fixtures/**`, `**/fixtures.rs`, `**/tests/**` (tests set a scene up with any library), where it excluded `**/adapters/**`, `**/factory.rs`, `**/composition.rs`, `**/*_test.rs`, `**/tests/fixtures/**`, `**/tests/allocation.rs` and `**/tests/bdd.rs` (python also `src/fixtures/**`). `client-at-adapter@0.2.0` itself admits the adapters, the port's `factory.rs` and an edge's `composition.rs`; every exclude left is a test role `manifest-excludes` checks.

Edited in place: places `unit-cases@0.2.0`, which reads a namespace `*_test.rs` with the tables of its groups.

Edited in place: the `unit-cases` entry also reads the group tests (`*_test/**/test.rs`), so a namespace `service_test.rs` is judged by its groups' rows (scenario `namespace-service-test-rows-in-its-groups` in concept).

Edited in place: the `unit-cases` entry reads `*_test/**/*.rs` beside its own files, the flat groups and the group directories of a namespace test, which the rule reads only when its entry includes them.

Edited in place: the `language-tastemaker` template is exact (`match = "declarations"`): an entry that adds an exclude, an include or a param the template does not write is reported (scenarios `client-entry-with-an-extra-exclude`, `client-entry-with-an-extra-include`, `client-entry-with-an-extra-param`); a manifest's variables may hold values beyond it.

Edited in place: the `test-shape`, `test-runners-match-rows` and `case-expectation` entries of every part (concept, port, adapter, collection, item) also read its flat groups, `<part>/*_test/**/*_test.rs`, as concept's `flat-group-test` placement does, so a plain `#[test]` in a group is reported (scenario `plain-test-in-a-flat-group`).

# language 0.4.2

Places `benchmark` 0.2.1 (and through it `benchmark-linter` 0.2.1).

Edited in place: the `language-cases` template's `run` runs every row of a word (a red or panicking row is collected with its name; one panic after the loop names every red row), as the crates' runners do; the `runner-compares-discriminants` scenario's two expected lines move from 80 to 93 with the longer body.

# language 0.4.1

Places `constants-once@0.2.0` (sibling adapters of one port each keep their own markup constants) and `public-fields@0.2.0` (a struct whose doc proves its privacy with a `compile_fail` example) where 0.4.0 placed the earlier versions. Nothing else changes.

# language

## 0.4.0

Places `contract` 0.3.0, whose scaffolded `parse` contract renders `begin_semantics`, `completion_requirement`, `extend_file_model` and `recognizes_explicit` beside the readers, and whose scaffolded `project` contract renders `project_entries` and `observed_entries`. The embedded renderings of the contracts follow, and the three `*-reaches-a-contract-part` scenarios move to the lines the longer contract gives.

## 0.3.0

Places `contract` 0.2.0, whose parse part may hold readers, and `readers-in-items` 0.2.0, which takes the homes of a reader from the `homes` param: the item folders of the collections and `inbound/contracts/parse/readers/<concept>/composition.rs`.

## 0.2.0

Places `benchmark` 0.2.0 (and through it `benchmark-linter` 0.2.0). Nothing else changes; 0.1.0 stays as published.

## 0.1.0

The first version. A language is placed beside its `crate` and `nu-hex` entries by one entry over `languages/{lang}`.

**The concepts, and what holds each** (the compiler first, then the conformance, then Tastemaker):

| concept | compiler | conformance | Tastemaker |
|---|---|---|---|
| the language answers every relation of the catalog | `Parse` and `Project` have one required method for each; a scaffold leaves them out, so the compiler lists them | `Reads` has a fixture with its full rows, `NotInThisLanguage` has no fixture with the construct and a required `Reason` | `language-parts` (the two use cases, the project adapter, the fixtures) |
| its kinds are one vocabulary | the adapter maps every library name onto a kind | `kind_count` fits a walk | `language-parts` (`src/domain/syntax/types.rs`, no directory for each kind) |
| it reads with a parser library | | | `language-parts` (an adapter for each library), `client-at-adapter` (the library is named only in its adapter) and `imports-allow` of its `tastemaker.toml` |
| a row is numbers | | the allocation test | `no-string-fields` |
| it touches no file, process or network but in its project adapter | | | `io-at-boundary` |
| it is linked | | | `language-parts` (`linked` in `src/lib.rs`) |
| it proves itself | | `tests/conformance.rs`, `tests/allocation.rs` | the templates `language-conformance-test`, `language-allocation-test` |

No part is optional. Not here: the engine is held to `no-language-names` by the product's own entry, because the rule reads only what is outside `languages/`.

**Owned templates.** `language-allocation-test` and `language-conformance-test` are the spec's own, because only a language uses them; they were `idiomatic.arch` templates, and nothing outside `languages/` named them.

**.** Every language test is the engine's, written once in `crates/language` and the SDK (compile-fail, conformance, allocation, call-count): a language holds only its answers and its fixtures. `tests/conformance.rs` and `tests/allocation.rs` are the spec's templates placed closed, exactly as rendered, so any test logic added to a language is a `#template` finding (fixtures `conformance-with-extra-test`, `allocation-with-extra-logic`). The conformance runs the refusals and the flow of the contract over the samples of the fixtures; the allocation test compares reading each sample with the language's `Parse::baseline`. The compile-fail cases are the engine's (`crates/language/tests/ui`) and are not placed in a language.

The typed fixtures are placed: `src/fixtures.rs` and each file beneath `src/fixtures/`, from the open `language-fixtures` template.

**.** `parse` and `project` are contract parts, not use cases: the spec places the `contract` spec at `src/inbound/contracts/parse` (`trait = "ryo::languages::Parse"`) and `src/inbound/contracts/project` (`ryo::languages::Project`), each exactly once, with the list `src/inbound/contracts.rs` (template `contracts`, closed: one `pub mod` per placed contract) and each contract's module file (`contract-module`, `count = 2`). The use-case findings on languages (`use-case-test#template`, `use-case-methods`, `use-case#template`, `use-case#closed`, `interface#template`, `types#template`, `interface-shape`) do not apply to a contract. The product's tastemaker.toml places the language's nu-hex with `implements = []` and no `use-case` entries. Fixtures: `project-contract-missing`, `project-contract-not-implemented`, the demo language's contracts in `complete`, and the templates' own.

`language-conformance-test` passes the manifest directory as `ryo::languages::contract::ManifestDir(env!("CARGO_MANIFEST_DIR"))`, a type, not a `&str` (coordinator, decl second wave). The SDK contract owns `ManifestDir`.

**Final review round.** `language-conformance-test` and `language-allocation-test` render the real call: three arguments to `conformance` (`ManifestDir`, `&Fixtures`) and `ManifestDir(..)` to `allocations_of_linked`, in rustfmt's layout and with Rust's doc line, so Rust's files and every scaffold's are byte-identical to them. `language-tastemaker` now requires the language's own `client-at-adapter` placement (its `libraries` = the parser and tool crates of its `imports-allow`); the product-level list no longer names any language's crates, so a new language's parser cannot escape its adapter silently. The `contract` template renders `pub struct <Part>Contract;` and `#[must_use] pub fn new() -> Self`. `language-parts` detects manifest items by the directory `src/domain/manifests/collections/` (what `ryo::reads!` leaves), and its fixtures use the real `ryo::reads!` shape.

- The two contracts' module files are two entries (`parse-module`, `project-module`) at their fixed paths, each with its `contract` param, instead of one entry with a `{contract}` capture: `tm workspace scaffold` cannot bind a capture no existing file gives, so the capture entry wrote neither module file of a new language and the explicit entries write both.

**Suite, corpus and bench (approved rails).** A language holds its official test suite, pinned, and real-world files: the new open template `language-suite` places `tests/fixtures/suite/README.md` (the suite's source and the pinned version), and `language-parts` requires a case under `tests/fixtures/suite/` and a file under `tests/fixtures/corpus/`. A language has a performance bench: the new open template `language-bench` places `benches/performance.rs` with a workload registration, the sizes `SIZE` and `GROWTH_LIMIT` and a function `complexity` that reads generated input at N and 4N and fails above about 5 times the time, and the new rule `bench-registers-workloads` holds that at least one workload is a `#[divan::bench]` function. Both templates sit at fixed paths, so `tm workspace scaffold` renders them. Fixtures: `missing-bench`, `missing-suite-readme`, `bench-without-workload`; the others gained both files.

The bench is present exactly when the language has a workload: the `crate` spec no longer requires `benches/performance.rs` (`crate-bench` takes `count = { min = 0 }`), and `bench-registers-workloads` and `bench-checks-growth` read the file when it exists, so an empty bench is never written. `language-parts` asks for no bench. Fixture `complete-without-bench` (a language without one passes).

**Suite cases and corpus by placement.** The new open template `language-tree` (it asks nothing of a file's content) and two entries, `suite-cases` (`tests/fixtures/suite/{*case}`, the README left out) and `corpus-files` (`tests/fixtures/corpus/{*file}`), each at least one. A rule cannot see these directories, because the workspace keeps rules off them; placement sees every file the workspace governs, so a language without a suite case or without a corpus file is a `language-tree(suite-cases)#count` or `language-tree(corpus-files)#count` finding at the language. They replace the two parts `language-parts` held. Fixtures `missing-suite-cases` and `missing-corpus`; the others gained a case and a corpus file.

**One scaffold per part writes a language that compiles.** The engine writes the files of the instance it is asked for and no nested instance, so the language is scaffolded part by part, leaf first (see the entries of this spec and of `contract`, `crate` and `nu-hex`); each part renders compiling code and the findings left are the author's: the kinds (`domain/syntax/types.rs`), the syntax port's method, the rows of tests. `src/fixtures.rs`, `src/fixtures/kinds.rs` and `src/fixtures/relations.rs` are placed from their own templates (`language-fixtures`, `language-kinds`, `language-relations`); the other files beneath `src/fixtures/` are `language-fixture`'s. `language-fixtures` declares `expected`, which the conformance test reads, with no kind, no manifest and the rows of `syntax_tree`; it states what the finished languages share. The suite and the corpus each hold a README (`language-suite`, `language-corpus`) and a NOTICE (`language-notice`) that says what the directory holds and where the files go; the cases of the suite and the files of the corpus (`language-tree`) are any further file, `count = { min = 0 }`, because the conformance already requires them where it reads them (the entries lose the minimum of one that a language could only satisfy by copying a suite). New fixtures: `missing-suite-notice`, `missing-corpus-notice`, `missing-corpus-readme`, `missing-kinds`, `missing-relations`, `crate-of-a-language`, `crate-without-test-targets`, `crate-without-linked`, `concept-modules-private`, `bench-without-generated`, `fixture-expected-without-rows`; `missing-corpus` and `missing-suite-cases` are gone with the minimum. `complete-without-bench`, `bench-without-workload`, `missing-tastemaker`, `missing-suite-readme` are derived from `complete`.

**The growth check runs only in the timing run.** `bench-checks-growth` also requires the call to `complexity` to sit in an `if` that tests `BENCH_FLAG`, the `--bench` argument that `cargo bench` passes and `cargo test` does not, so the gates' test step builds the bench and never times it. Fixture `unguarded` (the call in `main` unconditionally is a finding); the `checked` fixture and every language fixture's bench guard the call.

**The template's tree.** A language is the tree of `language-template`: flat concepts `syntax`, `nodes`, `tokens` (and `extras`, `model`, `language`) under `domain/`, no nesting. The standard the node and token names come from is pinned in `tests/fixtures/standard/`: the new open template `language-standard` places its README (source and version), `language-notice` (`held = "standard"`) its NOTICE, `language-tree` its files (`standard-files`); the workspace keeps rules off them. Each relation fixture is a pair, the input (`relation-inputs`, `language-tree`) and the typed rows (`relation-rows`, the new template `language-rows`, moved here from the `crate` spec's `fixture-tree`, which rendered it for a crate with a conformance test): one of the two without the other is a finding, so the input of `syntax_error`, which the workspace keeps rules off, is counted by placement and `item-fixtures` no longer reads it. Fixtures `missing-standard-readme`, `missing-standard-notice`, `relation-input-without-rows`, `relation-rows-without-input`. What the concepts must hold is the workspace rules `language-parts` (the required files and `STANDARD`) and `extras-iff-variant`.

**Closed.** The spec keeps `closed = false`: it places the parts of a language and not the files of the `crate` and `nu-hex` entries beside it, so closed it would refuse them. The language is closed by those entries: `nu-hex` is closed at the same directory, so a file nothing places is a `#closed` finding at the language (`languages/toml/src/stray.txt` is one), and the one open placement left is the crate's `fixture-tree`, for the data of `allocation/`, `manifests/` and `refusals/`.

**A language proves itself against its library and its standard.** Seven demands, each with a fixture that fails without it.

| demand | held by | fixture that bites |
|---|---|---|
| the walk is the library's own tree, never the adapter's element table | template `language-outbound-syntax-adapters-library-fixtures-walk` (declares `walked`) | `without-walked` |
| `walk.rs` and `oracle.rs` name nothing of the adapter, its schemas or the element table | rule `library-walk-is-its-own` | `imports-the-adapter`, `imports-the-schemas`, `imports-by-crate-path`, `imports-the-elements`, `builds-the-table` |
| the whole tree is compared with the pinned standard's dump, with a negative control | rule `adapter-test-holds-the-standard` | `without-pinned-case`, `without-control`, `without-standard-directory`, `spot-checks-only` |
| every kind of the library is mapped or named as dropped | template `language-outbound-syntax-adapters-library-schemas-test` (declares `library_kinds`, `dropped`, the case over every kind) | `without-every-kind` |
| an answer that the language lacks a relation cites the standard | product rule `absence-cites-the-standard` (`NoSuchConstruct` and `NoManifests`; with it a language has a manifest contract and its project adapter, or says why it has none) | `uncited`, `comment-without-the-standard`, `one-of-two-uncited`, `no-manifests-uncited` |
| the conformance lists each relation answered `NotYetRead` by name | the kernel conformance (`not_yet_read`) | `not_yet_read_test.rs` |
| the README has `## Differences from the standard` | rule `readme-states-its-differences` | `absent`, `wrong-level`, `another-title` |

`language-readme` is an open Markdown template, which the engine asks only to exist, so the section is held by the rule and the template writes it into a new README.

**Project and embedded adapters**. The project port's adapters (`src/outbound/project/adapters/<adapter>/`: npm, pnpm, bun, tsconfig, pep621, uv, poetry, pip, pipenv, cargo_metadata, gomod) and the embedded port's (`src/outbound/embedded/adapters/<adapter>/`: vue, svelte) are placed by six templates, each at least none: `language-project-adapter-module`, `-adapter`, `-adapter-test` and `language-embedded-adapter-module`, `-adapter`, `-adapter-test`. Where a file exists it is held to its template, and `tm scaffold` writes a new adapter from them with the port's contract called from its test. What a language owes is said by `language-parts` (an `adapter_test.rs` beside every project or embedded `adapter.rs`), which holds the embedded port by its own directory: a language without `src/outbound/embedded/` owes nothing for it. The test's use of the contract is the `adapter-contract` rule's, and a `schemas.rs` is the `schemas-members` rule's. Fixtures `project-adapter`, `project-adapter-without-test`, `project-adapter-test-without-contract`, `embedded-adapter`, `embedded-adapter-without-test`.

**Edited in place**: the `language-cargo` template and its fixtures depend on the SDK as `idiomatic-sh-ryo-sdk`.

**The growth check times the fastest of five reads**: `language-benches-performance` writes `ROUNDS` and a `timed` that returns the fastest of `ROUNDS` reads of each size, so one read the rest of the machine slowed never fails the check; `GROWTH_LIMIT` is unchanged.

Scenarios named `*-reaches-a-contract-part` prove the language's rules read the files of its contract parts.

**A string constant is declared once in a language**: `constants-once` is held over each language's own source, and the `tokens` concept names the standard the `nodes` concept declares (`crate::domain::nodes::constants::STANDARD`) where both share it.

Places `no-serde-in-domain` over the language's sources: serde only in an outbound adapter's schemas.

**A scaffolded language compiles warning-free**: `language-outbound-syntax-fixtures-contract` no longer imports the unused `Position`, and `language-lib` names `domain::language::constants::NAME` without the unneeded `crate::`; the rendered fixtures follow. Edited in place.

The benchmark contract crate is allowed as a development dependency by `language-depends-on-ryo-only@0.2.0`.
