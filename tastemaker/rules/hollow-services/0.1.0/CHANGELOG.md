# hollow-services 0.1.0

The first version. The `crate` spec's `composition` template declared
`pub struct Services {}`, so a composition with no services carried the empty struct, and
Tastemaker reported nothing on it because the struct was the template's own content. No
composition template declares `Services` any more (`crate` 0.12.0 places no composition;
`macro`, `cli` and `sdk` each place their own), and this rule refuses the empty one: a
struct named `Services` with no field.

Placed on every composition a spec places: `macro` 0.4.0 on `src/composition.rs`, `cli`
0.10.0 on each command's, `sdk` 0.1.0 on each function's, and `idiomatic.ryo`'s `language`
on a provider's.

What the syntax cannot see: a `Services` declared by a macro, and one whose only field is
a `PhantomData`.

Fixtures: passing `absent`, `with-services`, `other-struct`; failing `empty-services`,
`unit-services`.
