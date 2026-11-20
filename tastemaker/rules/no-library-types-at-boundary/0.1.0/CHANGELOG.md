# no-library-types-at-boundary 0.1.0

The first release.

A type path of the entry's `libraries` or `domain-libraries`, written in what crosses the crate's boundary, is a finding: a
signature of a `pub` item, a public field of a public struct, a variant of a public enum, a public type alias, constant or
static, a member of a public trait, a member of an `impl` of one of the entry's `contracts`, and a `pub use`. A path is
resolved through the file's own imports by its first segment, so an imported name, an import alias and a renamed crate count.
Scenario `library-crosses` (eleven findings, one of each and a newly added module); scenario `library-inside` (none: bodies,
private fields, crate-visible items, a public field of a crate-visible struct, a trait implementation that is no contract).
