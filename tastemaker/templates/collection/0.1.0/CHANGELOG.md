# collection

## 0.1.0

The module file of a collection rewritten for it declares the collection's interface and lists the items in order. It takes no params: the level is the spec instance's, and the optional members (the shared cases, types, errors and their tests) are held by the shared rule `module-declares`.

The template takes a required `kernel` param: `kernel` renders `pub mod` for the modules it declares, `provider` and `absent` render `pub(crate) mod`.

The `kernel` param is gone and the interface and item modules are declared without a visibility. Shared by the `concept` and `item` specs.

The template takes `item_trait`, the crate's `<collection>=<trait path>` pairs: items are declared `pub(crate) mod` when a pair names a trait of the language contract, and without a visibility otherwise.
