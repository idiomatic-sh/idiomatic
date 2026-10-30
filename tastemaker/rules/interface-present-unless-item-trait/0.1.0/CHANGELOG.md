# interface-present-unless-item-trait

## 0.1.0

The first version, owned by the `collection` spec until now: a collection has an `interface.rs` exactly when its items implement the trait it declares.

- **A shared rule, placed over the workspace, and param-free**. A collection spec's rules do not read the files of the item instances nested in it, so the spec-owned rule could see `interface.rs` but not the items, and its `item-trait` param is a list of `<collection>=<trait path>` pairs from which a rule cannot pick the pair of the collection it runs on. The rule reads the evidence in the files instead, which the `item` template holds to the collection's pair: an item that imports `super::super::interface::` while `<collection>/interface.rs` does not exist is a finding (at the item), and an `interface.rs` that no item of its collection imports is a finding (at the file). Placed by an entry of the product's tastemaker.toml with `include = ["<root>/**/collections/*/interface.rs", "<root>/**/collections/*/*/item.rs"]`, as `use-case-module-beside` is.
- Fixtures: `own-trait-with-interface`, `trait-of-another-crate-without-interface`, `reads-call-without-interface`, `interface-one-item-uses` (pass); `own-trait-without-interface`, `trait-of-another-crate-with-interface`, `reads-call-with-interface` (violation).
