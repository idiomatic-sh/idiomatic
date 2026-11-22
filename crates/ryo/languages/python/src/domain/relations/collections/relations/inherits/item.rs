//! The `inherits` relation of the language: each positional base of a class, by the dotted name it writes (`Generic[T]` names
//! `Generic`); a starred base has the unknown value of a star base, any other the unknown value of a dynamic base. A keyword such as
//! `metaclass=` is no base.

use crate::domain::extras::types::Extra;
use crate::domain::file_view::types::PythonWalk;
use crate::domain::literal_facts::types::TextBuffer;
use crate::domain::model::lexical::service::parent_of;
use crate::domain::model::types::Declarations;
use crate::domain::name_facts::service::base as named_base;
use crate::domain::names::service::base_target;
use crate::domain::names::service::unknown;
use crate::domain::node::service::{
    element_id, keyed_node, node_at, node_cell, nodes_of_kind, span_at, word,
};
use crate::domain::node::types::{Numbered, SyntaxFile};
use crate::domain::nodes::types::Category;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{child, kind_bits, nodes_of};
use crate::domain::syntax::types::Kind;
use ruff_python_ast::{AnyNodeRef, Expr};
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

/// The row of the base `expression` of the class at `class`; none for an expression the walk gave no id.
fn base_row(
    file: &SyntaxFile<'_>,
    words: &mut languages::FileWords,
    name: &mut TextBuffer,
    class: languages::Position,
    expression: &Expr,
) -> Option<languages::InheritsRow> {
    let at = element_id(file, expression)?;
    let target = match named_base(expression, name) {
        Some(reason) => unknown(reason),
        None => word(words, languages::NameText(&name.0)),
    };
    Some(languages::InheritsRow {
        occurrence: node_cell(file, at),
        declaration: node_cell(file, class),
        target,
        file: file.file,
        span: span_at(file, at),
    })
}

/// The class whose arguments hold the node `at` directly, with those arguments' bases: none for a node that stands anywhere
/// else.
fn bases_above<'f>(file: &SyntaxFile<'f>, at: languages::Position) -> Option<Numbered<'f>> {
    let arguments = parent_of(file.tree, at);
    let class = parent_of(file.tree, arguments);
    let node = node_at(file, class)?;
    let held = matches!(node_at(file, arguments)?, AnyNodeRef::Arguments(_));
    (held && matches!(node, AnyNodeRef::StmtClassDef(_))).then_some(Numbered { id: class, node })
}

// The engine fills `inherits` from the walked file: every row of the file, or the row of one base, read from the class above it.
ryo::fills! {
    InheritsItem => languages::Inherits, walk(PythonWalk) |file, words, rows| {
        let name = &mut file.buffers.text;
        for Numbered { id, node } in nodes_of_kind(&file.syntax, Kind::Node(Node::ClassDef)) {
            let AnyNodeRef::StmtClassDef(class) = node else {
                continue;
            };
            let Some(arguments) = &class.arguments else {
                continue;
            };
            for expression in &arguments.args {
                if let Some(row) = base_row(&file.syntax, words, name, id, expression) {
                    rows.write(row);
                }
            }
        }
    }
    key(languages::Column::Inherits(languages::InheritsColumn::Occurrence)) |file, words, key, rows| {
        let Some(at) = keyed_node(&file.syntax, key) else {
            return;
        };
        let Some(Numbered { id, node: AnyNodeRef::StmtClassDef(class) }) = bases_above(&file.syntax, at) else {
            return;
        };
        let bases = class.arguments.iter().flat_map(|arguments| &arguments.args);
        for expression in bases.filter(|expression| element_id(&file.syntax, *expression) == Some(at)) {
            if let Some(row) = base_row(&file.syntax, words, &mut file.buffers.text, id, expression) {
                rows.write(row);
            }
        }
    }
}
