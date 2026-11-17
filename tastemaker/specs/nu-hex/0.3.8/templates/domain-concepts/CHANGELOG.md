# domain-concepts

## 0.1.0

The `hex` spec 0.10.0's owned template `domain-concepts`, shared by the `nu-hex` and `adapter-crate` specs. Its content is unchanged; the `hex` spec keeps its own copy until it is retired.

Changed for the part spec. The list is rendered from `instances.part` (the concept parts of kind `concept` the product places in this domain; a part under a hex is told by its `port` capture) instead of from the module files' captures, so a placed concept the list omits is a finding of this template.

Changed for the part spec. The list is rendered from `instances.part` (the concept parts of kind `concept` the product places in this domain; a part under a hex is told by its `port` capture) instead of from the module files' captures, so a placed concept the list omits is a finding of this template.

The `kernel` param: `pub mod <concept>;` for a kernel crate, `pub(crate) mod <concept>;` otherwise. The nu-hex fixtures `kernel-complete` and `kernel-private-concepts` hold both ways: the template alone sees no concept instance.

The `kernel` param is gone and the modules are declared without a visibility, so a kernel crate's `pub mod` and every other crate's `pub(crate) mod` meet the template.
