# storage-shapes-in-schemas 0.1.0

The first release. Core holds plain Rust values, and the conversion to stored bytes happens only in the outbound adapter, with serde, in its `schemas.rs`. A string table is a storage shape, found in any file but the adapters' (`**/outbound/*/adapters/**`, `**/schemas.rs`).

## What it reads

- A struct named `*Table`, `*Pool`, `*Interner` or `*Strings` that names `Vec` and `String` or `str`.
- A record field named `strings`, `interned`, `interner`, `pool`, `table`, `*_table`, `*_pool` or `*_strings` whose type names `Vec` and `String` or `str`.
- A struct named `*Ref` that names an integer type, in a package that also declares such a table outside the adapters.

## Not seen

- Byte offsets, spans and indices: a source offset or an in-memory position is a domain value, and a position into stored bytes cannot be told from it by name.
- A table of another element type, a table named otherwise, a map used as an interner, a table reached through a type alias, and a reference named otherwise (`Slot(u32)`).

Placed for every file but an outbound adapter's own `schemas.rs` (or its `schemas/`): an adapter's implementation files and a `schemas.rs` of the core are held to the rule too. Fixtures: `adapter-implementation-string-table` and `core-schemas-string-table` (findings).
