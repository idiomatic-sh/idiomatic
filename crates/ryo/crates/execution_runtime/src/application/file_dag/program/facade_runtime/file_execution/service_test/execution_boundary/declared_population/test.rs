//! Declared populations are refused or kept exactly, never silently shortened.
use super::super::super::super::*;
use super::super::super::scratch_owner::fixtures::{execute, hosted};
use super::super::fixtures::source;
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
#[test]
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
#[test]
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
