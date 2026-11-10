//! The names of the `workspace_member` relation: the text its name and its columns carry when they leave the engine.

use super::item::WorkspaceMemberColumn;
use crate::domain::relations::constants::PACKAGE;

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "workspace-member"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "A package that belongs to a workspace."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: WorkspaceMemberColumn) -> &'static str {
    match column {
        WorkspaceMemberColumn::Workspace => "workspace",
        WorkspaceMemberColumn::Package => PACKAGE,
    }
}
