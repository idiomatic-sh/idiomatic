# types

## 0.1.0

The `hex` spec 0.10.0's owned template `types`, shared by the `nu-hex` and `adapter-crate` specs. Its content is unchanged; the `hex` spec keeps its own copy until it is retired.

**.** A use case's `types.rs` declares `<UseCase>Request` and `<UseCase>Report`, named from the directory as `errors.rs` declares `<UseCase>Error`: the template writes the two declarations for `kind = "use-case"` and the template check holds the file to them. Other kinds are unchanged.

A use case's `types.rs` no longer prescribes `<UseCase>Request` and `<UseCase>Report`, and an entry's prescribes only `<Entry>Report`: a verb with no input has no request, and a report holds only what a caller reads.

**.** A port's `types.rs` header scaffolds plain types: derives `Clone`, `Debug` and `PartialEq` and no handle, file, lock or `dyn`. The `example-port` fixture carries it.
