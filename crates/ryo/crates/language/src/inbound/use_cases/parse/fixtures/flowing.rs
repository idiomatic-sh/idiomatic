//! The language whose read keeps a control flow graph: for a file whose name ends in [`KEPT_GRAPH`], one root element, two blocks,
//! the edge between them, a step that is the root in the second block, and the root as a member of the first; none for any other
//! file. The `parse` feature reads the graph back out of the table.

use crate::domain::answers::service::absent;
use crate::domain::answers::types::{Reader, Support};
use crate::domain::builders::service::{builder_position, enter_node, leave_node};
use crate::domain::elements::types::{Completion, TableBuilder, Tree};
use crate::domain::flow::types::{
    FlowBlockFact, FlowEdgeFact, FlowEdgeKind, FlowMemberFact, FlowStepFact, FlowStepKind,
    StepOrdinal,
};
use crate::domain::flow_table::service::{
    add_flow_block, add_flow_edge, add_flow_member, add_flow_step,
};
use crate::inbound::use_cases::parse::errors::ParseError;
use crate::inbound::use_cases::parse::interface::ParseUseCase;
use crate::inbound::use_cases::parse::types::{
    KindNames, ParseConfigurationIdentity, ParseRequest, Recognition, SkippedDirectory,
};
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::types::{Count, Flag, KindBits, Offset};
use ryo_relations::domain::terms::types::UnknownName;

/// The end of the name of a file whose read keeps a graph.
pub const KEPT_GRAPH: &str = "two-block";

/// A language that reads every file and keeps a graph of two blocks, the first jumping to the second, for a file whose name ends
/// in [`KEPT_GRAPH`]; it answers every relation as not in it.
#[derive(Clone, Copy, Debug)]
pub struct Flowing;

/// Answers every relation as not in the language.
macro_rules! absent_all {
    (
        syntax { $( $variant:ident $module:ident $row:ident $column:ident ; )* }
        manifest { $( $manifest:tt )* }
        engine { $( $engine:tt )* }
    ) => {
        $(
            fn $module(
                &self,
            ) -> Support<Reader<ryo_relations::domain::relations::collections::relations::$module::item::$variant, Self::FileModel>> {
                absent(AbsenceCause::NoSuchConstruct)
            }
        )*
    };
}

impl ParseUseCase for Flowing {
    type FileModel = ();

    fn begin_semantics(
        &self,
        _request: &ParseRequest<'_>,
        _tree: &crate::domain::elements::types::Elements,
        _slot: &mut crate::domain::semantics::types::SemanticSlot,
    ) {
    }

    fn semantics(&self) -> Option<&dyn crate::domain::answers::types::Semantics> {
        None
    }

    fn completion_requirement(&self) -> crate::domain::elements::types::CompletionRequirement {
        crate::domain::elements::types::CompletionRequirement::Independent
    }

    fn extend_file_model(
        &self,
        _tree: &Tree<'_>,
        _from: ryo_relations::domain::relations::types::Position,
        _model: &mut (),
    ) {
    }

    fn recognizes_explicit(&self, _request: &ParseRequest<'_>) -> Recognition {
        Recognition::Declines
    }

    fn capabilities(&self) -> &'static [ryo_relations::domain::catalog::types::OptionalCapability] {
        &[]
    }

    fn configuration_identity(&self) -> ParseConfigurationIdentity<'_> {
        ParseConfigurationIdentity("")
    }

    fn file_model(&self, _tree: &Tree<'_>, _model: &mut ()) {}

    fn recognizes(&self, _request: &ParseRequest<'_>) -> Recognition {
        Recognition::Reads
    }

    fn skips(&self) -> &'static [SkippedDirectory] {
        &[]
    }

    fn unknown_reasons(&self) -> &'static [UnknownName] {
        &[]
    }

    fn kind_count(&self) -> Count {
        Count(1)
    }

    fn kind_names(&self) -> KindNames {
        KindNames(Vec::new())
    }

    fn read_tree(
        &self,
        request: &ParseRequest<'_>,
        builder: &mut TableBuilder<'_>,
    ) -> Result<(), ParseError> {
        if request.path.ends_with(KEPT_GRAPH) {
            let root = builder_position(builder);
            enter_node(builder, KindBits(1), Offset(0));
            leave_node(
                builder,
                Offset(u32::try_from(request.bytes.0.len()).unwrap_or(u32::MAX)),
            );
            let entry = FlowBlockFact {
                reachable: Flag::Raised,
                live: Flag::Raised,
                implicit_return: Flag::Lowered,
            };
            let source = add_flow_block(builder, entry);
            let target = add_flow_block(
                builder,
                FlowBlockFact {
                    implicit_return: Flag::Raised,
                    ..entry
                },
            );
            add_flow_edge(
                builder,
                FlowEdgeFact {
                    source,
                    target,
                    kind: FlowEdgeKind::Jump,
                },
            );
            add_flow_step(
                builder,
                FlowStepFact {
                    block: target,
                    ordinal: StepOrdinal(0),
                    kind: FlowStepKind::Statement,
                    node: root,
                },
            );
            add_flow_member(
                builder,
                FlowMemberFact {
                    node: root,
                    block: source,
                },
            );
        }
        Ok(())
    }

    fn complete(&self, _request: &ParseRequest<'_>, _completion: &mut Completion<'_>) {}

    ryo_relations::each_relation!(@declared ryo_relations::baseline_relations, absent_all);
}
