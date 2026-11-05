//! Selected populations are issued only against their admitted source population.
use super::*;
pub(in crate::application::file_dag) fn publish(
    owner: &RelationOwner,
    source: &PublishedInput,
    admitted: &AdmittedInput<'_>,
    selection: Arc<StoredSelection>,
    resources: [Option<super::super::derived::view::AcquiredView>; 2],
) -> Result<PublishedInput, OperatingError> {
    owner.generation(source)?;
    if !matches!(&source.record().body, RelationBody::Source(identity) if identity.relation == selection.relation)
    {
        return Err(OperatingError::NotRead);
    }
    let (population, _) = admitted.identity(owner)?;
    if population != owner.result_population(source)
        || !owner.matches_current(source, admitted.rows())
    {
        return Err(OperatingError::NotRead);
    }
    let BorrowedRows::Stored(columns) = admitted.rows() else {
        return Err(OperatingError::WrongOperation);
    };
    let rows = Rows::Selected(SelectedRows {
        source: columns,
        selection: Arc::clone(&selection),
    });
    let view = BorrowedRows::from(&rows);
    let contract = view.contract()?;
    if registered(RepresentationKind::Selected).inspect(&contract)? != PhysicalShape::Complete {
        return Err(OperatingError::ShapeMismatch);
    }
    owner.publication_with_resources(
        Arc::new(RelationResult {
            population: owner.allocate_population()?,
            producer: OnceLock::new(),
            generation: Arc::clone(&owner.generation.borrow()),

            body: RelationBody::Selected {
                source: Arc::clone(source.record()),
                selection,
            },
            inspected: OnceLock::from(Ok(PhysicalShape::Complete)),
            normalized: OnceLock::new(),
        }),
        resources,
    )
}
