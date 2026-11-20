//! Write the configured Python target independently of syntax recovery.

use crate::domain::file_view::types::PythonWalk;
use crate::domain::{
    model::types::Declarations,
    nodes::types::Node,
    syntax::{service::kind_bits, types::Kind},
};
use ryo::languages;

/// Reads the parser target configured for the current source.
pub(crate) struct LanguageTargetItem;

ryo::reads! {
    LanguageTargetItem => languages::LanguageTarget, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as kind_bits,
        each (std::iter::once(model.language_target)) as target {
            given major = languages::Integer(target.major.0);
            given minor = languages::Integer(target.minor.0);
            row { file: file, major: value(major), minor: value(minor), }
        }
    }
}

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
