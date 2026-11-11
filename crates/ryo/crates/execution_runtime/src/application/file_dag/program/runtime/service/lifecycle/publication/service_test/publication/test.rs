//! Compiled program lifecycle proof with a real owner.
use super::super::super::super::inputs::fixtures::{CompiledCase, compiled};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_store::application::file_dag::relations::interface::RelationStore as _;
use std::rc::Rc;

#[test]
fn publication_constructor_rejects_same_owner_proof_substitution_even_for_canonical_aliases() {
    use crate::application::file_dag::program::admission::{ProducerRoot, ReadProof};

    use ryo_execution_domain::domain::table_model::types::RepresentationInput;
    use ryo_values::domain::rows::types::{RowCount, Table};
    let CompiledCase { execution, roots } = compiled();
    let owner = Rc::clone(&execution.owner);
    let kinds = execution.program.regions()[roots[0].0]
        .instructions()
        .last()
        .unwrap()
        .contract()
        .output_kinds
        .clone()
        .unwrap();
    let rows = Table {
        cells: kinds
            .iter()
            .copied()
            .map(super::super::super::super::inputs::fixtures::value)
            .collect(),
        kinds,
        count: RowCount(1),
    };
    let a = owner.publish(ryo_execution_store::application::file_dag::relations::publication::types::PublicationBody::Computed(rows.clone())).unwrap();
    let b = owner.publish(ryo_execution_store::application::file_dag::relations::publication::types::PublicationBody::Computed(rows)).unwrap();
    assert!(a.same_record(&b).is_err());
    for aliases in [false, true] {
        if aliases {
            execution
                .file_state()
                .bind(&owner, ProducerRoot::Region(roots[0]), &a)
                .unwrap();
            execution
                .file_state()
                .bind(&owner, ProducerRoot::Region(roots[0]), &b)
                .unwrap();
        }
        let unread = |_| Err(OperatingError::NotRead);
        let a_rows = owner.rows(&a, &unread).unwrap();
        let b_rows = owner.rows(&b, &unread).unwrap();
        let a_proof = owner
            .admit(&a, RepresentationInput::from(&a_rows))
            .and_then(|admitted| admitted)
            .unwrap();
        let b_proof = owner
            .admit(&b, RepresentationInput::from(&b_rows))
            .and_then(|admitted| admitted)
            .unwrap();
        if aliases {
            assert_eq!(
                a_proof.identity(&owner).unwrap(),
                b_proof.identity(&owner).unwrap()
            );
        } else {
            assert_ne!(
                a_proof.identity(&owner).unwrap(),
                b_proof.identity(&owner).unwrap()
            );
        }
        for (publication, proof) in [(&a, &a_proof), (&b, &b_proof)] {
            let admitted =
                crate::application::file_dag::program::admission::AdmittedInput::from_publication(
                    Rc::clone(&owner),
                    publication.clone(),
                    proof,
                    ProducerRoot::Region(roots[0]),
                    ReadProof::complete(execution.partition().clone()),
                )
                .unwrap();
            assert!(
                admitted
                    .publication(&owner)
                    .unwrap()
                    .same_record(publication)
                    .is_ok()
            );
        }
        for (publication, proof) in [(&b, &a_proof), (&a, &b_proof)] {
            assert!(matches!(
                crate::application::file_dag::program::admission::AdmittedInput::from_publication(
                    Rc::clone(&owner),
                    publication.clone(),
                    proof,
                    ProducerRoot::Region(roots[0]),
                    ReadProof::complete(execution.partition().clone())
                ),
                Err(OperatingError::NotRead)
            ));
        }
    }
    Rc::try_unwrap(execution)
        .unwrap_or_else(|_| panic!("single production owner"))
        .finish()
        .unwrap();
}
