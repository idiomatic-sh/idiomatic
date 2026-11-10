# public-fields 0.2.0

A struct whose doc holds a `compile_fail` example is not read: a capability only its crate may make, or an opaque handle, proves in
a doctest that its field is unreachable, and rustdoc fails the build when it becomes reachable. Every other private or restricted
field of a struct in a `types.rs` is reported as before.
