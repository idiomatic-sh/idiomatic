
A `schemas.rs` with a wire shape has at least one `impl TryFrom<<Name>Schema>` in the same file.

The schemas file also holds the decode function (`pub(super) fn read`) that reads bytes into the core type through its conversion, so the clause that allowed no other function with a body is removed; `schema-confined` keeps the schema names inside this file.

A port's own public type may have one validating `TryFrom<&str>` in schemas.rs, with its error type public; this rule accepts it. Edited in place: the package is frozen.

An `impl From<<Name>Schema> for <CoreType>` converts a lossless wire shape wherever `impl TryFrom<<Name>Schema>` does: it satisfies the at-least-one-conversion requirement, and its source must be a `*Schema` or a `&str` like a `TryFrom` (a `From` into a `*Schema`, the other way, is left alone). Edited in place: the package is frozen.
