//! TypeScript `flow_block` rows: the basic blocks of the file's control flow graphs, numbered from zero, as the read kept them from `oxc_semantic`'s graph.

use crate::domain::model::types::ReadIndexes;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// Reads the `flow-block` rows of one TypeScript file from the graphs its read kept.
pub(crate) struct FlowBlockItem;

ryo::reads! {
    FlowBlockItem => languages::FlowBlock, model(ReadIndexes) |tree, _model, at, symbols| {
        kinds: [Kind::Node(Node::Program)] as kind_bits,
        each (languages::flow_of(tree.elements).blocks.iter()) as (number, block) {
            row {
                block: value(languages::Integer(i64::from(number.0))),
                reachable: value(block.reachable),
                live: value(block.live),
                implicit_return: value(block.implicit_return),
                file: file,
            }
        }
    }
}
