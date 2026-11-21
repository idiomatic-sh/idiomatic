//! The `view` service of the model: free functions named by verb, with no IO and no ports.

use super::types::{Built, ModelPart, PartState, SemanticModel};
use crate::domain::model::declarations::service as declarations;
use crate::domain::model::definitions::service as definitions;
use crate::domain::model::flow::service as flow;
use crate::domain::model::lexical::service as lexical;
use crate::domain::model::lexical::types::TypedTree;
use crate::domain::model::names::service as names;
use crate::domain::model::schedule::service as schedule;
use crate::domain::binding_schedule::constants::LAZY_ANNOTATIONS_FROM;
use crate::domain::model::reaching::types::FlowSource;
use crate::domain::model::resolution::service as resolution;
use crate::domain::model::schedule::types::ScheduleSource;
use crate::domain::model::types::BindingAnnotationTiming;

/// Builds the parts of `model` that `part` needs and `built` lacks, from the parser's own nodes: `tree` holds the node of each
/// place with its parent and the end of its subtree, and each node's index is its place. It reads no node table, so the walk can
/// ask it before its table is finished. Annotations are scheduled lazily when the model's language target defers them or the
/// module postpones them. A list of the flow part by key is made by the first fill that asks for one key of it.
pub(crate) fn built(model: &mut SemanticModel, built: &mut Built, part: ModelPart, tree: TypedTree<'_, '_>) {
    if part != ModelPart::Nothing && built.lexical == PartState::Pending {
        names::clear(&mut model.names);
        names::clear_words(model.words.get_mut());
        lexical::build(&mut model.lexical, &mut model.names, tree);
        declarations::build(&mut model.lexical, tree);
        definitions::build(&mut model.definitions, &model.lexical, &model.names, tree);
        built.lexical = PartState::Ready;
    }
    if part == ModelPart::Resolution && built.resolution == PartState::Pending {
        resolution::build(&mut model.resolution, &model.definitions, &model.lexical, tree);
        built.resolution = PartState::Ready;
    }
    let flows = matches!(
        part,
        ModelPart::Flow | ModelPart::ReadsByName | ModelPart::ShadowsByBinding | ModelPart::ReadsByNode
    );
    if flows && built.flow == PartState::Pending {
        let deferred = model.target >= LAZY_ANNOTATIONS_FROM || tree.annotations == BindingAnnotationTiming::Deferred;
        let timing = if deferred { BindingAnnotationTiming::Deferred } else { BindingAnnotationTiming::Immediate };
        schedule::build(
            &mut model.schedule,
            ScheduleSource { lexical: &model.lexical, definitions: &model.definitions, tree, timing },
        );
        flow::build(
            &mut model.flow,
            FlowSource {
                names: &model.names,
                lexical: &model.lexical,
                definitions: &model.definitions,
                schedule: &model.schedule,
                tree,
            },
        );
        built.flow = PartState::Ready;
    }
    if part == ModelPart::ReadsByName && built.reads_by_name == PartState::Pending {
        flow::list_reads(&mut model.flow);
        built.reads_by_name = PartState::Ready;
    }
    if part == ModelPart::ShadowsByBinding && built.shadows_by_binding == PartState::Pending {
        flow::list_shadows(&mut model.flow);
        built.shadows_by_binding = PartState::Ready;
    }
    if part == ModelPart::ReadsByNode && built.reads_by_node == PartState::Pending {
        flow::list_reads_by_node(&mut model.flow);
        built.reads_by_node = PartState::Ready;
    }
}
