//! The types of the `syntax` concept: plain data, public fields and derives, with no constructor
//! and no inherent impl. A kind of node or token is a node, a token or an extra, named in kebab case; the element table holds its number.

use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::tokens::types::Token;
use ryo::languages::{ByteSlice, Bytes, Count};

/// What a node or a token of a typescript syntax tree is: a node or a token of the standard, an extra the standard lacks, or an element
/// of a regular-expression pattern the file carries. The element table holds its number: the nodes first, then the tokens, then the
/// extras, then the pattern elements, each in the order of its enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Kind {
    /// A node.
    Node(Node),
    /// A token.
    Token(Token),
    /// A kind the standard lacks.
    Extra(Extra),
    /// An element of a parsed regular-expression pattern, kept under the element that carries it.
    Pattern(PatternNode),
}

/// Whether an element is a token, a leaf, or a node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Leaf {
    /// A token.
    Token,
    /// A node.
    Node,
}

/// How the text of a file is read, as its extension names it: TypeScript, TypeScript with JSX, TypeScript that is always a module or
/// always `CommonJS`, and the declarations of each.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Dialect {
    /// A `.ts` file, and any file no other dialect names: a module or a script as its imports and exports say.
    TypeScript,
    /// A `.tsx` file: TypeScript with JSX.
    Tsx,
    /// A `.mts` file: always a module.
    Module,
    /// A `.cts` file: always `CommonJS`.
    CommonJs,
    /// A `.d.ts` file: declarations alone.
    Declarations,
    /// A `.d.mts` file: the declarations of a module.
    ModuleDeclarations,
    /// A `.d.cts` file: the declarations of `CommonJS`.
    CommonJsDeclarations,
}

/// What a block of a file of another format is written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Written {
    /// Plain script, which the script language reads.
    Script,
    /// TypeScript.
    TypeScript,
    /// TypeScript with JSX.
    Tsx,
}

/// A block that a file of another format carries: where its text starts in the file, the text and what it is written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Stretch<'a> {
    /// Where the text starts in the file, in bytes.
    pub start: Count,
    /// The text of the block.
    pub text: ByteSlice<'a>,
    /// What the block is written in.
    pub written: Written,
}

/// What the text of a file is read as.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Scripting {
    /// The file as it is, in the dialect its name gives.
    Own(Dialect),
    /// The script the blocks of the file make, in the dialect the blocks together call for.
    Script(Bytes, Dialect),
}

/// What an element of a parsed regular-expression pattern is, named as the regular-expression grammar of the language's library names
/// it, in kebab case. One closed list, each name once.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    strum::EnumIter,
    strum::IntoStaticStr,
    strum::FromRepr,
    strum::EnumCount,
)]
#[strum(serialize_all = "kebab-case")]
#[repr(u16)]
pub(crate) enum PatternNode {
    /// The whole pattern.
    Pattern,
    /// Alternatives separated by `|`.
    Disjunction,
    /// One alternative: a sequence of terms.
    Alternative,
    /// `^`, `$`, `\b` or `\B`.
    BoundaryAssertion,
    /// `(?=…)`, `(?!…)`, `(?<=…)` or `(?<!…)`.
    LookAroundAssertion,
    /// A term with `*`, `+`, `?` or `{…}`.
    Quantifier,
    /// One character, written or escaped.
    Character,
    /// `.`.
    Dot,
    /// `\d`, `\D`, `\s`, `\S`, `\w` or `\W`.
    CharacterClassEscape,
    /// `\p{…}` or `\P{…}`.
    UnicodePropertyEscape,
    /// `[…]` or `[^…]`.
    CharacterClass,
    /// `a-z` inside a character class.
    CharacterClassRange,
    /// `\q{…}` inside a character class.
    ClassStringDisjunction,
    /// One string of a `\q{…}`.
    ClassString,
    /// `(…)` or `(?<name>…)`.
    CapturingGroup,
    /// `(?:…)`, with its modifiers.
    IgnoreGroup,
    /// `\1`.
    IndexedReference,
    /// `\k<name>`.
    NamedReference,
}
