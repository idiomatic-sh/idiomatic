//! The `widths` service's cases: the width it counts for every kind of operation is the width the schema algebra answers.
use super::service::{width_after, width_of};
use super::types::StepWidths;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::operation_schema::fixtures::{bindings, operations, tables};
use crate::domain::operation_schema::service::schema_of;
use crate::domain::operation_schema::types::InputShape;
use ryo_relations::domain::relations::types::Relation;
use ryo_relations::domain::terms::types::Symbol;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as Source, RowWidth, StepId, StepScope,
};
use ryo_values::domain::plans::service::kind_of;
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::query_plans::types::{Emit, Plan};

/// What a row asks of the service.
#[derive(Clone, Copy)]
enum Ask {
    /// The width of every fixture operation, beside its schema's.
    Schemas,
    /// How many steps a count of the width of the last step of a ladder of so many unions, each of the step before with itself,
    /// counts, and of one of twice as many.
    Ladder(usize),
}

/// What a row answers.
#[derive(Debug, PartialEq, Eq)]
enum Answer {
    /// How many fixture operations the width was counted for and agreed with the schema, and the kinds it could not be counted for.
    Agreed(usize, std::borrow::Cow<'static, [OperationKind]>),
    /// How many steps of a ladder at a depth and at twice it the count of its last step's width counted, beside how many it has.
    Counted(std::borrow::Cow<'static, str>),
}

type Input = Ask;
type Value = Answer;
/// The first kind whose counted width is not its schema's, with what the schema refused it for when it did.
type Failure = (
    OperationKind,
    Option<crate::domain::operating::errors::OperatingError>,
);

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_kind_but_the_hosts_rows_is_as_wide_as_its_schema"),
        word: Word::Succeeds,
        input: || Ask::Schemas,
        expect: Expect::Value(Answer::Agreed(
            30,
            std::borrow::Cow::Borrowed(&[OperationKind::Hosted]),
        )),
    },
    Case {
        name: CaseName("the_widths_under_a_ladder_twice_as_deep_are_counted_in_twice_the_steps"),
        word: Word::Succeeds,
        input: || Ask::Ladder(16),
        expect: Expect::Value(Answer::Counted(std::borrow::Cow::Borrowed(
            "16 links: 17 of 17 steps counted, the last Some(9) columns wide; 32 links: 33 of 33 steps counted, the last Some(9) columns wide",
        ))),
    },
];

/// How many steps a count of the width of the last step of the binding events kept whole, then `levels` unions, each of the step
/// before with itself, counts, beside how many steps it has.
fn ladder_counted(levels: usize) -> String {
    let mut steps = vec![PlanStep {
        written: Default::default(),
        operation: Operation::Select {
            from: Source::Relation(Relation::BindingEvent),
            comparisons: Vec::new(),
            kept: Vec::new(),
        },
        scope: StepScope::PerFile,
    }];
    for at in 0..levels {
        let before = Source::Step(StepId(u32::try_from(at).unwrap_or(u32::MAX)));
        steps.push(PlanStep {
            written: Default::default(),
            operation: Operation::Union {
                left: before,
                right: before,
            },
            scope: StepScope::PerFile,
        });
    }
    let last = StepId(u32::try_from(levels).unwrap_or(u32::MAX));
    let plan = Plan {
        observation_inputs: Vec::new(),
        parameters: Vec::new(),
        steps,
        comparison_origins: Vec::new(),
        observations: Vec::new(),
        emit: Emit {
            step: last,
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        frontier: Vec::new(),
    };
    let mut widths = StepWidths::default();
    let width = width_of(&plan, &plan.steps, Source::Step(last), &mut widths);
    format!(
        "{levels} links: {} of {} steps counted, the last {:?} columns wide",
        widths.counted.0,
        plan.steps.len(),
        width.map(|counted| counted.0),
    )
}

fn subject(ask: Input) -> Result<Value, Failure> {
    if let Ask::Ladder(levels) = ask {
        return Ok(Answer::Counted(std::borrow::Cow::Owned(format!(
            "{}; {}",
            ladder_counted(levels),
            ladder_counted(levels.saturating_mul(2))
        ))));
    }
    let mut agreed = 0;
    let mut uncounted = Vec::new();
    for operation in operations() {
        let inputs = tables(&operation, Symbol(0), true);
        let shapes: Vec<InputShape<'_>> = inputs
            .iter()
            .map(|table| InputShape {
                kinds: &table.kinds,
                tree: None,
            })
            .collect();
        let mut widths = [None; 3];
        for (width, table) in widths.iter_mut().zip(&inputs) {
            *width = Some(RowWidth(table.kinds.len()));
        }
        // The host's rows are an input of the schema algebra and no input of the plan: the plan does not say their width.
        if kind_of(&operation) == OperationKind::Hosted {
            widths = [None; 3];
        }
        let kind = kind_of(&operation);
        let schema = schema_of(&operation, &shapes, &bindings())
            .map_err(|error| (kind, Some(error)))?
            .len();
        match width_after(&operation, widths) {
            Some(counted) if counted == RowWidth(schema) => agreed += 1,
            Some(_) => return Err((kind, None)),
            None => uncounted.push(kind),
        }
    }
    Ok(Answer::Agreed(agreed, uncounted.into()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
