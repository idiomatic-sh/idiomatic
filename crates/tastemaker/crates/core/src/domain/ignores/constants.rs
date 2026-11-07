//! The text an ignore is read by.

/// What a malformed ignore that names no rule is told.
pub const NO_RULE: &str = "`ignore:` names no rule: write `ignore: <package/rule> - <reason>`";
/// What an unused ignore below code is told when its rule found something about the whole file.
pub const LATE_NOTE: &str =
    "; a finding about the whole file is ignored at the top of the file, before any code";
