//! Which regions get a distinct-selection alternative, and when it declines.
use super::super::super::*;
use super::super::fixtures::plans::Authored;
use super::super::fixtures::semantics::{BuildPolicy, Calls, Derived, Meanings};
use super::super::fixtures::{Population, Program, Residency, demand};

struct Built {
    program: Program,
    derived: Arc<Derived>,
}
fn program(authored: Authored, population: Population) -> Built {
    let derived = Arc::new(Derived::new(BuildPolicy::Normal));
    let program = Program::new(
        authored,
        population,
        Residency::Stable,
        Arc::new(Meanings::default()),
        derived.clone(),
    )
    .unwrap();
    Built { program, derived }
}
#[derive(Debug, PartialEq, Eq)]
enum Selection {
    Offered,
    Absent,
}
fn tail_selection(program: &Program, rule: RuleIndex) -> Selection {
    let tail = program.prepared.program().rules()[rule.0].original_steps[1]
        .as_ref()
        .unwrap()
        .logical;
    let region = program.prepared.program().logical_regions()[tail.0];
    if crate::domain::executable_program::service::selection(
        &program.prepared.program().regions()[region.0].route(),
    )
    .is_some()
    {
        Selection::Offered
    } else {
        Selection::Absent
    }
}

struct Answer(RowCount, Calls);
fn answer(population: Population) -> Answer {
    let built = program(Authored::distinct(), population);
    let mut symbols = Symbols::default();
    let mut file = built.program.open(&mut symbols).unwrap();
    let read = |relation| built.program.read(relation);
    let RootOutcome::Complete(answer) = file
        .rule(RuleIndex(0), demand(), &read, &mut symbols)
        .unwrap()
        .emit
    else {
        panic!("the distinct root must complete")
    };
    file.finish().unwrap();
    Answer(answer.count, built.derived.distinct_calls())
}

#[test]
fn a_distinct_project_region_gets_a_selection_alternative() {
    let program = program(Authored::distinct(), Population::Three).program;
    let root = *program.prepared.program().rules()[0].emit.as_ref().unwrap();
    assert!(
        crate::domain::executable_program::service::selection(
            &program.prepared.program().regions()[root.0].route()
        )
        .is_some()
    );
}

#[test]
fn derived_and_extending_regions_never_get_a_selection_alternative() {
    let grouped = program(Authored::grouped(), Population::Three).program;
    assert_eq!(
        tail_selection(&grouped, RuleIndex(0)),
        Selection::Absent,
        "GroupCount is not a selection"
    );
    let extended = program(Authored::continuations(), Population::Three).program;
    assert_eq!(
        tail_selection(&extended, RuleIndex(1)),
        Selection::Absent,
        "Extend is not a selection"
    );
}

#[test]
fn a_nonempty_primary_builds_the_distinct_payload_and_an_empty_one_declines() {
    let nonempty = answer(Population::Three);
    assert_eq!(nonempty.0, RowCount(3));
    assert!(
        nonempty.1.0 > 0,
        "a nonempty primary acquires a distinct payload"
    );
    let empty = answer(Population::Empty);
    assert_eq!(empty.0, RowCount(0));
    assert_eq!(
        empty.1,
        Calls(0),
        "an empty primary has no representatives: the ordinary route answers it"
    );
}
