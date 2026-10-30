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
}

/// Whether an expression writes a dotted name: a name, or attributes of one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Dottedness {
    /// It does.
    Dotted,
    /// It is any other expression.
    Other,
}

/// Whether a name or an attribute expression is a reference of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Use {
    /// It reads or deletes what it names, and is not part of a longer dotted name that is read.
    Reference,
    /// It binds, or it is part of a longer dotted name, or it writes no dotted name.
    Other,
}

/// What an alias of an import imports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Imported {
    /// `*`: every name of its module, which no file of it lists.
    Everything,
    /// The names it writes.
    Names,
}

/// Whether the value of a string expression is known where it is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Knownness {
    /// A string literal, or an f-string with no interpolation.
    Known,
    /// Any other expression, an interpolated string among them.
    Other,
}

/// Whether an expression is the object of an attribute that is read, and so part of the longer dotted name the attribute writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Reading {
    /// It is the object of an attribute that is read.
    ObjectOfRead,
    /// It stands on its own.
    Own,
}

/// An annotation and what it annotates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Annotation {
    /// The annotation's expression.
    pub annotation: ryo::languages::Position,
    /// The parameter, the function or the assignment target it annotates.
    pub target: ryo::languages::Position,
}

/// A single name an assignment binds to a string whose value is known.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Assignment {
    /// The name expression it binds.
    pub binding: ryo::languages::Position,
    /// The string it binds it to.
    pub value: ryo::languages::Position,
}
