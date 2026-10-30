
The edge and composition lists forbid a domain service of the edge's own crate (`crate::domain::*::service*`), not of a kernel crate: any crate may import the kernel's domain.

The use-case lists forbid the implementation structs of `use_case.rs` (`*::use_case::*Impl`), not the whole file: the `*Engine` traits of the language crate, which let an adapter call a language's use case through an interface, are declared there beside the blanket impls.

A path that starts `::crate::` is the path `crate::`: the `forbidden` globs of the `idiomatic-rust` spec's entries are written `{,::}crate::...`, so a leading colon does not hide an import from the directions. Fixtures `port-to-use-case-leading-colon`, `port-to-domain-leading-colon`.

The `forbidden` list keeps a module out of the parts that must not name it: the `idiomatic-rust` spec's hex directions name no module of the command line where a typed cell becomes text (`output::schemas`). Fixtures `core-names-value-rendering` (fails), `core-hands-typed-cells` (passes).
