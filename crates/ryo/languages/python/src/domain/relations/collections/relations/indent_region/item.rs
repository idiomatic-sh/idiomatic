//! Emit all regions once from the module root; neither directives nor lint policy are read.
use crate::domain::file_view::types::PythonWalk;
use crate::domain::layout_facts::service::indent_regions;
use crate::domain::node::service::{node_cell, text_between, word};
use crate::domain::{indentation::{constants::INCOMPLETE}};
use ryo::languages;
/// Python lexical INDENT/DEDENT regions.
pub(crate) struct IndentRegionItem;

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
