// ignore: idiomatic.arch/collection-directory - the six vocabularies share strum's derives to name their words, but each is a closed set of its own (declarations, parameters, values, mappings, dependencies, their origins), not items of one collection
//! The types of the `terms` concept: plain data, public fields and derives, with no constructor
//! and no inherent impl. Each closed set is an enum, so a language that answers with a word the
//! set does not have does not compile; the word is text only where the answer leaves the engine.

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

/// How a parameter takes its argument.
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
pub enum ParameterKind {
    /// The value a method is called on.
    Receiver,
    /// Passed by position only.
    Positional,
    /// Passed by position or by name.
    PositionalOrKeyword,
    /// Passed by name only.
    KeywordOnly,
    /// Collects the remaining positional arguments.
    VariadicPositional,
    /// Collects the remaining named arguments.
    VariadicKeyword,
    /// A named result the callable returns.
    Result,
}

/// What a value of a data document is.
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
pub enum ValueKind {
    /// Text.
    String,
    /// A whole number.
    Integer,
    /// A number with a fraction.
    Float,
    /// True or false.
    Boolean,
    /// A date, a time, or both.
    Datetime,
    /// An ordered list of values.
    Sequence,
    /// A mapping written inline as a value.
    InlineMapping,
    /// A mapping the key opens.
    Mapping,
    /// A list of mappings the key opens.
    SequenceOfMappings,
    /// The value that says there is none: JSON's `null`. TOML has no such value.
    Null,
}

/// How a mapping of a data document is written.
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
pub enum MappingKind {
    /// Opened by its own header.
    Standard,
    /// Never written, only implied by a longer header.
    Implicit,
    /// Opened by a dotted key.
    Dotted,
    /// Written inline as a value.
    Inline,
    /// One member of a list of mappings.
    SequenceMember,
}

/// How a package depends on another.
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
pub enum DependencyKind {
    /// Needed to build and to run.
    Normal,
    /// Needed to test.
    Dev,
    /// Needed to build only.
    Build,
    /// Expected of whoever uses the package.
    Peer,
}

/// Where a dependency says the package it needs comes from, as its manifest writes it: how the package is found, not whether the
/// repository holds it (the row's `target` says that). A requirement that names a package by a version range is `registry` even when a
/// workspace member of that name answers it.
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
pub enum DependencyOrigin {
    /// A package index, by the package's name and a version.
    Registry,
    /// A directory, by its path.
    Path,
    /// A git repository, by its URL.
    Git,
    /// An archive, by its URL.
    Url,
    /// A member of the manifest's own workspace, by its name.
    Workspace,
}

/// Where each closed set's words begin among the numbers a name can have: a word is a name that
/// no file has to write down, so it is found by its number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WordBlock {
    /// The words of [`DeclarationKind`].
    Declaration,
    /// The words of [`ParameterKind`].
    Parameter,
    /// The words of [`ValueKind`].
    Value,
    /// The words of [`MappingKind`].
    Mapping,
    /// The words of [`DependencyKind`].
    Dependency,
    /// The reasons a language declares for a value it cannot resolve statically, by their place in its list.
    Unknown,
    /// The words of [`DependencyOrigin`].
    Origin,
}

/// The words a column of a catalog relation can hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vocabulary {
    /// Any name a file wrote, or any number.
    Open,
    /// One of a closed set of words, which the block says.
    Closed(WordBlock),
}

/// Why a language cannot say a value of a column: the number of a reason in the closed list the language declares, within the range
/// the language has of the block of reasons.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnknownReason(pub u8);

/// A reason a language declares for a value it cannot resolve statically: its number, which is unique among the languages linked
/// together (each language has a range of its own, in `docs/language-contract.md`), and its name, `wildcard-import`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnknownName {
    /// The number the symbol of an unknown value of this reason has in the block of reasons.
    pub reason: UnknownReason,
    /// What the reason is called.
    pub text: ReasonName,
}

/// What a reason for an unknown value is called.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReasonName(pub &'static str);

/// A closed set of words.
pub trait Term: Copy {
    /// The block of numbers the set's words are in.
    const BLOCK: WordBlock;

    /// The word's number within its set.
    fn number(self) -> ChildOrdinal;

    /// The word as text, for where it leaves the engine.
    fn text(self) -> NameText<'static>;
}

/// Whether a symbol is a word of a closed set, or a name a file wrote.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wordness {
    /// A word.
    Term,
    /// A name a file wrote.
    Name,
}

/// A name, interned once: the same text is the same symbol everywhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Symbol(pub u32);

/// The place of an element among its parent's children.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChildOrdinal(pub u32);

/// The text of a name, borrowed: the bytes of the file it is in, or the derived text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NameText<'a>(pub &'a str);
