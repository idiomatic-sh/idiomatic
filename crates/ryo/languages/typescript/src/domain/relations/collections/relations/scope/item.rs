//! TypeScript scope rows from its standard syntax tree.
use crate::domain::file_view::service::{node_cell, of_kinds, span_at};
use crate::domain::file_view::types::TypeScriptWalk;
use crate::domain::syntax::reading::WordSink;
use crate::domain::model::types::ReadIndexes;
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service as scopes;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `scope` rows of one TypeScript file from its syntax tree.
pub(crate) struct ScopeItem;
ryo::reads! {
 ScopeItem => languages::Scope, model(ReadIndexes) |tree, _model, at, symbols| {
 kinds: [Kind::Node(Node::Program),Kind::Node(Node::FunctionDeclaration),Kind::Node(Node::FunctionExpression),Kind::Node(Node::ArrowFunctionExpression),Kind::Node(Node::ClassDeclaration),Kind::Node(Node::ClassExpression),Kind::Node(Node::TSDeclareFunction),Kind::Node(Node::TSEmptyBodyFunctionExpression),Kind::Node(Node::TSModuleDeclaration),Kind::Node(Node::TSModuleBlock),Kind::Node(Node::BlockStatement),Kind::Node(Node::CatchClause),Kind::Node(Node::ForStatement),Kind::Node(Node::ForInStatement),Kind::Node(Node::ForOfStatement),Kind::Node(Node::StaticBlock)] as kind_bits,
 require scopes::opens(crate::domain::syntax::service::kind(tree,at)) == crate::domain::scopes::types::Scope::Opens;

 row { parent: node(scopes::enclosing(tree,at)), child: node(at), file: file, span: span(at) }

 }
}

// The engine fills `scope` from the walked file: every row of the file, once.
ryo::fills! {
    ScopeItem => languages::Scope, walk(TypeScriptWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Scope as languages::CatalogRelation>::Row;
        let _ = words;
        let file = &*file;
        for at in of_kinds(file, &[Kind::Node(Node::Program),Kind::Node(Node::FunctionDeclaration),Kind::Node(Node::FunctionExpression),Kind::Node(Node::ArrowFunctionExpression),Kind::Node(Node::ClassDeclaration),Kind::Node(Node::ClassExpression),Kind::Node(Node::TSDeclareFunction),Kind::Node(Node::TSEmptyBodyFunctionExpression),Kind::Node(Node::TSModuleDeclaration),Kind::Node(Node::TSModuleBlock),Kind::Node(Node::BlockStatement),Kind::Node(Node::CatchClause),Kind::Node(Node::ForStatement),Kind::Node(Node::ForInStatement),Kind::Node(Node::ForOfStatement),Kind::Node(Node::StaticBlock)]) {
            if scopes::opens(crate::domain::syntax::service::kind(file, at)) != crate::domain::scopes::types::Scope::Opens {
                continue;
            }
            rows.write(Row {
                parent: node_cell(file, scopes::enclosing(file, at)),
                child: node_cell(file, at),
                file: file.file,
                span: span_at(file, at),
            });
        }
    }
}
