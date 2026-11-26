//! Whether a file is read as TypeScript or as JavaScript, from the syntax flag its `Program` holds: Oxc's source type, which
//! oxlint's JavaScript-only rules test (`is_typescript`).

use crate::domain::language::constants::TYPESCRIPT_SYNTAX;
use crate::domain::language::service::syntax_flag;
use crate::domain::file_view::service::of_kinds;
use crate::domain::file_view::types::TypeScriptWalk;
use crate::domain::syntax::reading::{Interner as _, Syntax as _, WordSink};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::constants::TYPESCRIPT_SYNTAX_FLAG;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// Reads the `compiler-feature` row of one TypeScript or JavaScript file: `typescript.syntax`, raised for TypeScript.
pub(crate) struct CompilerFeatureItem;


// The engine fills `compiler-feature` from the walked file: every row of the file, once.
ryo::fills! {
    CompilerFeatureItem => languages::CompilerFeature, walk(TypeScriptWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::CompilerFeature as languages::CatalogRelation>::Row;
        let mut words = WordSink::new(words);
        let file = &*file;
        for at in of_kinds(file, &[Kind::Node(Node::Program)]) {
            // The adapter writes the flag on every `Program` it reads.
            let Some(flag) = file.property_of(at, TYPESCRIPT_SYNTAX_FLAG) else {
                continue;
            };
            rows.write(Row {
                file: file.file,
                feature: words.derived(file.source, languages::NameText(TYPESCRIPT_SYNTAX)),
                enabled: syntax_flag(flag),
            });
        }
    }
}
