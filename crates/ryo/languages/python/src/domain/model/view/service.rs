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
use crate::domain::model::lexical::constants::MODULE;
use crate::domain::model::reaching::types::FlowSource;
use crate::domain::model::resolution::service as resolution;
use crate::domain::model::schedule::types::ScheduleSource;
use crate::domain::model::types::BindingAnnotationTiming;
use crate::domain::syntax::constants::{MODULE_FUTURE_ANNOTATIONS, MODULE_FUTURE_ANNOTATIONS_ENABLED};
use ruff_python_ast::AnyNodeRef;
use ryo::languages::NodeTable;
use ryo::languages::node_table::property_of;

/// Builds the parts of `model` that `part` needs and `built` lacks, from the parser's own nodes: `nodes` holds the node of each
/// node id of `table`, and each node's index is its id. Annotations are scheduled lazily when the model's language target
/// defers them or the module's future-annotations property is enabled.
pub(crate) fn built(
    model: &mut SemanticModel,
    built: &mut Built,
    part: ModelPart,
    table: &NodeTable,
    nodes: &[AnyNodeRef<'_>],
) {
    let tree = TypedTree { table, nodes };
    if part != ModelPart::Nothing && built.lexical == PartState::Pending {
        names::clear(&mut model.names);
        names::clear_words(model.words.get_mut());
        lexical::build(&mut model.lexical, &mut model.names, tree);
        declarations::build(&mut model.lexical, table);
        definitions::build(&mut model.definitions, &model.lexical, &model.names, tree);
        built.lexical = PartState::Ready;
    }
    if part == ModelPart::Resolution && built.resolution == PartState::Pending {
        resolution::build(&mut model.resolution, &model.definitions, &model.lexical, table);
        built.resolution = PartState::Ready;
    }
    if part == ModelPart::Flow && built.flow == PartState::Pending {
        let deferred = model.target >= LAZY_ANNOTATIONS_FROM
            || property_of(table, MODULE, MODULE_FUTURE_ANNOTATIONS)
                .is_some_and(|value| value.0 == MODULE_FUTURE_ANNOTATIONS_ENABLED);
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
}
