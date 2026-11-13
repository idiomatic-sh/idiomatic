//! Behavioral proof of `population`: the identity an admitted input carries, the same for one publication and new
//! after its population is released.
use super::service::PopulationId;
use crate::application::file_dag::relations::service::RelationOwner;
use crate::application::file_dag::relations::types::ReuseEligibility;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::domain::rows::types::{RowCount, Table};
use std::sync::Arc;

/// The publications whose admissions are compared.
#[derive(Clone, Copy, Debug)]
enum Published {
    /// One table published once and admitted twice.
    Once,
    /// Two tables, each published.
    Twice,
    /// One table, then another after the population is released.
    Released,
}

type Input = Published;
type Value = bool;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("one_publication_admitted_twice_has_one_population"),
        word: Word::Succeeds,
        input: || Published::Once,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("two_publications_are_two_populations"),
        word: Word::Succeeds,
        input: || Published::Twice,
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("a_released_population_is_never_the_next_one"),
        word: Word::Succeeds,
        input: || Published::Released,
        expect: Expect::Value(false),
    },
];

fn table(value: i64) -> Table {
    Table {
        kinds: vec![CellKind::Number],
        cells: vec![Cell::Number(Integer(value))],
        count: RowCount(1),
    }
}

/// Publishes `value` and answers the population its resolved rows are admitted under.
fn admitted(owner: &RelationOwner, value: i64) -> Result<PopulationId, OperatingError> {
    let publication = owner.computed(table(value))?;
    let rows = owner.resolve(publication.as_ref(), &|_| Err(OperatingError::NotRead))?;
    let proof = owner.admit(publication.as_ref(), RepresentationInput::from(&rows))?;
    Ok(proof.identity(owner)?.population())
}

/// Whether the two admissions `input` makes carry equal population identities.
fn subject(input: Input) -> Result<Value, Failure> {
    let owner = RelationOwner::new(
        ReuseEligibility::Eligible,
        Arc::new(crate::application::file_dag::fixtures::derived()),
    );
    let first = admitted(&owner, 7)?;
    let second = match input {
        Published::Once => {
            let publication = owner.computed(table(8))?;
            let rows = owner.resolve(publication.as_ref(), &|_| Err(OperatingError::NotRead))?;
            let once = owner.admit(publication.as_ref(), RepresentationInput::from(&rows))?;
            let again = owner.admit(publication.as_ref(), RepresentationInput::from(&rows))?;
            return Ok(once.identity(&owner)?.population() == again.identity(&owner)?.population());
        }
        Published::Twice => admitted(&owner, 8)?,
        Published::Released => {
            owner.release_population();
            admitted(&owner, 8)?
        }
    };
    Ok(first == second)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
