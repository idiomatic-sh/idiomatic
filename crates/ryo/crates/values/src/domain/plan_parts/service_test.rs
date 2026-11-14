//! The `plan_parts` service's cases, on values alone.

use ryo_relations::domain::relations::collections::relations::syntax_child::item::SyntaxChildColumn;
use ryo_relations::domain::relations::collections::relations::syntax_tree::item::SyntaxTreeColumn;
use ryo_relations::domain::relations::service::{all, columns};
use ryo_relations::domain::relations::types::{Column, Relation};

use super::service::{
    column_place, columns_of, each_column, each_column_mut, parameter_in, relation_column,
    remapped, step_in, tree_lookup_key,
};
use super::types::{
    Applied, ColumnIndex, Function, LiteralText, Operand, ParameterId, QueryLiteral, StepId,
    TreeLookupKey,
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
    /// Every column of every relation is the column at its place.
    EveryColumnRoundTrips,
    /// The place of the syntax tree's `file`.
    PlaceOfTheElementFile,
    /// The place of the syntax child's `field`.
    PlaceOfTheEdgeField,
    /// A place past the last column of a relation.
    PlacePastTheColumns,
    /// The lookups the engine makes of the tree where it is: an element by id, span, or file and span; an edge by an end.
    TreeLookupsByTheirKeys,
    /// A lookup keyed on any other column of the tree or its edges, or on another relation, is not one.
    OtherLookupsAreNot,
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
    Case {
        name: CaseName("every_column_round_trips_through_its_place"),
        word: Word::Succeeds,
        input: || Scenario::EveryColumnRoundTrips,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_element_file_is_the_fourth_column_of_the_syntax_tree"),
        word: Word::Succeeds,
        input: || Scenario::PlaceOfTheElementFile,
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("the_edge_field_is_the_fourth_column_of_the_syntax_child"),
        word: Word::Succeeds,
        input: || Scenario::PlaceOfTheEdgeField,
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("a_place_past_the_columns_is_no_column"),
        word: Word::Succeeds,
        input: || Scenario::PlacePastTheColumns,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName(
            "the_tree_is_looked_up_by_an_element_id_span_or_file_and_span_or_an_edge_end",
        ),
        word: Word::Succeeds,
        input: || Scenario::TreeLookupsByTheirKeys,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_lookup_by_any_other_column_is_no_tree_lookup"),
        word: Word::Succeeds,
        input: || Scenario::OtherLookupsAreNot,
        expect: Expect::Value(1),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    let items = [10_u8, 20];
    let place = |column| usize::try_from(column_place(column).0).unwrap_or(usize::MAX);
    let answered = match scenario {
        Scenario::PlaceOfTheElementFile => {
            return Ok(place(Column::SyntaxTree(SyntaxTreeColumn::File)));
        }
        Scenario::PlaceOfTheEdgeField => {
            return Ok(place(Column::SyntaxChild(SyntaxChildColumn::Field)));
        }
        Scenario::EveryColumnRoundTrips => all().iter().all(|relation| {
            columns(*relation)
                .into_iter()
                .all(|column| relation_column(*relation, column_place(column)) == Some(column))
        }),
        Scenario::TreeLookupsByTheirKeys => {
            let element = |column| column_place(Column::SyntaxTree(column));
            let edge = |column| column_place(Column::SyntaxChild(column));
            let id = Column::SyntaxTree(SyntaxTreeColumn::Id);
            let parent = Column::SyntaxChild(SyntaxChildColumn::Parent);
            let child = Column::SyntaxChild(SyntaxChildColumn::Child);
            let (file, span) = (
                element(SyntaxTreeColumn::File),
                element(SyntaxTreeColumn::Span),
            );
            tree_lookup_key(Relation::SyntaxTree, &[column_place(id)])
                == Some(TreeLookupKey::Node(id))
                && tree_lookup_key(Relation::SyntaxChild, &[column_place(parent)])
                    == Some(TreeLookupKey::Node(parent))
                && tree_lookup_key(Relation::SyntaxChild, &[edge(SyntaxChildColumn::Child)])
                    == Some(TreeLookupKey::Node(child))
                && tree_lookup_key(Relation::SyntaxTree, &[span]) == Some(TreeLookupKey::Span)
                && tree_lookup_key(Relation::SyntaxTree, &[file, span])
                    == Some(TreeLookupKey::FileThenSpan)
                && tree_lookup_key(Relation::SyntaxTree, &[span, file])
                    == Some(TreeLookupKey::SpanThenFile)
        }
        Scenario::OtherLookupsAreNot => {
            let edge = |column| column_place(Column::SyntaxChild(column));
            let kind = column_place(Column::SyntaxTree(SyntaxTreeColumn::Kind));
            tree_lookup_key(Relation::SyntaxChild, &[edge(SyntaxChildColumn::Field)]).is_none()
                && tree_lookup_key(Relation::SyntaxTree, &[kind]).is_none()
                && tree_lookup_key(Relation::Function, &[ColumnIndex(0)]).is_none()
                && tree_lookup_key(
                    Relation::SyntaxChild,
                    &[
                        edge(SyntaxChildColumn::Parent),
                        edge(SyntaxChildColumn::Child),
                    ],
                )
                .is_none()
                && tree_lookup_key(Relation::SyntaxTree, &[]).is_none()
        }
        Scenario::PlacePastTheColumns => all().iter().all(|relation| {
            let width = u32::try_from(columns(*relation).len()).unwrap_or(u32::MAX);
            relation_column(*relation, ColumnIndex(width)).is_none()
        }),
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
