# use-case

## 0.1.0

The first version, split from the retired `part` spec by kind. It lists its own files with their counts, shares the templates and rules the other kinds use, and is placed by the crate kind that holds it, so a product's tastemaker.toml names only the crate and its crate kind.

**.** `types_test.rs` and `errors_test.rs` are deleted.

- The use case's struct is `<UseCase>UseCaseImpl`; the interface and kind-suffix entries pass `exposed = []`.

- A `use-case` takes the nu-hex `implements` list. A use case it names has no `interface.rs`, `types.rs` or `errors.rs` (the `implemented` condition, answered by the `implemented-use-case` rule from its body; `implemented-shape` refuses the files), its `use_case.rs` renders `impl <crate>::<Interface> for <Uc>UseCaseImpl` and its `use_case_test.rs` (the owned `use-case-test` template) runs that crate's `fixtures::contract::CASES`.

- A use case other crates implement carries `fixtures.rs` and `fixtures/contract.rs` (optional; the owned `use-case-fixtures` and `use-case-contract` templates): the rows every implementation runs, which the implementing crate's `use_case_test.rs` imports.

- An implementing use case's test runs `<crate>::<module of the interface>::contract::run(&UseCaseImpl::new())` (for `ryo` / `languages::Language`: `ryo::languages::contract::run`), then its own cases; the fixtures use `ryo` / `languages::Language`.

- An implementing use case's test calls the contract entry named for the trait (`contract::parse` for `languages::Parse`, `contract::language` for `languages::Language`).

- The template of a `languages::Parse` use case answers `recognizes`, `kind_count`, `build` and one method for each syntax relation of the catalog, and the template of a `languages::Project` use case answers `manifests`, `resolve` and one method for each manifest relation; those two traits are all of ryo a language implements, so the entry's `implements` requires every relation by itself. The fixture of the generic implementation names a neutral trait, not a language's.

### SDK-facing contract use cases

A kernel crate's `parse` and `project` use cases are the contract other crates implement: their trait declares several methods, their body has no constructor, they have a `service.rs` that drives implementations, its test, and test support under `fixtures/`. The spec takes a required `methods` param (`one` from `nu-hex`, `many` from `kernel-crate`): `use-case-methods` reads a `many` use case only when it lacks a `fixtures/contract.rs`, the `use-case` template renders a constructor-less body for `many`, and the new templates `use-case-service` (`service.rs`), `unit-test` (`service_test.rs`) and `use-case-fixture` (`fixtures/{fixture}.rs`) place the extra files. The new rule `contract-only-files` keeps them to a use case that has a `fixtures/contract.rs`, so no feature-crate use case grows a service. Fixtures: `use-case-methods` (`contract-several-methods`, `many-without-contract-file`, `one-with-contract-file`), `contract-only-files` (five), the `use-case` template (`contract`, `contract-struct-misnamed`) and the three new templates.

- **`test-runners-match-rows`** is placed beside `test-shape`: a test file has a `run(CASES, Word::<word>, subject)` runner exactly for each case word its `CASES` table has rows of. The test templates (`unit-test`, `item-test`, `schemas-test`, `composition-test`, `command-test`, `entry-test`, `use-case-test`, `adapter-test`) no longer render all four runners: an empty table has no runner. The fixtures `*-without-rejects-runner` became `*-with-rejects-row-without-runner`.

### Test support of any depth, and constants

A contract use case's `fixtures/` holds any file or directory of test support, not one level of files: the `use-case-fixture` entry places `fixtures/{*fixture}` (its `fixture` param is the path beneath `fixtures/`, with its extension), so `fixtures/expected_rows/function.rs` is placed like `fixtures/rows.rs`. A use case may have a `constants.rs` (the `constants` template, optional) for the names its service uses. `contract-only-files` reaches both: a `constants.rs` and a file at any depth of `fixtures/` need the use case's `fixtures/contract.rs`. Fixtures: `constants-without-contract` and `nested-fixture-without-contract` (findings), the `contract-with-service` rule fixture and the `sdk-contract-use-case` spec fixture carry both, `nested-support-file` for the template.

### Test support without a contract

`fixtures.rs` (the `use-case-fixtures` template) declares `pub mod contract;` only when the use case has a `fixtures/contract.rs`; a use case without one keeps the scenario its tests share in a `fixtures.rs` that declares nothing of the contract. `contract-only-files` still reaches the files beneath `fixtures/`, `service.rs` and `constants.rs`. Fixtures: `use-case-with-scenario-fixtures` (passes) and `contract-fixtures-without-contract-module` (finding).

`use-case-declares`, at `error`, over `use_case.rs`: the file declares only `<Name>UseCaseImpl`, its `<Name>Ports` bundle and their impls. A free helper moves to `service.rs`, a type to the domain concept it belongs to; a type that names a port's or the use case's own types goes to the use case's `types.rs`.

`contract-only-files` no longer reaches `constants.rs`: any use case may name its constants, its cache bounds among them (`bounded-engine-state`). Fixture `constants-without-contract` now passes.
