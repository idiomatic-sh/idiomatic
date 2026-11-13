# concept-module

## 0.1.0

The `concept-module` template of the `hex` spec 0.10.0, renamed for the concept's file is `service.rs` with `service_test.rs`, and the concept's own `types.rs` and `errors.rs`, each with its test, are declared here when the spec's entries (`concept-types`, `concept-types-test`, `concept-errors`, `concept-errors-test`) place them.

It also declares the concept's own `collections.rs` (`pub(crate) mod collections;`) when the spec's entry `concept-collections` places one, which the first cut left to the author.

Changed for the part spec. The module file declares `service` and `service_test` only. The optional types, errors and collections members (with their tests) are no longer rendered from `present` of sibling entries, which lived in the namespace spec and are now the concept part's: `module-declares` holds them.

Changed for the part spec. The module file declares `service` and `service_test` only. The optional types, errors and collections members (with their tests) are no longer rendered from `present` of sibling entries, which lived in the namespace spec and are now the concept part's: `module-declares` holds them.

The `kernel` param is gone and the members are declared without a visibility, so `pub mod` and `pub(crate) mod` both meet the template. The module reads `present["service"]`.

The `kernel` param is gone: the members are `pub(crate)`, and the kernel crates place `concept-module-kernel`. The module reads `present["concept-service"]`, the name of the entry that now places the service (the shared template `concept-service`).
