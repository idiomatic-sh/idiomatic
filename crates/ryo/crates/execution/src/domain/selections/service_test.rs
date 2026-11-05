//! The `selections` service's cases, over the small tree.

use super::service::{project, select_rows, select_tree};
use super::types::ProjectedColumns;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::lowering::types::ElementPart;
use crate::domain::operating::types::OperatorContext;
use crate::domain::positions::service_test::{
    SMALL_SOURCE, SmallTree, lowering_of, room_of, rows_over, small_tree,
};
use crate::domain::positions::types::{Slots, Work};
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Comparison, Operand, QueryLiteral, StepId, Test,
};

/// What a row of the cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// Selects the elements of the tree of the kind `word`.
    TreeWords,
    /// Selects the rows of a step whose element is of the kind `word`.
    RowsOfWords,
    /// Keeps one row of each kind of the rows of a step.
    Distinct,
    /// Keeps every row of a step.
    Kept,
}

/// What a row of the cases does.
type Input = Scenario;
/// The positions that remain, shown.
type Value = &'static str;
type Failure = crate::domain::operating::errors::OperatingError;

fn kind_is_word() -> Comparison {
    Comparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(1)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText("word".into()))),
        origin: None,
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let SmallTree {
        elements,
        mut symbols,
    } = small_tree();
    let mut derived = crate::application::file_dag::testing::raw_scope()?;
    let mut out = Slots::default();
    let lowering = lowering_of(&[ElementPart::Kind]);
    let mut room = room_of(&[&[1, 2, 3]]);
    {
        let rows = rows_over(&elements);
        let mut scratch = OperatorScratch::default();
        let mut memo = Vec::new();
        let mut context = OperatorContext {
            semantic: None,
            prepared_kinds: None,
            prepared_matchers: None,
            bindings: &[],
            symbols: (&mut symbols).into(),
            source: SourceView(SMALL_SOURCE),
            paths: &[],
            arenas: &[],
            memo: &mut memo,
            scratch: &mut scratch,
        };
        match scenario {
            Scenario::TreeWords => select_tree(&rows, &[kind_is_word()], &mut context, &mut out)?,
            Scenario::RowsOfWords => {
                let mut on_kind = kind_is_word();
                on_kind.left = Operand::Column(ColumnIndex(0));
                select_rows(
                    &rows,
                    &lowering,
                    StepId(0),
                    &[on_kind],
                    Work {
                        derived: &mut derived,
                        room: &mut room,
                        context: &mut context,
                    },
                    &mut out,
                )?;
            }
            Scenario::Distinct | Scenario::Kept => {
                project(
                    &mut derived,
                    &rows,
                    &lowering,
                    StepId(0),
                    ProjectedColumns {
                        columns: &[ColumnIndex(0)],
                        repeats: if matches!(scenario, Scenario::Distinct) {
                            RepeatedRows::Dropped
                        } else {
                            RepeatedRows::Kept
                        },
                    },
                    &mut room,
                    &mut out,
                )?;
            }
        }
    }
    // The answer is leaked: a case compares it with text that lives for the whole run.
    Ok(Box::leak(
        out.0
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(" ")
            .into_boxed_str(),
    ))
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_elements_of_the_tree_of_a_kind_are_selected"),
        word: Word::Succeeds,
        input: || Scenario::TreeWords,
        expect: Expect::Value("1 3"),
    },
    Case {
        name: CaseName("the_rows_of_a_step_whose_element_is_of_a_kind_are_selected"),
        word: Word::Succeeds,
        input: || Scenario::RowsOfWords,
        expect: Expect::Value("1 3"),
    },
    Case {
        name: CaseName("a_projection_that_drops_repeats_keeps_the_first_row_of_each_kind"),
        word: Word::Succeeds,
        input: || Scenario::Distinct,
        expect: Expect::Value("1 2"),
    },
    Case {
        name: CaseName("a_projection_that_keeps_repeats_keeps_every_row"),
        word: Word::Succeeds,
        input: || Scenario::Kept,
        expect: Expect::Value("1 2 3"),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
