//! The `inherits` relation of the language: each positional base of a class, by the dotted name it writes (`Generic[T]` names
//! `Generic`); a starred base has the unknown value of a star base, any other the unknown value of a dynamic base. A keyword such as
//! `metaclass=` is no base.

use crate::domain::extras::types::Extra;
use crate::domain::file_view::types::PythonWalk;
use crate::domain::model::types::Declarations;
use crate::domain::name_facts::service::base as named_base;
use crate::domain::names::service::base_target;
use crate::domain::names::service::unknown;
use crate::domain::node::service::{id_of, node_cell, nodes_of_kind, span_at, word};
use crate::domain::node::types::Numbered;
use crate::domain::nodes::types::Category;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{child, kind_bits, nodes_of};
use crate::domain::syntax::types::Kind;
use ruff_python_ast::AnyNodeRef;
use ryo::languages;

/// Reads the `inherits` rows of one Python file from its syntax tree.
pub(crate) struct InheritsItem;

ryo::reads! {
    InheritsItem => languages::Inherits, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::ClassDef)] as kind_bits,
        given arguments = child(tree, at, Kind::Extra(Extra::CallArguments))?;
        each (nodes_of(tree, arguments, Category::Expression)) as base {
            row {
                occurrence: node(base),
                declaration: node(at),
                target: value(base_target(tree, symbols, base)),
                file: file,
                span: span(base),
            }
        }
    }
}

// The engine fills `inherits` from the walked file: every row of the file, once.
ryo::fills! {
    InheritsItem => languages::Inherits, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Inherits as languages::CatalogRelation>::Row;
        let name = &mut file.buffers.text;
        for Numbered { id, node } in nodes_of_kind(&file.syntax, Kind::Node(Node::ClassDef)) {
            let AnyNodeRef::StmtClassDef(class) = node else {
                continue;
            };
            let Some(arguments) = &class.arguments else {
                continue;
            };
            for expression in &arguments.args {
                let Some(at) = id_of(expression) else {
                    continue;
                };
                let target = match named_base(expression, name) {
                    Some(reason) => unknown(reason),
                    None => word(words, languages::NameText(&name.0)),
                };
                rows.write(Row {
                    occurrence: node_cell(&file.syntax, at),
                    declaration: node_cell(&file.syntax, id),
                    target,
                    file: file.syntax.file,
                    span: span_at(&file.syntax, at),
                });
            }
        }
    }
}
