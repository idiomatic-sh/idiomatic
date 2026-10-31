//! Read lazily interned structural classes; never emit unknown classes that could compare equal.
use crate::domain::{
    expression_shapes::{
        constants::INCOMPLETE,
        service::{prepare, shape_of},
    },
    model::types::Declarations,
    nodes::types::{Category, Node},
    syntax::{
        service::{category, kind_bits},
        types::Kind,
    },
};
use ryo::languages;
/// One class per original Python expression occurrence.
pub(crate) struct ExpressionShapeItem;
ryo::reads! {
 ExpressionShapeItem=>languages::ExpressionShape,model(Declarations)|tree,model,at,symbols|{
 kinds:[Kind::Node(Node::Module)] as kind_bits,
 when prepare(tree,model,symbols){
  Some(index)=>{
   each(crate::domain::semantic_syntax::service::positions(tree))as expression{
    require category(tree,expression)==Category::Expression;
    given shape=shape_of(&index,expression)?;
    row{id:node(expression),shape:node(shape),file:file,span:span(expression),}
   }
  }
  None=>{partial INCOMPLETE=>languages::Missing::Any;}
 }
 }
}
