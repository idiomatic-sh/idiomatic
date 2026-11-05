//! The names of the `annotates` relation: the text its name and its columns carry when they leave the engine.

use super::item::AnnotatesColumn;
use crate::domain::relations::constants::OCCURRENCE;

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "annotates"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "The type a binding, a parameter or a function result is annotated with, as written."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: AnnotatesColumn) -> &'static str {
    match column {
        AnnotatesColumn::Occurrence => OCCURRENCE,
        AnnotatesColumn::Owner => "owner",
        AnnotatesColumn::Annotation => "annotation",
        AnnotatesColumn::File => "file",
        AnnotatesColumn::Span => "span",
    }
}
