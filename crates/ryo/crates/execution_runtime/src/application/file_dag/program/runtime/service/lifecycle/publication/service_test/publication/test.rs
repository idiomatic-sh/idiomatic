//! Compiled program lifecycle proof with a real owner.
use super::super::super::super::inputs::fixtures::{CompiledCase, compiled};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_store::application::file_dag::relations::interface::RelationsPort as _;
use std::rc::Rc;

/// What two publications of one row by one owner observed when each proof is offered for its
/// own publication and for the other's.
#[derive(Debug, PartialEq)]
struct Observed {
    /// Whether the two publications are refused as one record.
    distinct_records: bool,
    /// Whether the two proofs name one identity.
    one_identity: bool,
    /// Of the two own-proof constructions, those whose admitted input is its publication's record.
    own_proofs_admitted: usize,
    /// The refusals of the two swapped-proof constructions.
    swapped: [Option<OperatingError>; 2],
}

/// Whether both publications are bound as aliases of one producer root.
type Input = bool;
type Value = Observed;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "publication_constructor_rejects_same_owner_proof_substitution_even_for_canonical_aliases",
        ),
        word: Word::Succeeds,
        input: || true,
        expect: Expect::Value(Observed {
            distinct_records: true,
            one_identity: true,
            own_proofs_admitted: 2,
            swapped: [Some(OperatingError::NotRead), Some(OperatingError::NotRead)],
        }),
    },
    Case {
        name: CaseName("publication_constructor_rejects_same_owner_proof_substitution"),
        word: Word::Succeeds,
        input: || false,
        expect: Expect::Value(Observed {
            distinct_records: true,
            one_identity: false,
            own_proofs_admitted: 2,
            swapped: [Some(OperatingError::NotRead), Some(OperatingError::NotRead)],
        }),
    },
];

/// Publishes one row twice, binds the publications as aliases or not, and constructs admitted
/// inputs from each proof.
fn subject(aliases: Input) -> Result<Value, Failure> {
    use crate::application::file_dag::program::admission::{
        service::ReadProof, types::ProducerRoot,
    };

    use ryo_execution_domain::domain::table_model::types::RepresentationInput;
    use ryo_values::domain::rows::types::{RowCount, Table};
    let CompiledCase { execution, roots } = compiled();
    let owner = Rc::clone(&execution.owner);
    let kinds = execution.program.regions()[roots[0].0]
        .instructions()
        .last()
        .ok_or(OperatingError::NotRun)?
        .contract()
        .output_kinds
        .clone()?;
    let rows = Table {
        cells: kinds
            .iter()
            .copied()
            .map(super::super::super::super::inputs::fixtures::value)
            .collect(),
        kinds,
        count: RowCount(1),
    };
    let a = owner.publish(ryo_execution_store::application::file_dag::relations::publication::types::PublicationBody::Computed(rows.clone()))?;
    let b = owner.publish(ryo_execution_store::application::file_dag::relations::publication::types::PublicationBody::Computed(rows))?;
    let distinct_records = a.same_record(&b).err().is_some();
    if aliases {
        execution
            .file_state()
            .bind(&owner, ProducerRoot::Region(roots[0]), &a)?;
        execution
            .file_state()
            .bind(&owner, ProducerRoot::Region(roots[0]), &b)?;
    }
    let unread = |_| Err(OperatingError::NotRead);
    let a_rows = owner.rows(&a, &unread)?;
    let b_rows = owner.rows(&b, &unread)?;
    let a_proof = owner
        .admit(&a, RepresentationInput::from(&a_rows))
        .and_then(|admitted| admitted)?;
    let b_proof = owner
        .admit(&b, RepresentationInput::from(&b_rows))
        .and_then(|admitted| admitted)?;
    let one_identity = a_proof.identity(&owner)? == b_proof.identity(&owner)?;
    let construct = |publication: &ryo_execution_store::application::file_dag::relations::handle::service::Publication, proof| {
        crate::application::file_dag::program::admission::service::AdmittedInput::from_publication(
            Rc::clone(&owner),
            publication.clone(),
            proof,
            ProducerRoot::Region(roots[0]),
            ReadProof::complete(execution.partition().clone()),
        )
    };
    let mut own_proofs_admitted = 0;
    for (publication, proof) in [(&a, &a_proof), (&b, &b_proof)] {
        let admitted = construct(publication, proof)?;
        own_proofs_admitted += usize::from(
            admitted
                .publication(&owner)?
                .same_record(publication)
                .err()
                .is_none(),
        );
    }
    let swapped = [construct(&b, &a_proof).err(), construct(&a, &b_proof).err()];
    drop((a_rows, b_rows));
    Rc::try_unwrap(execution)
        .map_err(|_| OperatingError::NotRun)?
        .finish()?;
    Ok(Observed {
        distinct_records,
        one_identity,
        own_proofs_admitted,
        swapped,
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
