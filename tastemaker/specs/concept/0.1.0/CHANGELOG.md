# concept

## 0.1.0

The first version, split from the retired `part` spec by kind. It lists its own files with their counts, shares the templates and rules the other kinds use, and is placed by the crate kind that holds it, so a product's tastemaker.toml names only the crate and its crate kind.

`types_test.rs` and `errors_test.rs` are deleted; the `collection` and `collections` templates are the concept's own.

- The `collection` module template declares the collection's types, errors, contract fixtures and items.

- **`constants.rs`** (`names`, `count = { min = 0 }`): the concept's text constants, the one file where `no-string-literals` allows literals; the template is open (asks only that the file exist).

The `names` template moved into this spec: only `concept` places it, and a template only one spec names is the spec's own.

- **`item-trait`**: a required param passed to the collections of the concept (the collection spec's `item-trait`). Fixtures `concept-with-item-trait-collection` and `item-trait-collection-with-interface-module`; the existing fixtures place the concept with `item-trait = "absent"`.

A concept that declares no function has no `service.rs` and no `service_test.rs`, and one that declares a function has both: `service` is `min = 0`, `service_test` is required when `service.rs` is present (the condition `has-service`), a `service.rs` must declare a function (`service-has-fn`), and no function is declared in `types.rs`, `errors.rs` or `constants.rs` (`fns-only-in-service`). The concept-module template declares `mod service` and `mod service_test` only when they exist. Fixtures `types-only-concept` and `fn-in-types`.

`fns-only-in-service` covers `types.rs` and `errors.rs`; `constants.rs` may hold the lookups over its constants (an `operator_named` over a table), which are not the concept's verbs.

- **`item-trait` is a list of pairs, and a `kernel` param**: both are passed to the collections of the concept, and the `collections` and `collection` (collection-module) templates take `kernel`: a kernel crate's concept declares `pub mod <collection>;` and, in the collection module, `pub mod interface;` and `pub mod <item>;`, as `crates/language/src/domain/relations/collections` does; every other crate keeps `pub(crate) mod`. The template fixtures see no captured collections, so the cases are the spec fixtures `kernel-concept-public` and `kernel-concept-crate-private`. The fixture `item-trait-collection-with-interface-module` moves to the shared rule `interface-present-unless-item-trait`.

- **The `kernel` param is gone, and no variant spec replaces it** The collection and item modules are declared without a visibility in the shared `collections` and `collection` templates: a template that names a visibility is compared exactly and one that names none accepts any, so a kernel crate's `pub mod` and a feature crate's `pub(crate) mod` both meet it. Visibility below the library root is no longer a template finding. The `kernel-concept-public` and `kernel-concept-crate-private` fixtures are gone.

- **`test-runners-match-rows`** is placed beside `test-shape`: a test file has a `run(CASES, Word::<word>, subject)` runner exactly for each case word its `CASES` table has rows of. The test templates (`unit-test`, `item-test`, `schemas-test`, `composition-test`, `command-test`, `entry-test`, `use-case-test`, `adapter-test`) no longer render all four runners: an empty table has no runner. The fixtures `*-without-rejects-runner` became `*-with-rejects-row-without-runner`.

A concept may have a `fixtures.rs` (the `concept-fixtures` template, owned and open, optional): the test support its tests and the tests of the concepts that read through it build their subjects from. Its module file declares it under the `fixtures` gate (`module-declares`), and no runtime file reads it (`no-runtime-fixtures`, which already leaves `fixtures.rs` out). Fixtures: the template's `example`, `extra-content` and `missing`, and the spec's `concept-with-fixtures`.

The `collections` and `collection-module` entries pass the concept's `item-trait` to their templates, which declare a collection and its items `pub(crate) mod` when a pair names a trait of the language contract (`ryo::languages::`), and as before otherwise. Fixtures `provider-collections-private`, `provider-collection-items-private`. The unit test the concept scaffolds (`unit-test`) holds a `succeeds` runner whose subject answers a failure until the author states it, so the scaffold compiles and a test with no rows fails by its name instead of by `todo!`.

`service-has-fn` counts a `macro_rules!` declared in the service as one of its verbs: a macro writes the functions of the crates that call it, so a service whose one verb is a macro is not hollow (the language kernel's `kinds!`, which writes every language's kind numbering). Fixtures `with-macro` (pass) and `with-macro-call` (a macro called, none declared: violation).
