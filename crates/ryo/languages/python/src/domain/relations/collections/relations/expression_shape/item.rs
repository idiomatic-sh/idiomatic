//! Read lazily interned structural classes; never emit unknown classes that could compare equal.
use crate::domain::file_view::types::PythonWalk;
use crate::domain::node::service::{node_cell, span_at, typed_nodes};
use crate::domain::node::types::Numbered;
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

// The engine fills `expression-shape` from the walked file: every row of the file, once.
ryo::fills! {
    ExpressionShapeItem => languages::ExpressionShape, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::ExpressionShape as languages::CatalogRelation>::Row;
        if file.syntax.completeness.syntax == languages::SyntaxCompleteness::Incomplete {
            rows.partial(languages::ReportText(INCOMPLETE), languages::Missing::Any);
            return;
        }
        let index = &mut file.buffers.shapes;
        shapes(&file.syntax, &mut ShapeBuild { index, words, text: &mut file.buffers.text });
        for (Numbered { id, .. }, class) in typed_nodes(&file.syntax).zip(index.classes.iter()) {
            if category_of(kind_of(node_table::kind_of(file.syntax.table, id))) != Category::Expression {
                continue;
            }
            if let Some(class) = class {
                rows.write(Row {
                    id: node_cell(&file.syntax, id),
                    shape: node_cell(&file.syntax, *class),
                    file: file.syntax.file,
                    span: span_at(&file.syntax, id),
                });
            }
        }
    }
}
