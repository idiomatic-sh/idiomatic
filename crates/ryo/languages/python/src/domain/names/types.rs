//! The types of the `names` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// Why a name a row holds cannot be resolved statically: the language's closed list, as `Parse::unknown_reasons` declares
/// it, each with the number it has in the language's range, 16 to 31.
#[derive(Clone, Copy, Debug, PartialEq, Eq, strum::EnumIter, strum::IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
#[repr(u8)]
pub(crate) enum Unresolved {
    /// `from m import *`: the names it binds are the module's.
    WildcardImport = 16,
    /// A decorator that is no dotted name and no call of one: `@handlers[0]`, `@f()()`.
    DynamicDecorator = 17,
    /// A base of a class that is no dotted name and no parameterized one: `class C(bases[0])`, `class C(make())`.
    DynamicBase = 18,
    /// A base of a class unpacked from a sequence: `class C(*bases)`.
    StarBase = 19,
    /// A callee that is no dotted name: `handlers[0]()`, `f()()`, `"x".join(...)`.
    DynamicCallee = 20,
    /// An f-string or a t-string with an interpolation: its value is made when it runs.
    InterpolatedString = 21,
    /// A requirement that writes a URL or a file and no name: the distribution names itself only once it is built.
    UnnamedRequirement = 22,
    /// No proven canonical import or builtin origin exists for this expression.
    QualifiedOrigin = 23,
}

/// Whether an expression writes a dotted name: a name, or attributes of one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Dottedness {
    /// It does.
    Dotted,
    /// It is any other expression.
    Other,
}
