//! Write the configured Python target independently of syntax recovery.

use crate::domain::file_view::types::PythonWalk;

use ryo::languages;

/// Reads the parser target configured for the current source.
pub(crate) struct LanguageTargetItem;

// The engine fills `language-target` from the walked file: every row of the file, once.
ryo::fills! {
    LanguageTargetItem => languages::LanguageTarget, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::LanguageTarget as languages::CatalogRelation>::Row;
        rows.write(Row {
            file: file.syntax.file,
            major: languages::Integer(file.target.major.0),
            minor: languages::Integer(file.target.minor.0),
        });
    }
}
