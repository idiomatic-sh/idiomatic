//! Per-row and request-owned routes beside a shared head: barrier, classification, isolation.
use super::super::super::*;
use super::super::fixtures::plans::Authored;
use super::super::fixtures::routes::RowRoute;
use super::super::fixtures::semantics::{Derived, Meanings};
use super::super::fixtures::{Population, Program, Residency, cells, demand};
use crate::domain::executable_program::fusion::types::{Fusibility, Ineligible};
use ryo_values::domain::plans::service::kind_of;

fn describe(attempt: Result<String, OperatingError>) -> String {
    attempt.unwrap_or_else(|error| format!("refused {error:?}"))
}
fn view(root: RootOutcome) -> String {
    match root {
        RootOutcome::Complete(table) => format!("Complete({:?}, {})", table.cells, table.count.0),
        RootOutcome::Failed(error) => format!("Failed({error:?})"),
        _ => "Other".to_owned(),
    }
}

fn exercise(route: RowRoute) -> Result<(), OperatingError> {
    let meanings = Arc::new(Meanings::default());
    let derived = Arc::new(Derived::new());
    let program = Program::new(
        Authored::row_route(route),
        Population::Three,
        Residency::Stable,
        meanings,
        derived,
    )?;
    let compiled = program.prepared.program();
    // The plain head over the function relation is a member of a shared class.
    program.head_class(RuleIndex(0))?;
    // The routed operation is its own region head, never a class member.
    let logical = compiled.rules()[1].original_steps[0]
        .as_ref()
        .map_err(Clone::clone)?
        .logical;
    let region = compiled.logical_regions()[logical.0];
    let instruction = compiled.regions()[region.0]
        .instructions()
        .first()
        .ok_or(OperatingError::NotRun)?;
    assert_eq!(kind_of(&instruction.operation()), route.kind(), "{route:?}");
    assert!(
        program.prepared.walk_class(instruction).is_none(),
        "{route:?} is a barrier of the source walk, not a class member"
    );
    let expected = match route {
        RowRoute::Rows | RowRoute::Hosted => Ineligible::Source,
        _ => Ineligible::RowFault,
    };
    assert_eq!(
        compiled.fusibility(logical),
        Some(Fusibility::Excluded(expected)),
        "{route:?}"
    );
    assert_eq!(
        compiled.regions()[region.0].instructions().len(),
        1,
        "{route:?}"
    );

    let mut symbols = Symbols::default();
    let mut file = program.open(&mut symbols)?;
    let read = |relation| program.read(relation);
    let head = |file: &mut FileRun, symbols: &mut Symbols| {
        file.rule(RuleIndex(0), demand(), &read, symbols)
            .map(|outcome| view(outcome.emit))
    };
    let routed = |file: &mut FileRun, symbols: &mut Symbols| {
        file.rule(RuleIndex(1), demand(), &read, symbols)
            .map(|outcome| view(outcome.emit))
    };
    let first = describe(head(&mut file, &mut symbols));
    assert!(
        first.starts_with("Complete"),
        "the shared head answers its three rows: {first}"
    );
    let counts = file.walk_counts();
    let answer = describe(routed(&mut file, &mut symbols));
    match route {
        RowRoute::Hosted => assert_eq!(
            answer,
            format!("Complete({:?}, 2)", cells(&[5, 6])),
            "the host relation's rows"
        ),
        RowRoute::Rows => assert!(answer.starts_with("Complete"), "{answer}"),
        _ => {}
    }
    assert_eq!(
        file.walk_counts(),
        counts,
        "{route:?} neither joins nor repeats a source pass"
    );
    assert_eq!(
        describe(routed(&mut file, &mut symbols)),
        answer,
        "{route:?} retry"
    );
    assert_eq!(
        describe(head(&mut file, &mut symbols)),
        first,
        "{route:?} cannot poison its healthy shared-class sibling"
    );
    file.finish()?;
    Ok(())
}

#[test]
fn per_row_and_request_owned_routes_are_barriers_beside_a_shared_head() {
    for route in RowRoute::ALL {
        exercise(route).unwrap_or_else(|error| panic!("{route:?}: {error:?}"));
    }
}
