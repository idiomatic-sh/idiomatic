//! Every operation kind is evaluated through its own meaning, in the mode it is scheduled in.
use super::fixtures::{FakeDriver, number, written};
use super::service::evaluate;
use super::types::{EvaluationInput, EvaluationScratch, EvaluationViews};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::source_matches::types::LineBytes;
use ryo_values::domain::plan_parts::types::{ColumnIndex, HostedNumber, Input as PlanInput};
use ryo_values::domain::plans::types::Operation;

/// Which operation runs, and over which input.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// A select of its second column over one scheduled row.
    SelectRow,
    /// A select given populations instead of its scheduled row.
    SelectPopulations,
    /// A hosted relation over its own population.
    HostedPopulation,
    /// A hosted relation given a scheduled row instead of its population.
    HostedRow,
}
type Input = Scenario;
type Value = std::borrow::Cow<'static, str>;
type Failure = PortableOperatorFault;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_select_keeps_its_kept_columns_of_the_scheduled_row"),
        word: Word::Succeeds,
        input: || Scenario::SelectRow,
        expect: Expect::Value(std::borrow::Cow::Borrowed("2")),
    },
    Case {
        name: CaseName("a_hosted_relation_copies_every_row_of_its_population"),
        word: Word::Succeeds,
        input: || Scenario::HostedPopulation,
        expect: Expect::Value(std::borrow::Cow::Borrowed("7|8")),
    },
    Case {
        name: CaseName("a_row_kind_refuses_populations"),
        word: Word::Refuses,
        input: || Scenario::SelectPopulations,
        expect: Expect::Error(PortableOperatorFault::WrongOperation),
    },
    Case {
        name: CaseName("a_population_kind_refuses_a_scheduled_row"),
        word: Word::Refuses,
        input: || Scenario::HostedRow,
        expect: Expect::Error(PortableOperatorFault::WrongOperation),
    },
];
fn subject(scenario: Input) -> Result<Value, Failure> {
    let row = [number(1), number(2)];
    let select = Operation::Select {
        from: PlanInput::Step(ryo_values::domain::plan_parts::types::StepId(0)),
        comparisons: vec![],
        kept: vec![ColumnIndex(1)],
    };
    let hosted = Operation::Hosted {
        relation: HostedNumber(0),
    };
    let (operation, input) = match scenario {
        Scenario::SelectRow => (&select, EvaluationInput::Row(&row)),
        Scenario::SelectPopulations => (&select, EvaluationInput::Populations),
        Scenario::HostedPopulation => (&hosted, EvaluationInput::Populations),
        Scenario::HostedRow => (&hosted, EvaluationInput::Row(&row)),
    };
    let mut driver = FakeDriver {
        population: vec![vec![number(7)], vec![number(8)]],
        ..FakeDriver::default()
    };
    let occurrences = FakeDriver::default();
    let views = EvaluationViews {
        source: LineBytes(b""),
        bindings: &[],
        regex: None,
        checked: None,
        appended: None,
        layers: None,
        cached_reach: None,
        cached_axis: None,
    };
    evaluate(
        operation,
        input,
        views,
        &mut driver,
        &mut EvaluationScratch::default(),
        &occurrences,
    )?;
    Ok(std::borrow::Cow::Owned(written(&driver.emitted)))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
