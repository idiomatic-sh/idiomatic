//! The types of the `refinements` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// What a declaration is, in Rust's own words: each has a refinement name and a shared kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Declared {
    /// A free function.
    Function,
    /// A `struct`.
    Struct,
    /// An `enum`.
    Enum,
    /// A `trait`.
    Trait,
    /// A single-name `let`.
    Variable,
    /// A `const`.
    Const,
    /// A `static`.
    Static,
    /// A `mod`.
    Module,
    /// A `type` alias.
    TypeAlias,
    /// A `union`.
    Union,
    /// A `macro_rules!` definition.
    MacroRules,
    /// A `macro` definition.
    Macro,
    /// A function of an `impl` or a `trait` with a `self` receiver.
    Method,
    /// A function of an `impl` or a `trait` without one.
    AssociatedFunction,
    /// A variant of an `enum`.
    Variant,
    /// A named field.
    Field,
    /// A positional field.
    TupleField,
    /// A generic type parameter.
    TypeParameter,
    /// A lifetime parameter.
    LifetimeParameter,
    /// A const generic parameter.
    ConstParameter,
    /// A name a `use` brings in.
    Use,
    /// A name an `extern crate` brings in.
    ExternCrate,
    /// An `impl` block, named by its header.
    Implementation,
    /// The label of a loop or a block, `'outer: loop {}`.
    Label,
}
