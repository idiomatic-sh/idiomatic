# validation-at-edges

## 0.1.0

The first release. It replaces `private-checked-fields` of the `hex` spec, which made a validated value's fields private and `TryFrom` its one door: validation now exists only at the four edges where the shape of the data is not ours, in a schemas.rs, and every core type is plain. A finding is a `TryFrom` impl, an `impl FromStr` or an inherent function with no receiver returning `Result<Self, _>` (or `Result<Name, _>`), of any visibility, in a file the entry selects; the specs exclude `**/schemas.rs` and test files.
