//! The names of the `mention` relation: the text its name and its columns carry when they leave the engine.

use super::item::MentionColumn;
use crate::domain::relations::constants::{OCCURRENCE, TARGET};

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "mention"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "An identifier written inside a macro call's token tree, where no syntax says whether it is a use."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: MentionColumn) -> &'static str {
    match column {
        MentionColumn::Occurrence => OCCURRENCE,
        MentionColumn::Owner => "owner",
        MentionColumn::Target => TARGET,
        MentionColumn::File => "file",
        MentionColumn::Span => "span",
    }
}
