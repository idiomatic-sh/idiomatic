//! TypeScript `flow_step` rows: the steps of the file's basic blocks that are elements, in block order, as the read kept them from `oxc_semantic`'s graph.

use crate::domain::file_view::service::node_cell;
use crate::domain::file_view::types::TypeScriptWalk;
use crate::domain::syntax::reading::WordSink;
use crate::domain::model::types::ReadIndexes;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// Reads the `flow-step` rows of one TypeScript file from the graphs its read kept.
pub(crate) struct FlowStepItem;

ryo::reads! {
    FlowStepItem => languages::FlowStep, model(ReadIndexes) |tree, _model, at, symbols| {
        kinds: [Kind::Node(Node::Program)] as kind_bits,
        each (languages::flow_of(tree.elements).steps.iter()) as step {
            row {
                block: value(languages::Integer(i64::from(step.block.0))),
                ordinal: value(languages::Integer(i64::from(step.ordinal.0))),
                step_kind: value(languages::intern_derived(symbols, tree.source, languages::flow_step_word(step.kind))),
                node: node(step.node),
                file: file,
            }
        }
    }
}

// The engine fills `flow-step` from the walked file: every step that is a node, once.
ryo::fills! {
    FlowStepItem => languages::FlowStep, walk(TypeScriptWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::FlowStep as languages::CatalogRelation>::Row;
        let mut words = WordSink::new(words);
        let Some(flow) = &file.flow else {
            return;
        };
        for step in &flow.steps {
            rows.write(Row {
                block: languages::Integer(i64::from(step.block.0)),
                ordinal: languages::Integer(i64::from(step.ordinal.0)),
                step_kind: crate::domain::syntax::reading::Interner::derived(&mut words, file.source, languages::flow_step_word(step.kind)),
                node: node_cell(file, step.node),
                file: file.file,
            });
        }
    }
}
