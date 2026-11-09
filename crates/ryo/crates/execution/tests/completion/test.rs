//! Blocked regions, empty publications and merged coverage, through the public file session.
use crate::file_intents::{answered, partial};
use crate::boundary_fixture::{authored_request, engine, source};
use ryo_execution::inbound::use_cases::execution::interface::{PreparedExecutionProgram, RootOutcome};
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase as _;
use ryo_relations::domain::relations::fixtures::kinds;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, StepId, StepScope};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::rows::types::{RowCount, Table};

/// Function's rows without those whose node is a Comment's, by node.
fn functions_without_comments() -> PreparedExecutionProgram {
    let mut input = authored_request(1);
    let select = |relation| PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Select {
            from: Input::Relation(relation),
            comparisons: vec![],
            kept: vec![ColumnIndex(0)],
        },
    };
    crate::boundary_fixture::plans(&mut input)[0].steps = vec![
        select(Relation::Function),
        select(Relation::Comment),
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Antijoin {
                left: Input::Step(StepId(0)),
                right: Input::Step(StepId(1)),
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
            },
        },
    ];
    crate::boundary_fixture::plans(&mut input)[0].emit.step = StepId(2);
    engine().prepare(input).expect("prepare an antijoin")
}

fn empty(relation: Relation) -> Table {
    Table {
        kinds: kinds(relation).to_vec(),
        count: RowCount(0),
        cells: vec![],
    }
}

/// S4: a region whose rule's evaluation reads a relation in part in the file is blocked before it runs: it publishes the empty
/// table, partial in that file, and reads nothing.
#[test]
fn a_region_reading_a_partial_relation_is_blocked_before_it_reads() {
    let program = functions_without_comments();
    let rows = [
        (Relation::Function, source()),
        (Relation::Comment, empty(Relation::Comment)),
    ];
    let (outcome, reads) = answered(&program, &rows, &[], &[]);
    let Ok(RootOutcome::Complete(whole)) = outcome else {
        panic!("with nothing read in part the antijoin completes")
    };
    assert_eq!(whole.count, RowCount(1), "the function no comment covers");
    let partial = [partial(Relation::Comment)];
    let (outcome, reads_blocked) = answered(&program, &rows, &partial, &partial);
    let Ok(RootOutcome::Partial { table, affected }) = outcome else {
        panic!("a blocked region publishes a partial answer")
    };
    assert_eq!(table.count, RowCount(0), "a blocked region publishes the empty table");
    assert_eq!(affected, [Some(ryo_relations::domain::relations::types::FileId(0))]);
    assert_eq!(reads_blocked, 0, "a blocked region reads no relation");
    assert!(reads > 0);
}

/// The count of Function's rows, read directly, without those whose node is a Comment's: the count reads no relation itself, so
/// what it knows of the reads comes from the antijoin's publication.
fn counted_relations_without_comments() -> PreparedExecutionProgram {
    let mut input = authored_request(1);
    crate::boundary_fixture::plans(&mut input)[0].steps = vec![
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Antijoin {
                left: Input::Relation(Relation::Function),
                right: Input::Relation(Relation::Comment),
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::GroupCount {
                from: Input::Step(StepId(0)),
                keys: vec![],
                distinct: None,
            },
        },
    ];
    crate::boundary_fixture::plans(&mut input)[0].emit.step = StepId(1);
    engine().prepare(input).expect("prepare a count of an antijoin of two relations")
}

/// S3: an empty input settles a region's answer before its kernel and before its other inputs are read; the empty table it
/// publishes keeps the coverage of the region's reads: partial in the file whose relation was read in part.
#[test]
fn an_empty_publication_keeps_the_partial_coverage_of_its_reads() {
    let program = counted_relations_without_comments();
    let rows = [
        (Relation::Function, empty(Relation::Function)),
        (Relation::Comment, empty(Relation::Comment)),
    ];
    let (outcome, reads) = answered(&program, &rows, &[], &[]);
    assert!(matches!(outcome, Ok(RootOutcome::Complete(_))));
    assert_eq!(
        reads, 2,
        "the empty left input settles the antijoin before its right input is read"
    );
    for relation in [Relation::Function, Relation::Comment] {
        let (outcome, _) = answered(&program, &rows, &[], &[partial(relation)]);
        let Ok(RootOutcome::Partial { table, affected }) = outcome else {
            panic!("an empty answer read in part is partial: {relation:?}")
        };
        assert_eq!(table.count, RowCount(0), "an empty count has no group");
        assert_eq!(affected, [Some(ryo_relations::domain::relations::types::FileId(0))]);
    }
}
