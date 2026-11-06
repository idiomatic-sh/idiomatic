//! The `plan_parts` service's cases, on values alone.

use super::service::{columns_of, each_column, each_column_mut, parameter_in, remapped, step_in};
use super::types::{
    Applied, ColumnIndex, Function, LiteralText, Operand, ParameterId, QueryLiteral, StepId,
};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// What a row of the cases asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// A step the plan has.
    StepThere,
    /// A step past the end of the plan.
    StepMissing,
    /// A parameter the plan has.
    ParameterThere,
    /// A parameter past the end of the plan.
    ParameterMissing,
    /// The columns a function's arguments read are the columns the operand reads.
    ColumnsInAFunction,
    /// The columns a function's arguments read are renamed with it.
    ColumnsInAFunctionRenamed,
    /// A function whose arguments are all fixed reads no column.
    FixedFunction,
    /// A function is remapped through its arguments, and not at all when one of its columns has no place.
    FunctionRemapped,
    /// Two comparisons that differ only in their diagnostic origin.
    OriginIgnored,
}

/// `concat(pascal(column 3), "Service", column 1)`.
fn nested() -> Operand {
    Operand::Function(Applied {
        function: Function::Concat,
        arguments: vec![
            Operand::Function(Applied {
                function: Function::Pascal,
                arguments: vec![Operand::Column(ColumnIndex(3))],
            }),
            Operand::Literal(QueryLiteral::Text(LiteralText("Service".into()))),
            Operand::Column(ColumnIndex(1)),
        ],
    })
}

/// What a row of the cases asks of the service.
type Input = Scenario;
/// Whether the one asked for was found, and was the right one.
type Value = usize;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_step_the_plan_has_is_found"),
        word: Word::Succeeds,
        input: || Scenario::StepThere,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_step_past_the_end_is_not_found"),
        word: Word::Succeeds,
        input: || Scenario::StepMissing,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_parameter_the_plan_has_is_found"),
        word: Word::Succeeds,
        input: || Scenario::ParameterThere,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_parameter_past_the_end_is_not_found"),
        word: Word::Succeeds,
        input: || Scenario::ParameterMissing,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_columns_of_a_function_are_the_columns_its_arguments_read"),
        word: Word::Succeeds,
        input: || Scenario::ColumnsInAFunction,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_columns_of_a_function_are_renamed_with_its_arguments"),
        word: Word::Succeeds,
        input: || Scenario::ColumnsInAFunctionRenamed,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_function_of_fixed_text_reads_no_column"),
        word: Word::Succeeds,
        input: || Scenario::FixedFunction,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_function_is_remapped_through_its_arguments"),
        word: Word::Succeeds,
        input: || Scenario::FunctionRemapped,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("comparison_equality_ignores_diagnostic_origin_ids"),
        word: Word::Succeeds,
        input: || Scenario::OriginIgnored,
        expect: Expect::Value(1),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    let items = [10_u8, 20];
    let answered = match scenario {
        Scenario::StepThere => step_in(&items, StepId(1)) == Some(&20),
        Scenario::StepMissing => step_in(&items, StepId(2)).is_none(),
        Scenario::ParameterThere => parameter_in(&items, ParameterId(0)) == Some(&10),
        Scenario::ParameterMissing => parameter_in(&items, ParameterId(2)).is_none(),
        Scenario::ColumnsInAFunction => {
            let mut read = Vec::new();
            each_column(&nested(), &mut |column| read.push(column));
            read == [ColumnIndex(3), ColumnIndex(1)] && columns_of(&nested()) == read
        }
        Scenario::ColumnsInAFunctionRenamed => {
            let mut operand = nested();
            each_column_mut(&mut operand, &mut |column| column.0 += 10);
            let mut read = Vec::new();
            each_column(&operand, &mut |column| read.push(column));
            read == [ColumnIndex(13), ColumnIndex(11)]
        }
        Scenario::FixedFunction => columns_of(&Operand::Function(Applied {
            function: Function::Snake,
            arguments: vec![Operand::Literal(QueryLiteral::Text(LiteralText(
                "TodoLists".into(),
            )))],
        }))
        .is_empty(),
        Scenario::OriginIgnored => {
            use super::types::{ComparisonOrigin, ComparisonOriginId, PlanComparison, Test};
            let comparison = |origin| PlanComparison {
                test: Test::Equal,
                left: Operand::Column(ColumnIndex(1)),
                right: Operand::Column(ColumnIndex(2)),
                origin,
            };
            comparison(ComparisonOrigin::Unrecorded)
                == comparison(ComparisonOrigin::Recorded(ComparisonOriginId(9)))
        }
        Scenario::FunctionRemapped => {
            let moved = remapped(&nested(), &|column| Some(ColumnIndex(column.0 + 1)));
            let mut read = Vec::new();
            if let Some(operand) = &moved {
                each_column(operand, &mut |column| read.push(column));
            }
            let lost = remapped(&nested(), &|column| (column.0 == 3).then_some(column));
            read == [ColumnIndex(4), ColumnIndex(2)] && lost.is_none()
        }
    };
    Ok(usize::from(answered))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
