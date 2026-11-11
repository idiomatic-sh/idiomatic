//! Required publication aliases share normalization and release its backing at the last consumer.
use super::super::super::super::RelationOwner;
use super::super::fixtures::{columns, read, row};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::{operating::errors::OperatingError, table_model::types::Rows};
use ryo_relations::domain::relations::{
    service::kinds,
    types::{CellKind, ColumnsView, Integer, Relation, Value as CellValue},
};
use std::sync::Arc;

fn lazy_source_normalization_is_shared_and_lives_until_the_last_consumer() {
    let owner = RelationOwner::new(
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        std::sync::Arc::new(crate::application::file_dag::fixtures::derived()),
    );
    let relation = Relation::SourceLocation;
    let schema = kinds(relation);
    let mut physical = columns(schema);
    physical[1].0.pop();
    let source = ColumnsView {
        kinds: schema,
        columns: &physical,
    };
    let publication = owner.source(relation, &Rows::Stored(source)).unwrap();
    let consumer = publication.clone();
    let weak = Arc::downgrade(publication.record());
    assert_eq!(
        owner.checks.get(),
        crate::application::file_dag::relations::types::CheckCount(0),
        "publication does not inspect undemanded source rows"
    );
    assert!(publication.as_ref().normalized.get().is_none());
    assert_eq!(owner.derived.publication_ownership_bytes().1, 0);
    let normalized_address;
    {
        let Rows::Computed(table) = owner.normalize(&publication, Rows::Stored(source)).unwrap()
        else {
            panic!("readable physical fallback must normalize")
        };
        normalized_address = std::ptr::from_ref(table);
        assert_eq!(read(&Rows::Computed(table)), [row(schema, 0)]);
    }
    let charged = owner.derived.publication_ownership_bytes();
    assert!(charged.0 > 0);
    assert_eq!(charged.1, 0);
    assert_eq!(charged.0, charged.2);
    drop(publication);
    assert!(weak.upgrade().is_some());
    let Rows::Computed(table) = owner.normalize(&consumer, Rows::Stored(source)).unwrap() else {
        panic!("same demanded fallback")
    };
    assert_eq!(normalized_address, std::ptr::from_ref(table));
    assert_eq!(
        owner.checks.get(),
        crate::application::file_dag::relations::types::CheckCount(1)
    );
    assert_eq!(
        owner.derived.publication_ownership_bytes(),
        charged,
        "alias does not charge normalization again"
    );
    drop(consumer);
    assert!(weak.upgrade().is_none());
    assert_eq!(owner.derived.publication_ownership_bytes(), (0, 0, 0));
}

fn computed_publication_clones_keep_required_backing_until_last_handle_in_live_generation() {
    use ryo_values::domain::rows::types::{RowCount, Table};
    let owner = RelationOwner::new(
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        crate::application::file_dag::fixtures::program_derived_port(),
    );
    let generation = owner.current_generation();
    let publication = owner
        .computed(Table {
            kinds: vec![CellKind::Number],
            cells: vec![
                CellValue::Number(Integer(41)),
                CellValue::Number(Integer(43)),
            ],
            count: RowCount(2),
        })
        .unwrap();
    let record = Arc::downgrade(publication.record());
    let alias = publication.clone();
    assert_eq!(
        alias.same_record(&publication),
        crate::application::file_dag::relations::types::Sameness::Same
    );
    let charged = owner.derived.publication_ownership_bytes();
    assert!(
        charged.0 > 0,
        "actual computed publication has required backing"
    );
    assert_eq!(charged.1, 0, "publication is not optional retention");
    assert_eq!(charged.0, charged.2, "one physical required backing charge");
    drop(publication);
    assert!(record.upgrade().is_some());
    assert_eq!(owner.derived.publication_ownership_bytes(), charged);
    {
        let rows = owner
            .resolve(alias.as_ref(), &|_| Err(OperatingError::NotRead))
            .unwrap();
        assert_eq!(
            read(&rows),
            [
                vec![CellValue::Number(Integer(41))],
                vec![CellValue::Number(Integer(43))]
            ]
        );
        assert_eq!(owner.derived.publication_ownership_bytes(), charged);
    }
    assert_eq!(
        owner.matches_generation(&generation),
        crate::application::file_dag::relations::types::Sameness::Same
    );
    assert_eq!(
        owner.current_generation().is_live(),
        crate::application::file_dag::relations::types::Liveness::Live
    );
    drop(alias);
    assert!(
        record.upgrade().is_none(),
        "last leased handle releases the actual record"
    );
    assert_eq!(owner.derived.publication_ownership_bytes(), (0, 0, 0));
    assert_eq!(
        owner.matches_generation(&generation),
        crate::application::file_dag::relations::types::Sameness::Same,
        "release did not rely on retiring the generation"
    );
    assert_eq!(
        owner.current_generation().is_live(),
        crate::application::file_dag::relations::types::Liveness::Live
    );
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    LazySourceNormalizationIsSharedAndLivesUntilTheLastConsumer,
    ComputedPublicationClonesKeepRequiredBackingUntilLastHandleInLiveGeneration,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("lazy_source_normalization_is_shared_and_lives_until_the_last_consumer"),
        word: Word::Succeeds,
        input: || Scenario::LazySourceNormalizationIsSharedAndLivesUntilTheLastConsumer,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "computed_publication_clones_keep_required_backing_until_last_handle_in_live_generation",
        ),
        word: Word::Succeeds,
        input: || {
            Scenario::ComputedPublicationClonesKeepRequiredBackingUntilLastHandleInLiveGeneration
        },
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::LazySourceNormalizationIsSharedAndLivesUntilTheLastConsumer => {
            lazy_source_normalization_is_shared_and_lives_until_the_last_consumer();
            Ok(())
        }
        Scenario::ComputedPublicationClonesKeepRequiredBackingUntilLastHandleInLiveGeneration => {
            computed_publication_clones_keep_required_backing_until_last_handle_in_live_generation(
            );
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
