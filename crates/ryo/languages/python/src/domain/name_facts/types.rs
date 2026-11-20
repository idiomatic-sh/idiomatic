//! The types of the `name_facts` concept: what an import binds and names, and what a decorator names and is handed.

use ryo::languages::NameText;

/// What one alias of an import binds and what it imports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ImportedName<'b> {
    /// The name it binds; none for `*`, which binds names no file states.
    pub binding: Option<NameText<'b>>,
    /// The dotted path it imports, the dots of a relative import first.
    pub target: NameText<'b>,
}

/// What a decorator names and the arguments it is called with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Decoration<'b> {
    /// The dotted name it writes or calls; none when it is any other expression.
    pub target: Option<NameText<'b>>,
    /// The text between the parentheses of its call; empty when it calls nothing.
    pub arguments: NameText<'b>,
}
