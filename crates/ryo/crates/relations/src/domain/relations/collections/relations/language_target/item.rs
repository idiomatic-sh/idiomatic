//! The source-language parser target associated with a file.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Integer};

/// `language-target`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct LanguageTarget;

impl super::super::interface::sealed::Sealed for LanguageTarget {}

#[macro_export]
#[doc(hidden)]
macro_rules! language_target {
    ($callback:path) => {
        $callback! {
            item LanguageTarget,
            row { /// One parser target configured for a source file.
                LanguageTargetRow },
            column { /// The columns of `language-target`.
                LanguageTargetColumn },
            expected { /// One `language-target` fixture row.
                LanguageTargetExpectedRow },
            class Engine,
            columns {
                /// The source file whose parser configuration is described.
                file File: FileId => File;
                /// The major component of the configured parser target.
                major Major: Integer => Number;
                /// The minor component of the configured parser target.
                minor Minor: Integer => Number;
            }
        }
    };
}

crate::language_target!(crate::relation_row);
