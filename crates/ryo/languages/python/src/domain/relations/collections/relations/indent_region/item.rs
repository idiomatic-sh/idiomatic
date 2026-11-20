//! Emit all regions once from the module root; neither directives nor lint policy are read.
use crate::domain::file_view::types::PythonWalk;
use crate::domain::layout_facts::service::indent_regions;
use crate::domain::node::service::{node_cell, text_between, word};
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

// The engine fills `indent-region` from the walked file: every row of the file, once.
ryo::fills! {
    IndentRegionItem => languages::IndentRegion, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::IndentRegion as languages::CatalogRelation>::Row;
        let regions = &mut file.buffers.regions;
        if indent_regions(&file.syntax, regions) == languages::SyntaxCompleteness::Incomplete {
            rows.partial(languages::ReportText(INCOMPLETE), languages::Missing::Any);
            return;
        }
        for region in regions.iter() {
            let indentation = text_between(&file.syntax, region.span.start, region.indentation_end);
            rows.write(Row {
                id: node_cell(&file.syntax, region.id),
                parent: node_cell(&file.syntax, region.parent),
                indentation: word(words, indentation),
                start_byte: languages::Integer(i64::from(region.span.start.0)),
                end_byte: languages::Integer(i64::from(region.span.end.0)),
                file: file.syntax.file,
                span: region.span,
            });
        }
    }
}
