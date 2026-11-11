//! Shared real request and semantic-descriptor preparation data for owner controls.
use ryo_execution_domain::domain::{
    derived_views::types::{ViewRequest, contract::value::DerivedContract},
    operation_schema::types::InputShape,
    table_model::{service::kinds_of, types::Rows},
};
use ryo_relations::domain::relations::types::Relation;

use std::sync::Arc;
/// The derived needs of a rule that selects every row of `relation` per file: it reads that relation, as the runtime's
/// preparation finds of a plan of one select over it, and walks no graph.
pub(crate) fn reads(relation: Relation) -> super::super::prepared::RuleDerivedNeeds {
    super::super::prepared::RuleDerivedNeeds {
        relations: Box::new([relation]),
        graph: super::cache::types::GraphAdmission::Disabled,
    }
}
/// The derived needs of a rule that walks the axis of `relation`: it reads that relation and walks a graph.
pub(crate) fn walks(relation: Relation) -> super::super::prepared::RuleDerivedNeeds {
    super::super::prepared::RuleDerivedNeeds {
        relations: Box::new([relation]),
        graph: super::cache::types::GraphAdmission::Enabled,
    }
}
#[expect(
    clippy::unwrap_used,
    reason = "a fixture: preparing a view of a real request over its own rows cannot fail, and a failure fails the proof"
)]
pub(crate) fn prepare_view(
    semantic: impl Into<crate::application::file_dag::derived::owner::service::DerivedAccess>,
    request: ViewRequest,
    rows: &Rows<'_>,
) -> Arc<DerivedContract> {
    Arc::new(
        super::super::prepare::view(
            semantic,
            &request,
            &[InputShape {
                kinds: kinds_of(rows),
                tree: None,
            }],
        )
        .unwrap(),
    )
}

pub mod graph;
pub mod graph_snapshot;
pub mod observations;
pub mod resources;
