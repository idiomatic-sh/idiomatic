//! The types of the `declaration_kind` concept: the closed set of words, as an enum.

/// What a declaration declares: the closed set of abstractions every language maps its own
/// declaration kinds onto, keeping its own distinction in the row's refinement.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    strum::IntoStaticStr,
    strum::FromRepr,
    strum::EnumCount,
    strum::EnumIter,
)]
#[strum(serialize_all = "kebab-case")]
#[repr(u8)]
pub enum DeclarationKind {
    /// A free function.
    Function,
    /// A function declared on a type.
    Method,
    /// A class.
    Class,
    /// A record type with named or positional fields.
    Struct,
    /// An enumeration.
    Enum,
    /// One member of an enumeration.
    EnumMember,
    /// A contract of methods a type implements nominally.
    Trait,
    /// A contract of members a type satisfies.
    Interface,
    /// A named type that is none of the kinds above.
    Type,
    /// Another name for an existing type.
    TypeAlias,
    /// A type a declaration is generic over.
    TypeParameter,
    /// A named group of declarations.
    Namespace,
    /// The unit of code a source file declares itself part of.
    Package,
    /// A name an import binds.
    Import,
    /// A named value that may be assigned: a variable or a local binding.
    Variable,
    /// A named value fixed where it is declared.
    Constant,
    /// A name a callable binds to an argument or to a result.
    Parameter,
    /// A data member declared on a type.
    Property,
    /// A named rule that expands into code where it is invoked.
    Macro,
    /// A titled part of a document; sections nest through the scope of the declaration. Code languages never write it.
    Section,
    /// An anchor, an id or a label a document gives a place so that other places can name it. Code languages never write it.
    Label,
    /// A block that implements a type, or a contract for a type, named by its header: an `impl` block. No other linked
    /// language has the construct: a class's members are declared in the class, and a contract it satisfies is `inherits`.
    Implementation,
}
