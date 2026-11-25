//! TypeScript `flow_member` rows: the block each element of the file's functions is evaluated in, as the read kept them from `oxc_semantic`'s graph.

use crate::domain::file_view::service::node_cell;
use crate::domain::file_view::types::TypeScriptWalk;
use crate::domain::model::types::ReadIndexes;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// Reads the `flow-member` rows of one TypeScript file from the graphs its read kept.
pub(crate) struct FlowMemberItem;

ryo::reads! {
    FlowMemberItem => languages::FlowMember, model(ReadIndexes) |tree, _model, at, symbols| {
        kinds: [Kind::Node(Node::Program)] as kind_bits,
        each (languages::flow_of(tree.elements).members.iter()) as member {
            row {
                node: node(member.node),
                block: value(languages::Integer(i64::from(member.block.0))),
                file: file,
            }
        }
    }
}

// The engine fills `flow-member` from the walked file: every node's block, once.
ryo::fills! {
    FlowMemberItem => languages::FlowMember, walk(TypeScriptWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::FlowMember as languages::CatalogRelation>::Row;
        let _ = words;
        let Some(flow) = &file.flow else {
            return;
        };
        for member in &flow.members {
            rows.write(Row {
                node: node_cell(file, member.node),
                block: languages::Integer(i64::from(member.block.0)),
                file: file.file,
            });
        }
    }
}
