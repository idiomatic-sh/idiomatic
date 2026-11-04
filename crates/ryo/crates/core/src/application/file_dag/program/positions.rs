//! One admitted native tuple instruction. Dependencies remain in ProgramExecution.
use super::{admission::KernelPermit, facade::PreparedProgram};
use crate::application::file_dag::{
    executor::PrimitiveExecutor,
    relations::{PublishedInput, RelationOwner, RelationResult},
};
use crate::domain::{
    lowering::types::{LoweredColumn, LoweredShape, RowPosition, RowWidth},
    native_positions::types::{PositionInput, PositionWork},
    operating::{errors::OperatingError, types::OperatorContext},
    operator::contract::{types::WorkCounters, views::BorrowedRows},
    positions::types::{Positions, Slots},
    table_cells::types::TreeForm,
    table_model::types::Rows,
};
use ryo_relations::domain::relations::types::CellKind;
use std::{
    rc::Rc,
    sync::{Arc, OnceLock},
};
pub(in crate::application::file_dag) struct ProducedPositions {
    parts: PositionParts,
}
pub(in crate::application::file_dag) struct PositionParts {
    pub source: Arc<RelationResult>,
    pub positions: Arc<Positions>,
    pub shape: Arc<LoweredShape>,
    pub kinds: Arc<[CellKind]>,
}
impl ProducedPositions {
    pub(in crate::application::file_dag) fn into_parts(self) -> PositionParts {
        self.parts
    }
}
pub(super) fn evaluate<'facts>(
    prepared: &PreparedProgram,
    permit: &KernelPermit<'_>,
    region: crate::domain::executable_program::types::RegionId,
    shape: &Arc<LoweredShape>,
    owner: &Rc<RelationOwner>,
    read: &dyn Fn(
        ryo_relations::domain::relations::types::Relation,
    ) -> Result<Rows<'facts>, OperatingError>,
    kernels: &dyn PrimitiveExecutor,
    context: &mut OperatorContext<'_>,
    work: &mut PositionWork,
    counts: &WorkCounters,
) -> Result<Option<ProducedPositions>, OperatingError> {
    let semantic_permit = permit.position_semantics(shape)?;
    let code = &prepared.program().regions()[region.0].instructions[0];
    let mut sources: Vec<PublishedInput> = Vec::new();
    let mut rows = Vec::new();
    for at in 0..code.inputs.len() {
        let admitted = permit.bound_input(at)?;
        let publication = admitted.publication(owner)?;
        let actual = admitted.rows(owner, read)?;
        owner.admit(publication.as_ref(), BorrowedRows::from(&actual))?;
        if !matches!(actual, Rows::Tree(_) | Rows::Positioned(_)) {
            return Ok(None);
        }
        sources.push(publication.clone());
        rows.push(actual);
    }
    let Some(tree) = rows.first().and_then(|rows| match rows {
        Rows::Tree(tree) => Some(*tree),
        Rows::Positioned(view) => Some(view.tree),
        _ => None,
    }) else {
        return Ok(None);
    };
    let mut direct = Vec::new();
    for rows in &rows {
        match rows {
            Rows::Tree(each) => {
                if !std::ptr::eq(each.elements, tree.elements) || each.file != tree.file {
                    return Err(OperatingError::NotRead);
                }
                let fields = match each.form {
                    TreeForm::Elements => &crate::domain::lowering::constants::ELEMENT,
                    TreeForm::Edges => &crate::domain::lowering::constants::EDGE,
                };
                let shape = LoweredShape {
                    width: RowWidth(1),
                    columns: fields
                        .iter()
                        .map(|field| LoweredColumn {
                            slot: RowPosition(0),
                            field: *field,
                        })
                        .collect(),
                };
                let start = usize::from(each.form == TreeForm::Edges);
                let end = ryo_language::domain::elements::service::real_count(each.elements).0;
                let slots = (start..end)
                    .map(u32::try_from)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| OperatingError::ShapeMismatch)?;
                direct.push(Some((
                    shape,
                    Positions {
                        width: RowWidth(1),
                        slots: Slots(slots),
                    },
                )));
            }
            Rows::Positioned(view) => {
                if !std::ptr::eq(view.tree.elements, tree.elements) || view.tree.file != tree.file {
                    return Err(OperatingError::NotRead);
                }
                direct.push(None);
            }
            _ => return Err(OperatingError::WrongOperation),
        }
    }
    let inputs: Vec<_> = rows
        .iter()
        .zip(&direct)
        .map(|(rows, direct)| match (rows, direct) {
            (Rows::Positioned(view), None) => Ok(PositionInput {
                shape: view.shape,
                rows: view.positions,
            }),
            (_, Some((shape, positions))) => Ok(PositionInput {
                shape,
                rows: positions,
            }),
            _ => Err(OperatingError::WrongOperation),
        })
        .collect::<Result<_, _>>()?;
    let failure = OnceLock::new();
    let semantics = crate::application::file_dag::execution::PrimitiveSemanticsBridge {
        port: kernels,
        permit: semantic_permit,
        binding: crate::domain::executable_program::physical::PhysicalBinding {
            native_join: false,
            source_len: context.source.0.len(),
        },
        failure: &failure,
    };
    let mut derived =
        crate::application::file_dag::derived::owner::raw_scope::RawScope::for_program(
            Rc::clone(owner),
            prepared.native_order(region)?,
        );
    let result = crate::domain::native_positions::service::evaluate(
        &semantics,
        &mut derived,
        permit.operation(),
        &tree,
        &inputs,
        shape,
        context,
        work,
        counts,
    );
    if let Some(error) = failure.into_inner() {
        return Err(error);
    }
    let positions = result?;
    let kinds = code.contract.collected_kinds.clone()?;
    Ok(Some(ProducedPositions {
        parts: PositionParts {
            source: owner.position_source(sources.first().ok_or(OperatingError::NotRun)?)?,
            positions: Arc::new(positions),
            shape: Arc::clone(shape),
            kinds: Arc::from(kinds),
        },
    }))
}
