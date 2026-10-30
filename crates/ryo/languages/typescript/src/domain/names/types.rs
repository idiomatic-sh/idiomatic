//! Names and bindings read from TypeScript syntax.
use ryo::languages::Position;
/// The exact spelling of one syntax element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Spelling<'a>(pub &'a str);
/// Whether an occurrence is a reference use or a binding/key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Reference {
    /// The name is read where it occurs: a reference.
    Use,
    /// The name is declared or used as a key where it occurs.
    Binding,
}
/// A binding and the string literal assigned to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Assignment {
    /// The name being bound.
    pub binding: Position,
    /// The string literal it is bound to.
    pub value: Position,
}
