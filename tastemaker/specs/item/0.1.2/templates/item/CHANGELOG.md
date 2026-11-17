# item

## 0.1.0

The `item` template renders per collection and per crate role: see the spec's CHANGELOG for the `item_trait` pairs and the `kernel` param (`kernel`: `pub struct <Part>;`; `provider`: `pub(crate) struct <Part>Item;` with no import and no impl, the `reads!` call writes it; `absent`: `struct <Part>Item;`).

The provider branch is rendered too: when the collection's pair names a `ReadItem`, the template asks for `use ryo::languages;` and the struct `pub(crate) struct <Part>Item;` and scaffolds, beside them, the skeleton of its `ryo::reads! { <Part>Item => languages::<Part> |tree, at, _symbols| { kinds: every, row { } } }` call (the template compares declarations and not a macro's tokens, so the call itself is held by the rule `item-implements-trait`), so `tm workspace scaffold` writes `item.rs` of a language item before it exists (the template reads the pair and the part's name, nothing the file would give). The relation is the part's PascalCase. A pair naming another trait (`ManifestItem`) still asks only for the doc line: its `reads!` arm differs by what the language derives and is the author's.

The language branch (`ReadItem`) renders a `reads!` call that compiles: it reads every element and says the relation is read only in part, in place of a `row { }` with no column.
