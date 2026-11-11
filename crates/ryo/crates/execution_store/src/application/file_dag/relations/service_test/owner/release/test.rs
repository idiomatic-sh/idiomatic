//! S15: through the store port, a publication is readable while its generation lives and refused once the store releases it.
use super::super::fixtures::row;
use crate::application::file_dag::relations::interface::RelationsPort;
use crate::application::file_dag::relations::publication::types::{ProducerScope, PublicationBody};
use crate::application::file_dag::relations::service::RelationOwner;
use crate::application::file_dag::relations::types::{ProducerId, ReuseEligibility};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::{service::kinds, types::Relation};
use ryo_values::domain::rows::types::{RowCount, Table};
use std::sync::Arc;

/// The column counts the port answered for a publication at each point of its store's life.
#[derive(Debug, PartialEq)]
struct Lifetime {
    /// Read right after publishing.
    published: Result<usize, OperatingError>,
    /// Read after its producer bound it and retired its derived views.
    after_retire: Result<usize, OperatingError>,
    /// Read after the store released its generation.
    after_release: Result<usize, OperatingError>,
    /// A publication of the next generation, read at once.
    later: Result<usize, OperatingError>,
}

type Input = Relation;
type Value = Lifetime;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_publication_is_read_while_its_generation_lives_and_refused_after_release"),
    word: Word::Succeeds,
    input: || Relation::Function,
    expect: Expect::Value(Lifetime {
        published: Ok(4),
        after_retire: Ok(4),
        after_release: Err(OperatingError::NotRead),
        later: Ok(4),
    }),
}];

/// One computed row of `relation`.
fn table(relation: Relation) -> Table {
    let schema = kinds(relation);
    Table {
        kinds: schema.to_vec(),
        cells: row(schema, 0),
        count: RowCount(1),
    }
}

/// Publishes a row of `relation` into a store of its own, retires its producer, releases the store and publishes again.
fn subject(relation: Relation) -> Result<Value, Failure> {
    let owner = RelationOwner::fixture(
        ReuseEligibility::Eligible,
        Arc::new(crate::outbound::derived::factory::semantics()),
    );
    let port: &dyn RelationsPort = &owner;
    let publication = port.publish(PublicationBody::Computed(table(relation)))?;
    let published = port.kinds(&publication).map(<[_]>::len);
    port.bind_producer(ProducerScope::File, ProducerId(0), &publication)?;
    port.retire_producer(ProducerScope::File, ProducerId(0));
    let after_retire = port.kinds(&publication).map(<[_]>::len);
    port.release();
    let after_release = port.kinds(&publication).map(<[_]>::len);
    let later = port.publish(PublicationBody::Computed(table(relation)))?;
    Ok(Lifetime {
        published,
        after_retire,
        after_release,
        later: port.kinds(&later).map(<[_]>::len),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
