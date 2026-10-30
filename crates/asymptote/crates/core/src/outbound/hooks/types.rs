//! The types of the `hooks` port: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::hooks::types::ScriptText;

/// The path of a hook script, as git answers for it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HookFile(pub Box<str>);

/// What git printed for a question, trimmed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GitAnswer(pub Box<str>);

/// What a repository holds as a hook.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Installed {
    /// Nothing.
    Absent,
    /// This script, in this file.
    Script {
        /// Where it is.
        file: HookFile,
        /// What it says.
        text: ScriptText,
    },
}
