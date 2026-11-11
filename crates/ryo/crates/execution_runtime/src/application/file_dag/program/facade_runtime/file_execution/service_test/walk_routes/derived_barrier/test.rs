//! A derived operation consumes a shared source head and never joins its class.
use super::super::super::super::*;
use super::super::fixtures::plans::Authored;
use super::super::fixtures::semantics::{Derived, Meanings};
use super::super::fixtures::{Population, Program, Residency, demand};
use crate::application::file_dag::program::native_walk::RetainedBatches;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::rows::types::RowCount;
use std::sync::Arc;

#[test]
fn derived_group_count_barrier_follows_a_shared_head_without_a_second_pass() {
    let program = Program::new(
        Authored::grouped(),
        Population::Three,
        Residency::Stable,
        Arc::new(Meanings::default()),
        Arc::new(Derived::new()),
    )
    .unwrap();
    let grouped = program
        .head_class(RuleIndex(0))
        .expect("the authored head of the grouped rule is a shared source class");
    assert_eq!(
        grouped,
        program.head_class(RuleIndex(1)).unwrap(),
        "equal authored predicates make one class across both rules"
    );
    let tail = program.prepared.program().rules()[0].original_steps[1]
        .as_ref()
        .unwrap()
        .logical;
    let region = program.prepared.program().logical_regions()[tail.0];
    let instruction = program.prepared.program().regions()[region.0]
        .instructions()
        .iter()
        .find(|instruction| instruction.logical() == tail)
        .unwrap();
    assert!(
        program.prepared.walk_class(instruction).is_none(),
        "a derived GroupCount is a barrier, never a source-walk class"
    );
    let mut symbols = Symbols::default();
    let mut file = program.open(&mut symbols).unwrap();
    let read = |relation| program.read(relation);
    let RootOutcome::Complete(counted) = file
        .rule(RuleIndex(0), demand(), &read, &mut symbols)
        .unwrap()
        .emit
    else {
        panic!("the derived consumer must complete over the shared head")
    };
    assert_eq!(
        counted.count,
        RowCount(2),
        "authored rows 2 and 3 pass the predicate, one group each"
    );
    let RootOutcome::Complete(plain) = file
        .rule(RuleIndex(1), demand(), &read, &mut symbols)
        .unwrap()
        .emit
    else {
        panic!("the plain sibling must complete from the same class")
    };
    assert_eq!(plain.count, RowCount(2));
    let work = file.walk_counts();
    assert_eq!(work.passes.0, 1, "both consumers share one source pass");
    assert_eq!(work.source_rows.0, 3);
    assert_eq!(work.predicate_tests.0, 3);
    assert_eq!(
        file.walk.retained(),
        RetainedBatches(0),
        "both consumers delivered, so the batch retired"
    );
    file.finish().unwrap();
}
