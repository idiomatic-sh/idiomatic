//! TypeScript `flow_block` rows: the basic blocks of the file's control flow graphs, numbered from zero, as the read kept them from `oxc_semantic`'s graph.

use crate::domain::file_view::types::TypeScriptWalk;
use ryo::languages;

/// Reads the `flow-block` rows of one TypeScript file from the graphs its read kept.
pub(crate) struct FlowBlockItem;


// The engine fills `flow-block` from the walked file: every block of the file's control flow graphs, once.
ryo::fills! {
    FlowBlockItem => languages::FlowBlock, walk(TypeScriptWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::FlowBlock as languages::CatalogRelation>::Row;
        let _ = words;
        let Some(flow) = &file.flow else {
            return;
        };
        for (number, block) in flow.blocks.iter().enumerate() {
            rows.write(Row {
                block: languages::Integer(i64::try_from(number).unwrap_or(i64::MAX)),
                reachable: block.reachable,
                live: block.live,
                implicit_return: block.implicit_return,
                file: file.file,
            });
        }
    }
}
