//! Declared populations are refused or kept exactly, never silently shortened.
use super::super::super::super::service::*;
use super::super::super::super::*;
use super::super::super::scratch_owner::fixtures::{execute, hosted};
use super::super::fixtures::source;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    service::kinds,
    types::{Relation, Value as Cell},
};
use ryo_values::domain::{
    plan_parts::types::{Input as PlanInput, RepeatedRows, StepId},
    plans::types::Operation,
    rows::types::{RowCount, Table},
};
fn malformed_declared_population_is_refused_not_silently_shortened() {
    let table = Table {
        kinds: kinds(Relation::SyntaxTree).to_vec(),
        cells: vec![Cell::Node(ryo_relations::domain::relations::types::NodeId(
            0,
        ))],
        count: RowCount(2),
    };
    assert!(matches!(
        source(Rows::Computed(&table)),
        Err(OperatingError::ShapeMismatch)
    ));
}
fn zero_width_bags_keep_their_declared_population() {
    let (result, _) = execute(
        Operation::Project {
            from: PlanInput::Step(StepId(0)),
            columns: vec![],
            distinct: RepeatedRows::Kept,
        },
        hosted(&[], vec![vec![], vec![], vec![]]),
        &mut Symbols::default(),
    );
    let output = result.unwrap();
    assert_eq!(output.count.0, 3);
    assert!(output.cells.is_empty());
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    MalformedDeclaredPopulationIsRefusedNotSilentlyShortened,
    ZeroWidthBagsKeepTheirDeclaredPopulation,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("malformed_declared_population_is_refused_not_silently_shortened"),
        word: Word::Succeeds,
        input: || Scenario::MalformedDeclaredPopulationIsRefusedNotSilentlyShortened,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("zero_width_bags_keep_their_declared_population"),
        word: Word::Succeeds,
        input: || Scenario::ZeroWidthBagsKeepTheirDeclaredPopulation,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::MalformedDeclaredPopulationIsRefusedNotSilentlyShortened => {
            malformed_declared_population_is_refused_not_silently_shortened();
            Ok(())
        }
        Scenario::ZeroWidthBagsKeepTheirDeclaredPopulation => {
            zero_width_bags_keep_their_declared_population();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
