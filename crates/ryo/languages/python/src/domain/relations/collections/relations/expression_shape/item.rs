//! Read lazily interned structural classes; never emit unknown classes that could compare equal.
use crate::domain::file_view::types::{PythonFile, PythonWalk};
use crate::domain::model::view::types::PartState;
use crate::domain::node::service::{keyed_id, node_cell, span_at, table_id};
use crate::domain::node::types::SyntaxFile;
use crate::domain::shape_facts::service::shapes;
use crate::domain::shape_facts::types::ShapeBuild;
use crate::domain::syntax::service::{category_of, kind_of};
use crate::domain::{
    expression_shapes::{constants::INCOMPLETE, service::prepare},
    model::types::Declarations,
    nodes::types::{Category, Node},
    syntax::{
        service::{category, kind_bits},
        types::Kind,
    },
};
use ryo::languages::{self, node_table};
/// One class per original Python expression occurrence.
pub(crate) struct ExpressionShapeItem;
ryo::reads! {
 ExpressionShapeItem=>languages::ExpressionShape,model(Declarations)|tree,model,at,symbols|{
 kinds:[Kind::Node(Node::Module)] as kind_bits, expanded,
 when prepare(tree,model,symbols){
  Some(index)=>{
   each(crate::domain::semantic_syntax::service::positions(tree))as expression{
    require category(tree,expression)==Category::Expression;
    given shape=languages::shape_class_at(&index,expression)?;
    row{id:node(expression),shape:node(shape),file:file,span:span(expression),}
   }
  }
  None=>{partial INCOMPLETE=>languages::Missing::Any;}
 }
 }
}

/// Whether the parser read the whole file, so that its shapes can be told; when it did not, the relation is said read only in
/// part.
fn told_whole(
    file: &PythonFile<'_>,
    rows: &mut dyn languages::Rows<languages::ExpressionShapeRow>,
) -> languages::SyntaxCompleteness {
    let whole = file.syntax.completeness.syntax;
    if whole == languages::SyntaxCompleteness::Incomplete {
        rows.partial(languages::ReportText(INCOMPLETE), languages::Missing::Any);
    }
    whole
}

/// Finds the shapes of the file's nodes, when no fill of the file has yet: a shape's class is named by a member of it, so the
/// shapes are found whole, once, by the first fill that asks, whole or by key.
fn shaped(file: &mut PythonFile<'_>, words: &mut languages::FileWords) {
    if file.analysed.shapes == PartState::Pending {
        let index = &mut file.buffers.shapes;
        shapes(
            &file.syntax,
            &mut ShapeBuild {
                index,
                words,
                text: &mut file.buffers.text,
            },
        );
        file.analysed.shapes = PartState::Ready;
    }
}

/// The row of the expression `id`, whose shape's class is named by `class`; none for a node that is no expression.
fn shape_row(
    file: &SyntaxFile<'_>,
    id: languages::Position,
    class: languages::Position,
) -> Option<languages::ExpressionShapeRow> {
    let expression =
        category_of(kind_of(node_table::kind_of(file.table, id))) == Category::Expression;
    expression.then(|| languages::ExpressionShapeRow {
        id: node_cell(file, id),
        shape: node_cell(file, class),
        file: file.file,
        span: span_at(file, id),
    })
}

// The engine fills `expression-shape` from the walked file: every row of the file, or the row of one expression, read from the
// class the file's shapes give it.
ryo::fills! {
    ExpressionShapeItem => languages::ExpressionShape, walk(PythonWalk) |file, words, rows| {
        if told_whole(file, rows) == languages::SyntaxCompleteness::Incomplete {
            return;
        }
        shaped(file, words);
        // The nodes of the file's own tree, then of each embedded segment, by their place.
        for place in 0..file.syntax.tree.nodes.len() {
            let place = languages::Position(u32::try_from(place).unwrap_or(u32::MAX));
            let id = table_id(file.syntax.tree.segments, place);
            let class = file.buffers.shapes.classes.get(usize::from(id)).copied().flatten();
            if let Some(row) = class.and_then(|class| shape_row(&file.syntax, id, class)) {
                rows.write(row);
            }
        }
    }
}
