//! Admitted native tuple publication owned by the relation population lifecycle.
use super::*;
/// The owner accepts only its admitted source, not caller-supplied records or epochs.
pub(in crate::application::file_dag) fn publish(
    owner: &RelationOwner,
    source: &PublishedInput,
    admitted: &AdmittedInput<'_>,
    positions: crate::domain::positions::types::Positions,
    shape: Arc<crate::domain::lowering::types::LoweredShape>,
    kinds: Arc<[CellKind]>,
) -> Result<PublishedInput, OperatingError> {
    owner.generation(source)?;
    let (population, _) = admitted.identity(owner)?;
    if population != owner.result_population(source)
        || !owner.matches_current(source, admitted.rows())
    {
        return Err(OperatingError::NotRead);
    }
    let tree = match admitted.rows() {
        BorrowedRows::Tree(tree) => tree,
        BorrowedRows::Positioned(view) => view.tree,
        _ => return Err(OperatingError::WrongOperation),
    };
    let view = BorrowedRows::Positioned(crate::domain::table_model::types::PositionedRows {
        tree,
        positions: &positions,
        shape: &shape,
        kinds: &kinds,
    });
    let contract = view.contract()?;
    if registered(RepresentationKind::Positioned).inspect(&contract)? != PhysicalShape::Complete {
        return Err(OperatingError::ShapeMismatch);
    }
    owner.publication(Arc::new(RelationResult {
        population: owner.allocate_population()?,
        producer: OnceLock::new(),
        producer_scope: OnceLock::new(),
        generation: Arc::clone(&owner.generation.borrow()),

        body: RelationBody::Positioned {
            source: owner.position_source(source)?,
            positions: Arc::new(positions),
            shape,
            kinds,
        },
        inspected: OnceLock::from(Ok(PhysicalShape::Complete)),
        normalized: OnceLock::new(),
    }))
}
