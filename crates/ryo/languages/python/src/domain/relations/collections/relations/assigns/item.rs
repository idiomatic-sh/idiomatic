//! The `assigns` relation of the language: the string whose value is known that an assignment binds to a single name, linked to the
//! name and the string, owned by the function it is in or the file's start.

use crate::domain::model::types::Declarations;
use crate::domain::names::service::assignments;
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service::function_of;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use crate::domain::file_view::types::PythonWalk;
use crate::domain::literal_facts::service::string_value;
use crate::domain::literal_facts::types::StringValue;
use crate::domain::model::view::service::built;
use crate::domain::model::view::types::ModelPart;
use crate::domain::node::service::{expressions_of, node_cell, nodes_of_kind, scope_id, span_at, table_id};
use crate::domain::node::types::Numbered;
use ruff_python_ast::AnyNodeRef;
use ryo::languages;

/// Reads the `assigns` rows of one Python file from its syntax tree.
pub(crate) struct AssignsItem;

ryo::reads! {
    AssignsItem => languages::Assigns, model(Declarations) |tree, model, at, _symbols| {
        kinds: [Kind::Node(Node::Assign), Kind::Node(Node::AnnAssign), Kind::Node(Node::NamedExpr)] as kind_bits,
        each (assignments(tree, at)) as assigned {
            row {
                occurrence: node(at),
                binding: node(assigned.binding),
                value: node(assigned.value),
                owner: node(function_of(tree, at)),
                file: file,
                span: span(assigned.value),
            }
        }
    }
}

// The engine fills `assigns` from the walked file: the string a single name is bound to, once.
ryo::fills! {
    AssignsItem => languages::Assigns, walk(PythonWalk) |file, _words, rows| {
        /// One row of the relation.
        type Row = <languages::Assigns as languages::CatalogRelation>::Row;
        built(file.model, &mut file.built, ModelPart::Lexical, file.syntax.tree);
        let scope_of = &file.model.lexical.scope_of;
        let joined = &mut file.buffers.text;
        for kind in [Node::Assign, Node::AnnAssign, Node::NamedExpr] {
            for Numbered { id, .. } in nodes_of_kind(&file.syntax, Kind::Node(kind)) {
                let expressions: Vec<_> = expressions_of(&file.syntax, id).collect();
                let count = expressions.len();
                let (targets, value) = match kind {
                    Node::Assign => (count.saturating_sub(1), count.saturating_sub(1)),
                    Node::AnnAssign if count == 3 => (1, 2),
                    Node::NamedExpr => (1, 1),
                    _ => (0, 0),
                };
                let Some(&value) = expressions.get(value) else {
                    continue;
                };
                let known = crate::domain::node::service::place_of_id(file.syntax.tree, value)
                    .and_then(|place| file.syntax.tree.nodes.get(usize::from(place)).copied())
                    .map(AnyNodeRef::from)
                    .and_then(|node| string_value(node, joined))
                    .is_some_and(|text| matches!(text, StringValue::Known(_)));
                if !known {
                    continue;
                }
                let owner = function_owner(&file.syntax, scope_of, id);
                for &binding in expressions.iter().take(targets) {
                    if crate::domain::syntax::service::kind_of(languages::node_table::kind_of(file.syntax.table, binding)) != Kind::Node(Node::Name) {
                        continue;
                    }
                    rows.write(Row {
                        occurrence: node_cell(&file.syntax, id),
                        binding: node_cell(&file.syntax, binding),
                        value: node_cell(&file.syntax, value),
                        owner: node_cell(&file.syntax, owner),
                        file: file.syntax.file,
                        span: span_at(&file.syntax, value),
                    });
                }
            }
        }
    }
}

/// The nearest function or lambda the node `id` is in, or the module, as a table id.
fn function_owner(file: &crate::domain::node::types::SyntaxFile<'_>, scope_of: &[languages::Position], id: languages::Position) -> languages::Position {
    let tree = file.tree;
    let mut scope = scope_id(file, scope_of, id);
    loop {
        let place = crate::domain::node::service::place_of_id(tree, scope);
        let node = place.and_then(|place| tree.nodes.get(usize::from(place)).copied()).map(AnyNodeRef::from);
        if matches!(node, Some(AnyNodeRef::StmtFunctionDef(_) | AnyNodeRef::ExprLambda(_))) {
            return scope;
        }
        let up = place
            .and_then(|place| scope_of.get(usize::from(place)))
            .map_or(languages::Position(0), |&up| table_id(tree.segments, up));
        if up == scope {
            return scope;
        }
        scope = up;
    }
}
