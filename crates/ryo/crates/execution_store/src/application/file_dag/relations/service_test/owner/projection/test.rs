//! Forged metadata never gains authority it was not issued.
use super::super::super::super::RelationOwner;
use super::super::fixtures::columns;
use super::fixtures::Forged;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_execution_domain::domain::{operating::errors::OperatingError, table_model::types::Rows};
use ryo_relations::domain::relations::types::{CellKind, ColumnsView};
fn a_foreign_publication_with_correct_kind_has_no_admission_authority() {
    let owner = RelationOwner::new(
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        std::sync::Arc::new(crate::application::file_dag::fixtures::derived()),
    );
    let physical = columns(&[CellKind::Number]);
    let rows = Rows::Stored(ColumnsView {
        kinds: &[CellKind::Number],
        columns: &physical,
    });
    assert!(matches!(
        owner.inspect(&Forged, RepresentationInput::from(&rows)),
        Err(OperatingError::NotRead)
    ));
    assert!(matches!(
        owner.admit(&Forged, RepresentationInput::from(&rows)),
        Err(OperatingError::NotRead)
    ));
    assert!(matches!(
        owner.resolve(&Forged, &|_| panic!(
            "forged publication must not acquire a provider"
        )),
        Err(OperatingError::NotRead)
    ));
    assert_eq!(
        owner.checks.get(),
        crate::application::file_dag::relations::types::CheckCount(0)
    );
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    AForeignPublicationWithCorrectKindHasNoAdmissionAuthority,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_foreign_publication_with_correct_kind_has_no_admission_authority"),
    word: Word::Succeeds,
    input: || Scenario::AForeignPublicationWithCorrectKindHasNoAdmissionAuthority,
    expect: Expect::Value(()),
}];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::AForeignPublicationWithCorrectKindHasNoAdmissionAuthority => {
            a_foreign_publication_with_correct_kind_has_no_admission_authority();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
