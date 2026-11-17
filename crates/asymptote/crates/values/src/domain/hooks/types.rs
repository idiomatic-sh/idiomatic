//! The types of the `hooks` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// The text of a hook script.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScriptText(pub Box<str>);

/// Whose a script is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ownership {
    /// `asymptote init` wrote it, and may rewrite or remove it.
    Ours,
    /// Anyone else's: never touched.
    Foreign,
}
