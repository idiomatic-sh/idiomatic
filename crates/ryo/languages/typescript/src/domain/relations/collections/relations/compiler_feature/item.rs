//! Whether a file is read as TypeScript or as JavaScript, from the syntax flag its `Program` holds: Oxc's source type, which
//! oxlint's JavaScript-only rules test (`is_typescript`).

use crate::domain::language::constants::TYPESCRIPT_SYNTAX;
use crate::domain::language::service::syntax_flag;
use crate::domain::model::types::ReadIndexes;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::constants::TYPESCRIPT_SYNTAX_FLAG;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// Reads the `compiler-feature` row of one TypeScript or JavaScript file: `typescript.syntax`, raised for TypeScript.
pub(crate) struct CompilerFeatureItem;

ryo::reads! {
    CompilerFeatureItem => languages::CompilerFeature, model(ReadIndexes) |tree, _model, at, symbols| {
        kinds: [Kind::Node(Node::Program)] as kind_bits,
        // The adapter writes the flag on every `Program` it reads.
        given flag = languages::property_at(tree.elements, at, TYPESCRIPT_SYNTAX_FLAG)?;
        given feature = languages::intern_derived(symbols, tree.source, languages::NameText(TYPESCRIPT_SYNTAX));
        row { file: file, feature: value(feature), enabled: value(syntax_flag(flag)), }
    }
}
