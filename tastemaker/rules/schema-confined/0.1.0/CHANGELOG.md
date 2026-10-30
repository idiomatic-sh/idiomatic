# schema-confined 0.1.0

A name ending `Schema` or `SchemaError` is written only in a `schemas.rs` and its `schemas_test.rs`. The decode function lives in `schemas.rs` beside the shapes, so an adapter, a command or an entry calls it and receives core types; a command's clap argument struct is `<Command>ArgumentsSchema` in the command's `schemas.rs` ("sure"). It is the other half of `schema-name`, which no longer forbids a function with a body in `schemas.rs`.

 a port's own public type may have one validating `TryFrom<&str>` in schemas.rs, with its error type public; this rule accepts it. Edited in place: the package is frozen.
