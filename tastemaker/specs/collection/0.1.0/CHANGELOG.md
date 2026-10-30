# collection

## 0.1.0

The first version, split from the retired `part` spec by kind. It lists its own files with their counts, shares the templates and rules the other kinds use, and is placed by the crate kind that holds it, so a product's tastemaker.toml names only the crate and its crate kind.

**.** The collection's shared cases move from `interface_test.rs` to `fixtures/contract.rs` beside `fixtures.rs`, and the `item` spec leaves out the `fixtures` directory; the collection's types and errors tests are deleted.

- The kind-suffix entry passes `exposed = []`.

- **`item-trait`**: a required param, the path of the trait every item implements when it is a trait of another crate (`ryo::languages::ReadItem`, `ManifestItem`), or `absent` when the collection declares its own `<Collection>Item`. The `collection-interface` template is `count = { min = 0 }` (a count cannot depend on a param) and the rule `interface-present-unless-item-trait` holds both directions: an `interface.rs` missing while the param is `absent` is a finding (at each item's module file), an `interface.rs` present while it names a trait is a finding. The `collection` module templates of the concept and item specs declare `mod interface;` only when the file is present. The collection passes the param to the item spec, which holds each item to the trait (`item-implements-trait`). Fixtures: `item-trait-named`, `item-trait-named-with-interface`, `item-trait-not-implemented`, the four of the rule, and `collection-without-interface` now expects the rule's finding instead of the count's.

- **`item-trait` is a list of `<collection>=<trait path>` pairs, and a `kernel` param**. The collection passes both to the item spec. The `item-module` template takes `kernel`: `pub mod item;` in a kernel crate, `pub(crate) mod item;` elsewhere, as the files of `crates/language` have them. The rule `interface-present-unless-item-trait` leaves this spec and becomes a shared, param-free rule placed over the workspace by `tastemaker.toml` (a collection spec's rules do not read the files of the item instances nested in it): its fixtures `collection-without-interface` and `item-trait-named-with-interface` move to the rule's own (`own-trait-without-interface`, `trait-of-another-crate-with-interface`); the fixtures that placed the collection with `item-trait = "absent"` pass `[]` and `kernel = "absent"`, and `item-trait-named` passes the pair `shapes=ryo::languages::ReadItem`.

- **The `kernel` param is gone** `item-module` declares `mod item;` without a visibility, so any visibility meets it; the `collection-*` templates are as before.

- **`test-runners-match-rows`** is placed beside `test-shape`: a test file has a `run(CASES, Word::<word>, subject)` runner exactly for each case word its `CASES` table has rows of. The test templates (`unit-test`, `item-test`, `schemas-test`, `composition-test`, `command-test`, `entry-test`, `use-case-test`, `adapter-test`) no longer render all four runners: an empty table has no runner. The fixtures `*-without-rejects-runner` became `*-with-rejects-row-without-runner`.

The `item-module` template declares an item's code `pub(crate) mod item;` when the collection names a trait of the language contract (`item-trait`), and `mod item;` otherwise; the entry passes the param. The shared cases the `collection-contract` template scaffolds assert that they have no row yet, the way a port's contract does, instead of `todo!`. Fixture `provider-item-module-private`.
