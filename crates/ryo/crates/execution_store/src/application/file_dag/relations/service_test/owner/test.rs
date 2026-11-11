//! Catalog-wide physical access and owner-only admission, independent of semantic completeness.
use super::super::super::RelationOwner;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_execution_domain::domain::{
    coverage::types::{Lacking, Partial},
    operating::errors::OperatingError,
    table_model::{types::PhysicalShape, types::Rows},
};
use ryo_relations::domain::relations::{
    service::{all, kinds, to_bits},
    types::{CellKind, ColumnsView, FileId, Integer, Relation, Value as CellValue},
};
use ryo_values::domain::stored_facts::types::PartialCause;
use std::{cell::Cell, mem::size_of, sync::Arc};

fn compact_publications_share_backing_accounting_and_expire_with_generation() {
    use ryo_relations::domain::relations::types::Words;
    use ryo_values::domain::table_words::{
        service,
        types::{DecodeMode, WordSlice},
    };
    use ryo_values::{
        domain::rows::types::{RowCount, Table},
        domain::stored_facts::types::KeptWords,
    };

    let table = Table {
        kinds: vec![CellKind::Number],
        cells: vec![CellValue::Number(Integer(4)), CellValue::Number(Integer(5))],
        count: RowCount(2),
    };
    let mut encoded = Words::default();
    service::encode_table(&table, &[], &mut encoded);
    let words = Arc::new(KeptWords(service::pack(WordSlice(&encoded.0))));
    let words_weak = Arc::downgrade(&words);
    let ryo_values::domain::table_words::types::Admitted {
        table: compact,
        names,
    } = ryo_values::domain::compact_table::service::admit_compact(
        ryo_values::domain::table_words::types::CompactBacking(Arc::clone(&words)),
        &mut Vec::new(),
        DecodeMode::Complete,
    )
    .unwrap();
    assert!(names.is_empty());
    assert_eq!(
        ryo_values::domain::compact_table::service::compact_rectangular(&compact),
        ryo_values::domain::table_words::types::TableShape::Rectangular
    );
    let compact = Arc::new(compact);
    let compact_weak = Arc::downgrade(&compact);
    let owner = RelationOwner::new(
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        std::sync::Arc::new(crate::application::file_dag::fixtures::derived()),
    );
    let first = owner.source_compact(Arc::clone(&compact)).unwrap();
    owner
        .admit(first.as_ref(), RepresentationInput::Compact(&compact))
        .unwrap();
    let first_unique = owner.derived.publication_ownership_bytes().2;
    let second = owner.source_compact(Arc::clone(&compact)).unwrap();
    owner
        .admit(second.as_ref(), RepresentationInput::Compact(&compact))
        .unwrap();
    assert_eq!(
        first.same_record(&second),
        crate::application::file_dag::relations::types::Sameness::Different
    );
    // A second publication of the same immutable Compact and word buffer
    // adds only its own record. Shared backing allocations are not charged
    // again to the owner ledger.
    assert_eq!(
        owner.derived.publication_ownership_bytes().2,
        first_unique + size_of::<super::super::super::RelationResult>() + 2 * size_of::<usize>()
    );
    assert_eq!(
        first_unique,
        crate::application::file_dag::derived::ledger::capacity::compact_table(&compact).0
            + size_of::<ryo_values::domain::stored_facts::types::KeptWords>()
            + 2 * size_of::<usize>()
            + words.0.0.capacity()
            + size_of::<super::super::super::RelationResult>()
            + 2 * size_of::<usize>(),
    );
    let unread = |_| Err(OperatingError::NotRead);
    let resolved = owner.resolve(first.as_ref(), &unread).unwrap();
    assert_eq!(
        read(&resolved),
        [
            vec![CellValue::Number(Integer(4))],
            vec![CellValue::Number(Integer(5))]
        ]
    );
    drop(resolved);
    let consumer = first.clone();
    assert_eq!(owner.derived.publication_ownership_bytes().1, 0);
    drop(compact);
    drop(words);
    assert!(compact_weak.upgrade().is_some());
    assert!(words_weak.upgrade().is_some());
    owner.release_population();
    assert!(matches!(
        owner.resolve(first.as_ref(), &unread),
        Err(OperatingError::NotRead)
    ));
    assert!(matches!(
        owner.resolve(second.as_ref(), &unread),
        Err(OperatingError::NotRead)
    ));
    drop(first);
    drop(second);
    assert!(compact_weak.upgrade().is_some());
    assert!(owner.derived.publication_ownership_bytes().0 > 0);
    drop(consumer);
    assert!(compact_weak.upgrade().is_none());
    assert!(words_weak.upgrade().is_none());
    assert_eq!(owner.derived.publication_ownership_bytes(), (0, 0, 0));
}
use super::fixtures::{columns, group, read, row};

fn every_catalog_relation_is_admitted_resolved_and_read_with_its_actual_schema() {
    let owner = RelationOwner::new(
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        std::sync::Arc::new(crate::application::file_dag::fixtures::derived()),
    );
    let mut groups = [0; 3];
    for relation in all() {
        let schema = kinds(*relation);
        let physical = columns(schema);
        let source = ColumnsView {
            kinds: schema,
            columns: &physical,
        };
        let rows = Rows::Stored(source);
        let publication = owner.source(*relation, &rows).unwrap();
        assert_eq!(
            owner.inspect(publication.as_ref(), RepresentationInput::from(&rows)),
            Ok(PhysicalShape::Complete),
            "{relation:?}"
        );
        assert_eq!(
            owner.inspect(publication.as_ref(), RepresentationInput::from(&rows)),
            Ok(PhysicalShape::Complete),
            "{relation:?}"
        );
        let resolved = owner
            .resolve(publication.as_ref(), &|asked| {
                assert_eq!(asked, *relation);
                Ok(Rows::Stored(source))
            })
            .unwrap();
        assert_eq!(
            read(&resolved),
            [row(schema, 0), row(schema, 1)],
            "{relation:?}"
        );
        groups[group(*relation)] += 1;
    }
    assert_eq!(
        groups,
        [57, 8, 4],
        "69 catalog relations, separately from five physical representations"
    );
    assert_eq!(
        owner.checks.get(),
        crate::application::file_dag::relations::types::CheckCount(69)
    );
    assert_eq!(
        owner.reuses.get(),
        crate::application::file_dag::relations::types::CheckCount(69)
    );
}

fn population_epoch_invalidates_proofs_even_when_same_buffer_and_count_are_reused() {
    let relation = Relation::FilesystemLineCount;
    let schema = kinds(relation);
    let mut physical = columns(schema);
    let owner = RelationOwner::new(
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        std::sync::Arc::new(crate::application::file_dag::fixtures::derived()),
    );
    let original = {
        let rows = Rows::Stored(ColumnsView {
            kinds: schema,
            columns: &physical,
        });
        let publication = owner.source(relation, &rows).unwrap();
        assert_eq!(
            owner.inspect(publication.as_ref(), RepresentationInput::from(&rows)),
            Ok(PhysicalShape::Complete)
        );
        publication
    };
    let address = physical[1].0.as_ptr();
    owner.release_population();
    physical[1].0.clear();
    physical[1].0.extend([
        to_bits(CellValue::Number(Integer(200))).0,
        to_bits(CellValue::Number(Integer(201))).0,
    ]);
    assert_eq!(address, physical[1].0.as_ptr());
    assert_eq!(physical[1].0.len(), 2);
    let source = ColumnsView {
        kinds: schema,
        columns: &physical,
    };
    let rows = Rows::Stored(source);
    assert!(matches!(
        owner.inspect(original.as_ref(), RepresentationInput::from(&rows)),
        Err(OperatingError::NotRead)
    ));
    let reads = Cell::new(0);
    assert!(matches!(
        owner.resolve(original.as_ref(), &|_| {
            reads.set(reads.get() + 1);
            Ok(Rows::Stored(source))
        }),
        Err(OperatingError::NotRead)
    ));
    assert_eq!(
        reads.get(),
        0,
        "old epoch is refused before provider access"
    );
    let fresh = owner.source(relation, &rows).unwrap();
    assert_eq!(
        original.same_record(&fresh),
        crate::application::file_dag::relations::types::Sameness::Different
    );
    assert_eq!(
        owner.inspect(fresh.as_ref(), RepresentationInput::from(&rows)),
        Ok(PhysicalShape::Complete)
    );
    assert_eq!(
        owner.checks.get(),
        crate::application::file_dag::relations::types::CheckCount(2)
    );
    let resolved = owner
        .resolve(fresh.as_ref(), &|_| Ok(Rows::Stored(source)))
        .unwrap();
    let mut expected = [row(schema, 0), row(schema, 1)];
    expected[0][1] = CellValue::Number(Integer(200));
    expected[1][1] = CellValue::Number(Integer(201));
    assert_eq!(read(&resolved), expected);
    let other = RelationOwner::new(
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        std::sync::Arc::new(crate::application::file_dag::fixtures::derived()),
    );
    assert!(matches!(
        other.inspect(fresh.as_ref(), RepresentationInput::from(&rows)),
        Err(OperatingError::NotRead)
    ));
}

fn stable_partial_population_reuses_structure_without_proving_semantic_absence() {
    let relation = Relation::Function;
    let physical = columns(kinds(relation));
    let rows = Rows::Stored(ColumnsView {
        kinds: kinds(relation),
        columns: &physical,
    });
    let partials = [Partial {
        relation,
        cause: PartialCause("fixture partial".into()),
        lacking: Lacking::Any,
        at: ryo_execution_domain::domain::coverage::types::PartialPlace::File(FileId(9)),
    }];
    let owner = RelationOwner::new(
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        std::sync::Arc::new(crate::application::file_dag::fixtures::derived()),
    );
    let first = owner.source(relation, &rows).unwrap();
    let second = owner.source(relation, &rows).unwrap();
    assert_eq!(
        first.same_record(&second),
        crate::application::file_dag::relations::types::Sameness::Same
    );
    for publication in [&first, &second] {
        assert_eq!(
            owner.inspect(publication.as_ref(), RepresentationInput::from(&rows)),
            Ok(PhysicalShape::Complete)
        );
    }
    assert_eq!(
        owner.checks.get(),
        crate::application::file_dag::relations::types::CheckCount(1)
    );
    let prepared =
        crate::application::file_dag::derived::prepared::RequestDerived::new(Box::new([
            crate::application::file_dag::derived::owner::fixtures::reads(relation),
        ]));
    owner
        .derived
        .prepare_program_file(
            &prepared,
            &[ryo_execution_domain::domain::coverage::types::RuleIndex(0)],
            &partials,
        )
        .unwrap();
    assert!(
        !owner.derived.relation_eligible(relation),
        "physical completeness cannot erase a partial relation"
    );
}

fn normalization_reuses_one_owned_fallback_without_promoting_missing_rows() {
    let relation = Relation::SourceLocation;
    let schema = kinds(relation);
    let mut physical = columns(schema);
    physical[1].0.pop();
    let source = ColumnsView {
        kinds: schema,
        columns: &physical,
    };
    let owner = RelationOwner::new(
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        std::sync::Arc::new(crate::application::file_dag::fixtures::derived()),
    );
    let publication = owner
        .source(Relation::Function, &Rows::Stored(source))
        .unwrap();
    let first = owner.normalize(&publication, Rows::Stored(source)).unwrap();
    let second = owner.normalize(&publication, Rows::Stored(source)).unwrap();
    let (Rows::Computed(first_table), Rows::Computed(second_table)) = (&first, &second) else {
        panic!("ragged input needs an owned normalized population")
    };
    assert!(std::ptr::eq(*first_table, *second_table));
    assert_eq!(read(&first), [row(schema, 0)]);
    assert_eq!(read(&second), read(&first));
    assert_eq!(
        owner.checks.get(),
        crate::application::file_dag::relations::types::CheckCount(1)
    );
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    CompactPublicationsShareBackingAccountingAndExpireWithGeneration,
    EveryCatalogRelationIsAdmittedResolvedAndReadWithItsActualSchema,
    PopulationEpochInvalidatesProofsEvenWhenSameBufferAndCountAreReused,
    StablePartialPopulationReusesStructureWithoutProvingSemanticAbsence,
    NormalizationReusesOneOwnedFallbackWithoutPromotingMissingRows,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("compact_publications_share_backing_accounting_and_expire_with_generation"),
        word: Word::Succeeds,
        input: || Scenario::CompactPublicationsShareBackingAccountingAndExpireWithGeneration,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "every_catalog_relation_is_admitted_resolved_and_read_with_its_actual_schema",
        ),
        word: Word::Succeeds,
        input: || Scenario::EveryCatalogRelationIsAdmittedResolvedAndReadWithItsActualSchema,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "population_epoch_invalidates_proofs_even_when_same_buffer_and_count_are_reused",
        ),
        word: Word::Succeeds,
        input: || Scenario::PopulationEpochInvalidatesProofsEvenWhenSameBufferAndCountAreReused,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "stable_partial_population_reuses_structure_without_proving_semantic_absence",
        ),
        word: Word::Succeeds,
        input: || Scenario::StablePartialPopulationReusesStructureWithoutProvingSemanticAbsence,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("normalization_reuses_one_owned_fallback_without_promoting_missing_rows"),
        word: Word::Succeeds,
        input: || Scenario::NormalizationReusesOneOwnedFallbackWithoutPromotingMissingRows,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::CompactPublicationsShareBackingAccountingAndExpireWithGeneration => {
            compact_publications_share_backing_accounting_and_expire_with_generation();
            Ok(())
        }
        Scenario::EveryCatalogRelationIsAdmittedResolvedAndReadWithItsActualSchema => {
            every_catalog_relation_is_admitted_resolved_and_read_with_its_actual_schema();
            Ok(())
        }
        Scenario::PopulationEpochInvalidatesProofsEvenWhenSameBufferAndCountAreReused => {
            population_epoch_invalidates_proofs_even_when_same_buffer_and_count_are_reused();
            Ok(())
        }
        Scenario::StablePartialPopulationReusesStructureWithoutProvingSemanticAbsence => {
            stable_partial_population_reuses_structure_without_proving_semantic_absence();
            Ok(())
        }
        Scenario::NormalizationReusesOneOwnedFallbackWithoutPromotingMissingRows => {
            normalization_reuses_one_owned_fallback_without_promoting_missing_rows();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
