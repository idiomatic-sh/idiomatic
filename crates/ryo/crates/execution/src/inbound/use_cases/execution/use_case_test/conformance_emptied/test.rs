//! Each operation's presence declaration rests on its kernel: an input computed empty inside a region that runs gives
//! no row.
use super::super::super::fixtures as owner_fixtures;
use super::super::conformance::fixtures::ObservedDelivery;
use super::super::conformance::fixtures::RefusalOutcome;
use crate::inbound::use_cases::execution::interface::OperatingError;
use crate::inbound::use_cases::execution::interface::SemanticLookup;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::expectations::SOURCE;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::values::KnownSyntax;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::*;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::cases::Case;
use ryo_values::cases::CaseName;
use ryo_values::cases::Expect;
use ryo_values::cases::Word;
use ryo_values::cases::run;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::rows::types::Table;
use std::sync::Arc;

/// Delivers an operation whose listed inputs have no rows while the region that computes it runs. Each such input is
/// the difference of its authored rows with themselves: its presence is that of the authored rows, which have some, so
/// the operation's kernel is evaluated over an empty input instead of being found absent.
fn compare_emptied(
    operation: &Operation,
    inputs: &[Table],
    emptied: &[usize],
    bindings: &[ryo_values::domain::query_bindings::types::ParameterBinding],
    source: &[u8],
    symbols: &mut Symbols,
    semantic: Option<&dyn SemanticLookup>,
) -> Result<ObservedDelivery, OperatingError> {
    use ryo_values::domain::plan_parts::types::{Input, StepId, StepScope};
    use ryo_values::domain::plans::types::PlanStep;
    let schema = super::super::conformance::test::schema(operation, inputs, bindings);
    let mut authored =
        super::super::conformance::test::author(operation, inputs, bindings, symbols);
    let mut last = authored
        .plan
        .steps
        .pop()
        .expect("the operation under proof is the authored plan's last step");
    let mut sources = Vec::new();
    ryo_values::domain::plans::service::each_input(&mut last.operation, &mut |input| {
        sources.push(*input)
    });
    let mut replaced = Vec::new();
    for (at, source) in sources.iter().enumerate() {
        if emptied.contains(&at) {
            replaced.push(Input::Step(StepId(
                u32::try_from(authored.plan.steps.len()).expect("a step number fits in u32"),
            )));
            authored.plan.steps.push(PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Difference {
                    left: *source,
                    right: *source,
                },
            });
        } else {
            replaced.push(*source);
        }
    }
    let mut at = 0;
    ryo_values::domain::plans::service::each_input(&mut last.operation, &mut |input| {
        *input = replaced[at];
        at += 1;
    });
    authored.plan.steps.push(last);
    authored.plan.emit.step =
        StepId(u32::try_from(authored.plan.steps.len() - 1).expect("a step number fits in u32"));
    super::super::conformance::fixtures::deliver(
        authored,
        schema,
        bindings,
        source,
        symbols,
        None,
        semantic,
        RefusalOutcome::Failed,
        Arc::new(owner_fixtures::operators()),
    )
}

/// Each operation's presence declaration rests on its kernel: a required input with no rows, the others as authored, gives
/// no row; a union of two empty inputs gives none; and a reach join over no edge finds no pair, so closure is not
/// reflexive. Each emptied input is computed empty inside a region that runs, so the kernel answers, not the presence
/// decision.
fn every_required_input_empty_answers_no_rows() -> Result<(), Failure> {
    use ryo_execution_domain::domain::operator::contract::presence::types::PresenceMask;
    let semantic = KnownSyntax;
    let rows = |case: &mut expectations::Fixture, emptied: &[usize]| {
        compare_emptied(
            &case.operation,
            &case.inputs,
            emptied,
            &case.bindings,
            SOURCE,
            &mut case.symbols,
            Some(&semantic),
        )
        .map(|observed| {
            (
                observed.collected.map(|table| table.count.0),
                observed.rows.len(),
            )
        })
    };
    for kind in OperationKind::ALL {
        let presence =
            ryo_execution_domain::domain::operator::contract::service::semantics(*kind).presence;
        let width = fixture(*kind).inputs.len();
        let emptied: Vec<Vec<usize>> = match presence {
            PresenceMask::AllOf(mask) => (0..width)
                .filter(|at| mask.0 & (1 << at) != 0)
                .map(|at| vec![at])
                .collect(),
            PresenceMask::AnyOf(mask) => {
                vec![(0..width).filter(|at| mask.0 & (1 << at) != 0).collect()]
            }
            PresenceMask::Always => continue,
        };
        assert!(
            !emptied.is_empty(),
            "{kind:?} declares a required input its fixture lacks"
        );
        for inputs in emptied {
            let mut case = fixture(*kind);
            assert_eq!(
                rows(&mut case, &inputs)?,
                (Ok(0), 0),
                "{kind:?} with inputs {inputs:?} empty"
            );
        }
    }
    let mut reach = fixture(OperationKind::Join);
    if let Operation::Join { reach: lookup, .. } = &mut reach.operation {
        *lookup = Some(ryo_values::domain::plan_parts::types::ReachLookup {
            source: ryo_values::domain::plan_parts::types::ColumnIndex(0),
            destination: ryo_values::domain::plan_parts::types::ColumnIndex(1),
            keys: vec![],
            columns: vec![
                ryo_values::domain::plan_parts::types::ColumnIndex(0),
                ryo_values::domain::plan_parts::types::ColumnIndex(1),
            ],
        });
    }
    assert_eq!(
        rows(&mut reach, &[1])?,
        (Ok(0), 0),
        "a reach join over no edge"
    );
    Ok(())
}
type Input = fn() -> Result<Value, Failure>;
type Value = ();
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName(stringify!(every_required_input_empty_answers_no_rows)),
    word: Word::Succeeds,
    input: || every_required_input_empty_answers_no_rows,
    expect: Expect::Value(()),
}];
fn subject(input: Input) -> Result<Value, Failure> {
    input()
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
