//! Original scan/transport/key assertions through pure scanning or real compiled execution.
use super::super::super::*;
use super::fixtures::{CheckedSink, context, files, indexed_join, project_into};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::replay::{service::scan, types::Collect};
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use ryo_values::domain::rows::errors::RowsError;
use ryo_values::domain::rows::types::Table;
fn a_scan_passes_on_every_row_of_a_table() {
    context(|context| {
        let from = files(3);
        let mut into = Table::default();
        scan(
            &Rows::Computed(&from),
            &mut vec![],
            context,
            &mut Collect(&mut into),
        )
        .unwrap();
        assert_eq!(into.cells, from.cells);
    });
}
fn a_scan_stops_at_the_row_the_taker_refuses() {
    context(|context| {
        let mut sink = CheckedSink {
            calls: 0,
            refuse: true,
            rows: vec![],
        };
        assert_eq!(
            scan(&Rows::Computed(&files(3)), &mut vec![], context, &mut sink),
            Err(OperatingError::NotRun)
        );
        assert_eq!(sink.calls, 1);
    });
}
fn the_engine_sink_reads_the_original_source_context() {
    let mut sink = CheckedSink {
        calls: 0,
        refuse: false,
        rows: vec![],
    };
    project_into(0, &mut sink).unwrap();
    assert_eq!(sink.rows, files(2).cells);
    assert_eq!(sink.calls, 2);
}
fn a_real_step_passes_on_the_rows_it_computes() {
    let mut into = CheckedSink {
        calls: 0,
        refuse: false,
        rows: vec![],
    };
    project_into(0, &mut into).unwrap();
    assert_eq!(into.rows, files(2).cells);
}
fn rows_the_real_computation_refuses_are_not_passed_on() {
    let mut sink = CheckedSink {
        calls: 0,
        refuse: false,
        rows: vec![],
    };
    assert!(matches!(
        project_into(99, &mut sink),
        Err(OperatingError::Table(RowsError::NoSuchColumn { index: 99 }))
    ));
    assert_eq!(sink.calls, 0);
}
fn a_key_finds_the_rows_that_have_it_in_input_order() {
    assert_eq!(
        indexed_join(1, 0).unwrap().cells,
        [Cell::Number(Integer(10)), Cell::Number(Integer(30))]
    );
}
fn a_key_no_row_has_finds_nothing() {
    assert_eq!(indexed_join(9, 0).unwrap().count.0, 0);
}
fn a_join_row_without_its_key_column_is_refused() {
    assert!(matches!(
        indexed_join(1, 3),
        Err(OperatingError::Table(RowsError::NoSuchColumn { index: 3 }))
    ));
}
fn computed_inputs_use_the_common_typed_index_without_a_native_tree_finding() {
    assert_eq!(
        indexed_join(2, 0).unwrap().cells,
        [Cell::Number(Integer(20))]
    );
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    AScanPassesOnEveryRowOfATable,
    AScanStopsAtTheRowTheTakerRefuses,
    TheEngineSinkReadsTheOriginalSourceContext,
    ARealStepPassesOnTheRowsItComputes,
    RowsTheRealComputationRefusesAreNotPassedOn,
    AKeyFindsTheRowsThatHaveItInInputOrder,
    AKeyNoRowHasFindsNothing,
    AJoinRowWithoutItsKeyColumnIsRefused,
    ComputedInputsUseTheCommonTypedIndexWithoutANativeTreeFinding,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_scan_passes_on_every_row_of_a_table"),
        word: Word::Succeeds,
        input: || Scenario::AScanPassesOnEveryRowOfATable,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_scan_stops_at_the_row_the_taker_refuses"),
        word: Word::Succeeds,
        input: || Scenario::AScanStopsAtTheRowTheTakerRefuses,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("the_engine_sink_reads_the_original_source_context"),
        word: Word::Succeeds,
        input: || Scenario::TheEngineSinkReadsTheOriginalSourceContext,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_real_step_passes_on_the_rows_it_computes"),
        word: Word::Succeeds,
        input: || Scenario::ARealStepPassesOnTheRowsItComputes,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("rows_the_real_computation_refuses_are_not_passed_on"),
        word: Word::Succeeds,
        input: || Scenario::RowsTheRealComputationRefusesAreNotPassedOn,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_key_finds_the_rows_that_have_it_in_input_order"),
        word: Word::Succeeds,
        input: || Scenario::AKeyFindsTheRowsThatHaveItInInputOrder,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_key_no_row_has_finds_nothing"),
        word: Word::Succeeds,
        input: || Scenario::AKeyNoRowHasFindsNothing,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_join_row_without_its_key_column_is_refused"),
        word: Word::Succeeds,
        input: || Scenario::AJoinRowWithoutItsKeyColumnIsRefused,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("computed_inputs_use_the_common_typed_index_without_a_native_tree_finding"),
        word: Word::Succeeds,
        input: || Scenario::ComputedInputsUseTheCommonTypedIndexWithoutANativeTreeFinding,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::AScanPassesOnEveryRowOfATable => {
            a_scan_passes_on_every_row_of_a_table();
            Ok(())
        }
        Scenario::AScanStopsAtTheRowTheTakerRefuses => {
            a_scan_stops_at_the_row_the_taker_refuses();
            Ok(())
        }
        Scenario::TheEngineSinkReadsTheOriginalSourceContext => {
            the_engine_sink_reads_the_original_source_context();
            Ok(())
        }
        Scenario::ARealStepPassesOnTheRowsItComputes => {
            a_real_step_passes_on_the_rows_it_computes();
            Ok(())
        }
        Scenario::RowsTheRealComputationRefusesAreNotPassedOn => {
            rows_the_real_computation_refuses_are_not_passed_on();
            Ok(())
        }
        Scenario::AKeyFindsTheRowsThatHaveItInInputOrder => {
            a_key_finds_the_rows_that_have_it_in_input_order();
            Ok(())
        }
        Scenario::AKeyNoRowHasFindsNothing => {
            a_key_no_row_has_finds_nothing();
            Ok(())
        }
        Scenario::AJoinRowWithoutItsKeyColumnIsRefused => {
            a_join_row_without_its_key_column_is_refused();
            Ok(())
        }
        Scenario::ComputedInputsUseTheCommonTypedIndexWithoutANativeTreeFinding => {
            computed_inputs_use_the_common_typed_index_without_a_native_tree_finding();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
