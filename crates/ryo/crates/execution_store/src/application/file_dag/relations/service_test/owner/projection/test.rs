//! Forged metadata never gains authority it was not issued.
use super::super::super::super::service::RelationOwner;
use super::super::fixtures::columns;
use super::fixtures::Forged;
use crate::application::file_dag::relations::types::CheckCount;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::table_model::types::PublishedRelation;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_execution_domain::domain::{operating::errors::OperatingError, table_model::types::Rows};
use ryo_relations::domain::relations::service::kinds;
use ryo_relations::domain::relations::types::{ColumnsView, Relation};

/// What a publication of the right kind asks of the owner.
#[derive(Clone, Copy, Debug)]
enum Ask {
    Inspect,
    Admit,
    Resolve,
}

/// Who issued the publication the ask is made on behalf of.
#[derive(Clone, Copy, Debug)]
enum Issuer {
    /// A publication the owner never issued, of the right kind.
    Forged,
    /// The owner's own publication of the source.
    Owner,
}

/// The owner's refusal, and the checks it spent on the forged publication.
#[derive(Debug, PartialEq)]
struct Refused {
    error: OperatingError,
    checks: CheckCount,
}

type Input = (Issuer, Ask);
/// The checks the owner spent on an issued publication it admitted.
type Value = CheckCount;
type Failure = Refused;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_issued_publication_of_the_right_kind_is_admitted"),
        word: Word::Succeeds,
        input: || (Issuer::Owner, Ask::Admit),
        expect: Expect::Value(CheckCount(1)),
    },
    Case {
        name: CaseName("a_foreign_publication_with_correct_kind_has_no_admission_authority"),
        word: Word::Refuses,
        input: || (Issuer::Forged, Ask::Admit),
        expect: Expect::Error(Refused {
            error: OperatingError::NotRead,
            checks: CheckCount(0),
        }),
    },
    Case {
        name: CaseName("a_foreign_publication_with_correct_kind_is_not_inspected"),
        word: Word::Refuses,
        input: || (Issuer::Forged, Ask::Inspect),
        expect: Expect::Error(Refused {
            error: OperatingError::NotRead,
            checks: CheckCount(0),
        }),
    },
    Case {
        name: CaseName("a_foreign_publication_with_correct_kind_acquires_no_provider"),
        word: Word::Refuses,
        input: || (Issuer::Forged, Ask::Resolve),
        expect: Expect::Error(Refused {
            error: OperatingError::NotRead,
            checks: CheckCount(0),
        }),
    },
];

/// Asks a fresh owner, over the line count source, on behalf of the publication.
fn subject((issuer, ask): Input) -> Result<Value, Failure> {
    let owner = RelationOwner::new(
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        std::sync::Arc::new(crate::application::file_dag::fixtures::derived()),
    );
    let relation = Relation::FilesystemLineCount;
    let physical = columns(kinds(relation));
    let rows = Rows::Stored(ColumnsView {
        kinds: kinds(relation),
        columns: &physical,
    });
    let refused = |error| Refused {
        error,
        checks: owner.checks.get(),
    };
    let issued = match issuer {
        Issuer::Forged => None,
        Issuer::Owner => Some(owner.source(relation, &rows).map_err(refused)?),
    };
    let publication: &dyn PublishedRelation = match &issued {
        Some(issued) => issued.as_ref(),
        None => &Forged,
    };
    let answer = match ask {
        Ask::Inspect => owner
            .inspect(publication, RepresentationInput::from(&rows))
            .map(drop),
        Ask::Admit => owner
            .admit(publication, RepresentationInput::from(&rows))
            .map(drop),
        Ask::Resolve => owner
            .resolve(publication, &|_| {
                panic!("forged publication must not acquire a provider")
            })
            .map(drop),
    };
    answer.map(|()| owner.checks.get()).map_err(refused)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
