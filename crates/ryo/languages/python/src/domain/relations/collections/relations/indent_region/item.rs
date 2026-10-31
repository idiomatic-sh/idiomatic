//! Emit all regions once from the module root; neither directives nor lint policy are read.
use crate::domain::{
    indentation::{
        constants::INCOMPLETE,
        service::{prefix, regions},
    },
    model::types::Declarations,
    nodes::types::Node,
    syntax::{service::kind_bits, types::Kind},
};
use ryo::languages;
/// Python lexical INDENT/DEDENT regions.
pub(crate) struct IndentRegionItem;
ryo::reads! {
 IndentRegionItem => languages::IndentRegion, model(Declarations) |tree, model, at, symbols| {
 kinds:[Kind::Node(Node::Module)] as kind_bits,
 when regions(tree,&model.indentation) {
  Some(index)=>{
   each (index.rows.iter()) as region {
    row {
     id:node(region.id),parent:node(region.parent),indentation:value(prefix(tree,symbols,region)),
     start_byte:value(languages::Integer(i64::from(region.span.start.0))),
     end_byte:value(languages::Integer(i64::from(region.span.end.0))),
     file:file,span:value(region.span),
    }
   }
  }
  None=>{partial INCOMPLETE => languages::Missing::Any;}
 }
 }
}
