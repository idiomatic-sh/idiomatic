# mirrored-types 0.1.0

The first release. A struct whose named fields are, as a set, the named fields of another
struct of the same crate is a copy of one concept: Tastemaker's lint answer and the
analysis port's finding, its two unknowns, ryo's undecided rule and the lint's unknown. The
crate keeps one and passes it on.

## What it reads

- The names of each struct's named fields, compared as a set between the structs of one
 package (`file-package`); a struct is compared when it has three or more.
- Left out by design: `schemas.rs` (a wire shape mirrors its core type), `collections/`
 (one shape per item), test code, use-case wiring (`*UseCaseImpl`, `*Ports`), and structs of
 different crates (an SDK's edge type mirrors its core's).

## Not seen

- Two-field copies (a heading and a value): two fields are too often two concepts with the
 same shape to report without a type comparison.
- Copies whose field names differ, and tuple structs.
