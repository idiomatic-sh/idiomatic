//! Physical regions selected and invoked by the one application scheduler.
//! Native positions and fused columns are algorithms, never alternate graph owners.
use super::{
    execution::{CommonOperatorEngine, PrimitiveSemanticsBridge},
    service::GraphInputs,
};
use crate::domain::{
    lowering::types::Counting,
    operating::{
        errors::OperatingError,
        types::{Emit, OperatorContext},
    },
};
use std::sync::Arc;

/// Prepare immutable index requirements once with the request, before it is
/// shared by file/project scopes. No population, cache, or row lives here.
pub(crate) fn prepare(
    graph: &mut crate::domain::file_program::types::RequestGraph,
    semantic: Arc<dyn crate::outbound::derived::interface::DerivedPort>,
) {
    for node in &mut graph.nodes {
        let Some(chain) = node
            .region
            .as_mut()
            .and_then(|region| region.column.as_mut())
        else {
            continue;
        };
        let schemas = [crate::domain::operation_schema::types::InputSchema {
            kinds: ryo_relations::domain::relations::service::kinds(chain.right),
            tree: None,
        }];
        let request = crate::domain::derived_views::types::ViewRequest::Equality {
            columns: chain.right_on.clone(),
        };
        if let Ok(index) = super::derived::prepare::view(Arc::clone(&semantic), &request, &schemas)
            && index.metadata.is_ok()
        {
            chain.index = Some(Arc::new(index));
        }
    }
}

/// All eligibility decisions precede output. Refusal/errors propagate; there is
/// no partial-output fallback into a second algorithm.
pub(super) fn stream_region(
    engine: &CommonOperatorEngine<'_>,
    feed: &mut GraphInputs<'_, '_>,
    context: &mut OperatorContext<'_>,
    room: &mut crate::domain::operating::types::Room,
    emit: &mut dyn Emit,
) -> Result<bool, OperatingError> {
    let mut output = super::execution::output::State::take(&mut room.scratch, &mut room.memo);
    let mut executed = false;
    let result = output.run(emit, |emit| {
        executed = stream_region_inner(engine, feed, context, emit)?;
        Ok(())
    });
    output.restore(&mut room.scratch, &mut room.memo);
    result.map(|()| executed)
}
fn stream_region_inner(
    engine: &CommonOperatorEngine<'_>,
    feed: &mut GraphInputs<'_, '_>,
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
) -> Result<bool, OperatingError> {
    let Some(region) = feed.region() else {
        return Ok(false);
    };
    let index = region.step.0 as usize;
    let (native, classes, picked) = feed.native_binding();
    if native == crate::domain::operator::types::TreeSteps::Lowered
        && (region
            .layout
            .lowering
            .shapes
            .get(index)
            .is_some_and(Option::is_some)
            || region.layout.lowering.counting.get(index) == Some(&Counting::Counts))
    {
        let crate::domain::table_model::types::Rows::Tree(tree) =
            (feed.reader())(ryo_relations::domain::relations::types::Relation::SyntaxTree)?
        else {
            return Ok(false);
        };
        let mut positions = crate::domain::positions::types::LoweredRoom::default();
        positions.ready.resize(
            region.plan.steps.len(),
            crate::domain::positions::types::Readiness::NotMade,
        );
        positions
            .made
            .resize_with(region.plan.steps.len(), Default::default);
        positions.picked = picked;
        let mut derived = super::derived::owner::raw_scope::RawScope::new(feed.owner());
        let mut run = crate::domain::fragment::types::FragmentRun {
            derived: &mut derived,
            plan: &region.plan,
            lowering: &region.layout.lowering,
            tree: crate::domain::table_cells::types::TreeRows {
                form: crate::domain::table_cells::types::TreeForm::Elements,
                ..tree
            },
            room: &mut positions,
            classes: &classes.classes,
        };
        let semantics = PrimitiveSemanticsBridge {
            port: engine.operators,
        };
        if crate::domain::fragment::service::counts(&run, region.step) == Counting::Counts {
            crate::domain::fragment::service::emit_counts(
                &semantics,
                &mut run,
                region.step,
                context,
                emit,
            )?;
        } else {
            crate::domain::fragment::service::make(&semantics, &mut run, region.step, context)?;
            crate::domain::fragment::service::emit(&mut run, region.step, context, emit)?;
        }
        return Ok(true);
    }
    if feed.column_enabled()
        && let Some(chain) = &region.column
    {
        let publication = feed.original_publication(region.rule, chain.left_step, context)?;
        let owner = feed.owner();
        let mut table = ryo_values::domain::rows::types::Table::default();
        let mut counts = crate::domain::column_kernel::types::KernelCounts::default();
        if column::execute_publication(
            &PrimitiveSemanticsBridge {
                port: engine.operators,
            },
            chain,
            &|relation| (feed.reader())(relation),
            &owner,
            &publication,
            context,
            &mut table,
            region
                .layout
                .kinds
                .get(index)
                .ok_or(OperatingError::NotRun)?,
            &mut counts,
        )? {
            feed.column_counts(counts);
            crate::domain::replay::service::scan(
                &crate::domain::table_model::types::Rows::Computed(&table),
                &mut Vec::new(),
                context,
                emit,
            )?;
            return Ok(true);
        }
    }
    Ok(false)
}
pub(crate) mod column;
pub(crate) mod join;
pub(crate) mod ready;
