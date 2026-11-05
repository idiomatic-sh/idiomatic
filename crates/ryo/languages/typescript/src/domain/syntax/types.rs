//! The types of the `syntax` concept: plain data, public fields and derives, with no constructor
//! and no inherent impl. A kind of node or token is a node, a token or an extra, named in kebab case; the element table holds its number.

use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::TypeScriptNode;
use crate::domain::tokens::types::Token;

/// What a node or a token of a typescript syntax tree is: a node or a token of the standard, or an extra the standard lacks. The element table
/// holds its number: the nodes first, then the tokens, then the extras, each in the order of its enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Kind {
    /// A node.
    Node(TypeScriptNode),
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
    TypeScriptNode,
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
