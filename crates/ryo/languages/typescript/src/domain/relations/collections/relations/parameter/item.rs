//! TypeScript parameter rows.
use crate::domain::file_view::service::{node_cell, of_kinds, span_at};
use crate::domain::file_view::types::TypeScriptWalk;
use crate::domain::syntax::reading::WordSink;
use crate::domain::names::service as names;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// Reads the `parameter` rows of one TypeScript file from its syntax tree.
pub(crate) struct ParameterItem;

// The engine fills `parameter` from the walked file: every row of the file, once.
ryo::fills! {
    ParameterItem => languages::Parameter, walk(TypeScriptWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Parameter as languages::CatalogRelation>::Row;
        let mut words = WordSink::new(words);
        let file = &*file;
        for at in of_kinds(file, &[
            Kind::Node(Node::FunctionDeclaration),
            Kind::Node(Node::FunctionExpression),
            Kind::Node(Node::ArrowFunctionExpression),
            Kind::Node(Node::TSDeclareFunction),
            Kind::Node(Node::TSEmptyBodyFunctionExpression),
        ]) {
            for (ordinal, parameter) in names::parameters(file, at).enumerate() {
                rows.write(Row {
                    id: node_cell(file, parameter),
                    kind: crate::domain::syntax::service::parameter_kind(file, parameter),
                    refinement: crate::domain::syntax::service::kind_symbol(file, &mut words, parameter),
                    name: names::parameter_name(file, &mut words, parameter),
                    ordinal: languages::Integer(i64::try_from(ordinal).unwrap_or(i64::MAX)),
                    owner: node_cell(file, at),
                    file: file.file,
                    span: span_at(file, parameter),
                });
            }
        }
    }
}
