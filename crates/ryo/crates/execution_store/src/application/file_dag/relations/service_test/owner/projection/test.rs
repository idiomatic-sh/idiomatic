//! Forged metadata never gains authority it was not issued.
use super::super::super::super::RelationOwner;
use super::super::fixtures::columns;
use super::fixtures::Forged;
use crate::application::file_dag::relations::types::CheckCount;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_execution_domain::domain::{operating::errors::OperatingError, table_model::types::Rows};
use ryo_relations::domain::relations::types::{CellKind, ColumnsView};

/// What a forged publication of the right kind asks of the owner.
#[derive(Clone, Copy, Debug)]
enum Ask {
    Inspect,
    Admit,
    Resolve,
}

/// The owner's refusal, and the checks it spent on the forged publication.
#[derive(Debug, PartialEq)]
struct Refused {
    error: OperatingError,
    checks: CheckCount,
}

type Input = Ask;
type Value = ();
type Failure = Refused;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_foreign_publication_with_correct_kind_has_no_admission_authority"),
        word: Word::Refuses,
        input: || Ask::Admit,
        expect: Expect::Error(Refused {
            error: OperatingError::NotRead,
            checks: CheckCount(0),
        }),
    },
    Case {
        name: CaseName("a_foreign_publication_with_correct_kind_is_not_inspected"),
        word: Word::Refuses,
        input: || Ask::Inspect,
        expect: Expect::Error(Refused {
            error: OperatingError::NotRead,
            checks: CheckCount(0),
        }),
    },
    Case {
        name: CaseName("a_foreign_publication_with_correct_kind_acquires_no_provider"),
        word: Word::Refuses,
        input: || Ask::Resolve,
        expect: Expect::Error(Refused {
            error: OperatingError::NotRead,
            checks: CheckCount(0),
        }),
    },
];

/// Asks a fresh owner, over one number column, on behalf of the forged publication.
fn subject(input: Input) -> Result<Value, Failure> {
    let owner = RelationOwner::new(
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        std::sync::Arc::new(crate::application::file_dag::fixtures::derived()),
    );
    let physical = columns(&[CellKind::Number]);
    let rows = Rows::Stored(ColumnsView {
        kinds: &[CellKind::Number],
        columns: &physical,
    });
    let answer = match input {
        Ask::Inspect => owner
            .inspect(&Forged, RepresentationInput::from(&rows))
            .map(drop),
        Ask::Admit => owner
            .admit(&Forged, RepresentationInput::from(&rows))
            .map(drop),
        Ask::Resolve => owner
            .resolve(&Forged, &|_| {
                panic!("forged publication must not acquire a provider")
            })
            .map(drop),
    };
    answer.map_err(|error| Refused {
        error,
        checks: owner.checks.get(),
    })
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
