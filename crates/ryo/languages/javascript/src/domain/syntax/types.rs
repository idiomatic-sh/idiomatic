//! The types of the `syntax` concept: plain data, public fields and derives, with no constructor
//! and no inherent impl. A kind of node or token is a node, a token or an extra, named in kebab case; the element table holds its number.

use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::tokens::types::Token;
use ryo::languages::{ByteSlice, Bytes, Count};

/// What a node or a token of a javascript syntax tree is: a node or a token of the standard, or an extra the standard lacks. The element table
/// holds its number: the nodes first, then the tokens, then the extras, each in the order of its enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Kind {
    /// A node.
    Node(Node),
    /// A token.
    Token(Token),
    /// A kind the standard lacks.
    Extra(Extra),
}

/// Whether an element is a token, a leaf, or a node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Leaf {
    /// A token.
    Token,
    /// A node.
    Node,
}

/// Whether a function of the tree is declared: a function declaration, or the body of a method, which its key names; a function
/// expression or an arrow written as a value declares nothing, and is no row of `function` (its parameters are still `parameter` rows).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Declaredness {
    /// A function declaration.
    Function,
    /// The body of a method, a getter or a setter, with its key when the key is a name and not computed.
    Method(Option<ryo::languages::Position>),
    /// A function expression or an arrow written as a value.
    Expression,
}

/// How the text of a file is read: the goal ECMA-262 parses it for, a module or a script, or a module when it writes module syntax and a
/// script when not, with JSX read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Goal {
    /// An ECMAScript module, whatever its syntax: a `.mjs` file.
    Module,
    /// A `CommonJS` script, read as a script rather than a module: a `.cjs` file.
    Script,
    /// Module syntax detected and JSX read, as Babel and the React toolchains read `.js`: a `.js` or `.jsx` file, and the blocks of
    /// javascript a file of another format carries.
    Detected,
}

/// What a block of a file of another format is written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Written {
    /// Script: what this language reads.
    Script,
    /// A typed language, which the typed language reads.
    Typed,
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
    /// The file as it is, for its goal.
    Own(Goal),
    /// The script the blocks of the file make, with JSX read, as a `.js` file is.
    Script(Bytes),
}
