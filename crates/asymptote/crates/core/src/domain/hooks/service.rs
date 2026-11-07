//! The `hooks` service: the script of a hook and whose a script is, with no IO and no ports.

use super::constants::{COMMAND, MARKER, NEWLINE, SHEBANG, SPACE};
use super::types::{Ownership, ScriptText};
use crate::domain::manifest::types::HookName;

/// The script of the hook `name`: the shebang, the comment that marks it asymptote's, and the check it runs.
#[must_use]
pub fn script(name: &HookName) -> ScriptText {
    let command = [COMMAND, SPACE, &name.0].concat();
    let mut lines = [SHEBANG, MARKER, &command].join(NEWLINE);
    lines.push_str(NEWLINE);
    ScriptText(lines.into())
}

/// Whose `text` is: asymptote's when its second line is the marker, anyone else's otherwise.
#[must_use]
pub fn ownership(text: &ScriptText) -> Ownership {
    if text.0.lines().nth(1) == Some(MARKER) {
        Ownership::Ours
    } else {
        Ownership::Foreign
    }
}
