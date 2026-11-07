//! The `contributor_rows` service's cases: what an answer built from the absence of rows keeps, from what its operands lack.
use ryo_relations::domain::relations::types::FileId;
use crate::domain::completion::types::CoverageState;
use crate::domain::output_coverage::types::OperandOrdinal;
use super::types::AbsenceMask;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

/// What a row of the cases proves.
#[derive(Clone, Copy)]
enum Scenario {
    /// Contributor partiality filters absence answers before keys can be projected away.
    ContributorPartialityFiltersAbsenceAnswersBeforeKeysCanBeProjectedAway,
    /// A grouped answer withholds only the partitions of the affected files.
    AGroupedAnswerWithholdsOnlyThePartitionsOfTheAffectedFiles,
    /// An inapplicable file withholds nothing from a symbol keyed group and only its own rows from a file keyed one.
    AnInapplicableFileWithholdsNothingFromASymbolKeyedGroupAndOnlyItsOwnRowsFromAFileKeyedOne,
}

/// What a row of the cases asks.
type Input = Scenario;
type Value = ();
type Failure = ();

fn contributor_partiality_filters_absence_answers_before_keys_can_be_projected_away() {
    use super::service::filter_contributor_rows;
    use crate::domain::operator::contract::service::semantics;
    use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
    use ryo_values::domain::{
        plan_parts::types::{ColumnIndex, Input, StepId},
        plans::{service::kind_of, types::Operation},
        rows::types::{RowCount, Table},
    };
    let left = Input::Step(StepId(0));
    let right = Input::Step(StepId(1));
    let candidates = [
        Cell::File(FileId(1)),
        Cell::Number(Integer(10)),
        Cell::File(FileId(0)),
        Cell::Number(Integer(20)),
        Cell::File(FileId(0)),
        Cell::Number(Integer(20)),
        Cell::File(FileId(2)),
        Cell::Number(Integer(30)),
    ];
    let operations = [
        Operation::Antijoin {
            left,
            right,
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
        },
        Operation::Difference { left, right },
        Operation::GroupCount {
            from: left,
            keys: vec![ColumnIndex(0)],
            distinct: None,
        },
        Operation::GroupDigest {
            from: left,
            keys: vec![ColumnIndex(0)],
            over: ColumnIndex(1),
        },
    ];
    for operation in operations {
        let absence = semantics(kind_of(&operation)).absence;
        let input_kinds = vec![vec![CellKind::File, CellKind::Number]; 2];
        let digest = matches!(operation, Operation::GroupDigest { .. });
        let extend_digest = |cells: &[Cell]| {
            cells
                .chunks(2)
                .flat_map(|row| {
                    let mut row = row.to_vec();
                    if digest {
                        row.push(Cell::Number(Integer(1)));
                    }
                    row
                })
                .collect::<Vec<_>>()
        };
        for affected in [vec![Some(FileId(2)), Some(FileId(1))], vec![None]] {
            let mut kinds = vec![CellKind::File, CellKind::Number];
            if digest {
                kinds.push(CellKind::Number);
            }
            let mut rows = Table {
                kinds,
                cells: extend_digest(&candidates),
                count: RowCount(4),
            };
            let contributions = input_kinds
                .iter()
                .enumerate()
                .map(|(ordinal, kinds)| crate::domain::output_coverage::types::InputCoverage {
                    ordinal: OperandOrdinal(ordinal),
                    kinds,
                    coverage: CoverageState::Partial {
                        files: affected.clone(),
                        lacks: vec![],
                    },
                    lineage: Default::default(),
                })
                .collect::<Vec<_>>();
            filter_contributor_rows(&operation, absence, &contributions, &mut rows);
            assert_eq!(
                rows.cells,
                if affected.contains(&None) {
                    vec![]
                } else {
                    extend_digest(&[
                        Cell::File(FileId(0)),
                        Cell::Number(Integer(20)),
                        Cell::File(FileId(0)),
                        Cell::Number(Integer(20)),
                    ])
                }
            );
        }
    }
    let global_count = Operation::GroupCount {
        from: left,
        keys: vec![],
        distinct: None,
    };
    let mut count = Table {
        kinds: vec![CellKind::Number],
        cells: vec![Cell::Number(Integer(4))],
        count: RowCount(1),
    };
    filter_contributor_rows(
        &global_count,
        AbsenceMask(1),
        &[crate::domain::output_coverage::types::InputCoverage {
            ordinal: OperandOrdinal(0),
            kinds: &[CellKind::File],
            coverage: CoverageState::Partial {
                files: vec![Some(FileId(1))],
                lacks: vec![],
            },
            lineage: Default::default(),
        }],
        &mut count,
    );
    assert_eq!(
        count.count,
        RowCount(0),
        "an unkeyed aggregate cannot localize uncertainty"
    );
}

/// A count keyed by a later column of its operand answers where its key sits in the answer, whichever schema the caller probes with.
fn a_grouped_answer_withholds_only_the_partitions_of_the_affected_files() {
    use crate::domain::coverage::service::drop_uncertain;
    use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
    use ryo_values::domain::{
        plan_parts::types::{ColumnIndex, Input, StepId},
        plans::types::Operation,
        rows::types::{RowCount, Table},
    };
    let operation = Operation::GroupCount {
        from: Input::Step(StepId(0)),
        keys: vec![ColumnIndex(1)],
        distinct: None,
    };
    let answer = || Table {
        kinds: vec![CellKind::File, CellKind::Number],
        cells: (0..3)
            .flat_map(|file| [Cell::File(FileId(file)), Cell::Number(Integer(1))])
            .collect(),
        count: RowCount(3),
    };
    let operand = [CellKind::Number, CellKind::File];
    for probed in [&operand[..], &answer().kinds[..]] {
        let mut skipped = answer();
        drop_uncertain(&operation, probed, &mut skipped, &[Some(FileId(1))]);
        assert_eq!(skipped.count, RowCount(2), "the other partitions answer");
        assert!(
            !skipped.cells.contains(&Cell::File(FileId(1))),
            "the partition of the skipped file is withheld, not answered"
        );
        let mut repository = answer();
        drop_uncertain(&operation, probed, &mut repository, &[None]);
        assert_eq!(
            repository.count,
            RowCount(0),
            "an unlocated cause withholds all"
        );
    }
}

/// A file no language reads, or whose syntax a rule skipped, is out of scope for a group not keyed by file and costs a group keyed
/// by file only its own rows; it is never reported complete for the rows keyed to it.
fn an_inapplicable_file_withholds_nothing_from_a_symbol_keyed_group_and_only_its_own_rows_from_a_file_keyed_one()
 {
    use super::service::filter_contributor_rows;
    use crate::domain::output_coverage::service::output_coverage;
    use crate::domain::output_coverage::types::InputCoverage;
    use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
    use ryo_relations::domain::terms::types::Symbol;
    use ryo_values::domain::{
        plan_parts::types::{ColumnIndex, Input, StepId},
        plans::types::Operation,
        rows::types::{RowCount, Table},
    };
    let group = |key: u32| Operation::GroupCount {
        from: Input::Step(StepId(0)),
        keys: vec![ColumnIndex(key)],
        distinct: None,
    };
    let operand = [CellKind::Symbol, CellKind::File];
    let coverage = || CoverageState::Inapplicable {
        files: vec![FileId(1)],
    };
    let by_symbol = Table {
        kinds: vec![CellKind::Symbol, CellKind::Number],
        cells: (0..3)
            .flat_map(|name| [Cell::Symbol(Symbol(name)), Cell::Number(Integer(1))])
            .collect(),
        count: RowCount(3),
    };
    let mut kept = by_symbol.clone();
    filter_contributor_rows(
        &group(0),
        AbsenceMask(1),
        &[InputCoverage {
            ordinal: OperandOrdinal(0),
            kinds: &operand,
            coverage: coverage(),
            lineage: Default::default(),
        }],
        &mut kept,
    );
    assert_eq!(
        kept.count,
        RowCount(3),
        "a symbol-keyed group withholds nothing"
    );
    assert_eq!(
        output_coverage(
            &group(0),
            &kept.kinds,
            &[InputCoverage {
                ordinal: OperandOrdinal(0),
                kinds: &operand,
                coverage: coverage(),
                lineage: Default::default(),
            }],
        )
        .unwrap(),
        CoverageState::Complete,
        "an answer with no file to be keyed by is not made partial by an inapplicable file"
    );
    let by_file = Table {
        kinds: vec![CellKind::File, CellKind::Number],
        cells: (0..3)
            .flat_map(|file| [Cell::File(FileId(file)), Cell::Number(Integer(1))])
            .collect(),
        count: RowCount(3),
    };
    let mut kept = by_file.clone();
    filter_contributor_rows(
        &group(1),
        AbsenceMask(1),
        &[InputCoverage {
            ordinal: OperandOrdinal(0),
            kinds: &operand,
            coverage: coverage(),
            lineage: Default::default(),
        }],
        &mut kept,
    );
    assert_eq!(
        kept.count,
        RowCount(2),
        "a file-keyed group withholds its own rows"
    );
    assert!(!kept.cells.contains(&Cell::File(FileId(1))));
    assert_eq!(
        output_coverage(
            &group(1),
            &kept.kinds,
            &[InputCoverage {
                ordinal: OperandOrdinal(0),
                kinds: &operand,
                coverage: coverage(),
                lineage: Default::default(),
            }],
        )
        .unwrap(),
        coverage(),
        "the rows keyed to the file stay inapplicable, never complete"
    );
    let absence = Operation::Antijoin {
        left: Input::Step(StepId(0)),
        right: Input::Step(StepId(1)),
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(0)],
    };
    let mut checked = by_symbol.clone();
    filter_contributor_rows(
        &absence,
        AbsenceMask(2),
        &[InputCoverage {
            ordinal: OperandOrdinal(1),
            kinds: &operand,
            coverage: coverage(),
            lineage: Default::default(),
        }],
        &mut checked,
    );
    assert_eq!(
        checked.count,
        RowCount(0),
        "a skipped file is still unavailable for an absence check"
    );
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("contributor_partiality_filters_absence_answers_before_keys_can_be_projected_away"),
        word: Word::Succeeds,
        input: || Scenario::ContributorPartialityFiltersAbsenceAnswersBeforeKeysCanBeProjectedAway,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_grouped_answer_withholds_only_the_partitions_of_the_affected_files"),
        word: Word::Succeeds,
        input: || Scenario::AGroupedAnswerWithholdsOnlyThePartitionsOfTheAffectedFiles,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("an_inapplicable_file_withholds_nothing_from_a_symbol_keyed_group_and_only_its_own_rows_from_a_file_keyed_one"),
        word: Word::Succeeds,
        input: || Scenario::AnInapplicableFileWithholdsNothingFromASymbolKeyedGroupAndOnlyItsOwnRowsFromAFileKeyedOne,
        expect: Expect::Value(()),
    },
];

fn subject(input: Input) {
    match input {
        Scenario::ContributorPartialityFiltersAbsenceAnswersBeforeKeysCanBeProjectedAway => contributor_partiality_filters_absence_answers_before_keys_can_be_projected_away(),
        Scenario::AGroupedAnswerWithholdsOnlyThePartitionsOfTheAffectedFiles => a_grouped_answer_withholds_only_the_partitions_of_the_affected_files(),
        Scenario::AnInapplicableFileWithholdsNothingFromASymbolKeyedGroupAndOnlyItsOwnRowsFromAFileKeyedOne => an_inapplicable_file_withholds_nothing_from_a_symbol_keyed_group_and_only_its_own_rows_from_a_file_keyed_one(),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
