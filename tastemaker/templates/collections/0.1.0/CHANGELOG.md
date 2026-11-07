# collections

## 0.1.0

A collections list that declares each collection it lists. It takes no params: a collections namespace appears once per spec instance, so the collections it lists are the `collection` captures of its instance.

The template takes a required `kernel` param: `kernel` renders `pub mod` for the modules it declares, `provider` and `absent` render `pub(crate) mod`.

The `kernel` param is gone and the modules are declared without a visibility. Shared by the `concept` and `item` specs.

The template takes `item_trait`: a collection is declared `pub(crate) mod` when a pair names a trait of the language contract, and without a visibility otherwise.
