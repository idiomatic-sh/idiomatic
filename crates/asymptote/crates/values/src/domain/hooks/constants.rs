//! The words of a hook script.

/// The first line of a hook script.
pub const SHEBANG: &str = "#!/bin/sh";
/// The comment line that says a script is asymptote's own: the line `init` finds its scripts by.
pub const MARKER: &str = "# written by asymptote init; edit [hooks] in asymptote.toml instead";
/// The command a script runs, before the hook's name.
pub const COMMAND: &str = "exec asymptote check --hook";
/// What ends a line.
pub const NEWLINE: &str = crate::domain::judgement::constants::NEWLINE;
/// What separates a command from its argument.
pub const SPACE: &str = " ";
