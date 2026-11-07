# hollow-artifacts

## 0.1.0

The `hex` spec 0.10.0's owned rule `hollow-artifacts`, shared by the `nu-hex` and `adapter-crate` specs. The query is unchanged: it reads declarations, never the layout. Only the words that named the hex or its steps changed; the `hex` spec keeps its own copy until it is retired.

A file whose only item is a top-level macro call (`ryo::reads! {.. }`, `declare!`, `link!`) is not hollow: the declaration is its content. The query already counted `macro-call` among the items; two fixtures (`declaration-macro-file`, `declaration-macro-block-file`) now pin it, and they fail when `macro-call` leaves the item kinds. Edited in place: the package is frozen.

A unit struct that a macro call names is not a placeholder: a language provider's item is `pub(crate) struct ScopeItem;` and the `ryo::reads! { ScopeItem =>.. }` call beside it writes the trait impl, so the impl is not in the text. The query adds, to the identifiers an `impl.. for` names as its self type, the identifiers that stand directly in the token tree of a macro call. Fixtures `unit-named-by-macro` (pass) and `unit-not-named-by-macro` (violation: the call names another struct).
