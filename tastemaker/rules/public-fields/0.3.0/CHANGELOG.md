# public-fields 0.3.0

A field must be at least as visible as its struct, where 0.2.0 asked every field to be `pub`: a `pub(crate)` struct with `pub(crate)` fields, an engine's record sealed to its crate, passes, and making its fields `pub` would have undone the seal. A field narrower than its struct is still reported: a private or `pub(crate)` field of a `pub` struct, a private or `pub(super)` field of a `pub(crate)` struct. Scenarios `crate-sealed-type-with-crate-fields` (passes) and `crate-sealed-type-with-a-narrower-field` (reported); the 0.2.0 scenarios hold unchanged. Everything else, the `compile_fail` proof among it, is as in 0.2.0. 0.2.0 is removed with this version, not kept as published: no lock names it once every spec ryo places names 0.3.0, nothing has been released, and an unreleased version keeps no copy for compatibility.

## 0.2.0

A struct whose doc holds a `compile_fail` example is not read: a capability only its crate may make, or an opaque handle, proves in
a doctest that its field is unreachable, and rustdoc fails the build when it becomes reachable. Every other private or restricted
field of a struct in a `types.rs` is reported as before.
