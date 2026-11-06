//! A source class is not a continuation result, and its consumers own immutable answers.
use super::super::super::*;
use super::super::fixtures::plans::Authored;
use super::super::fixtures::semantics::{BuildPolicy, Calls, Derived, Meanings};
use super::super::fixtures::{Population, Program, Residency, cells, demand};

fn exercise(residency: Residency) -> Result<(), OperatingError> {
    let meanings = Arc::new(Meanings::default());
    let derived = Arc::new(Derived::new(BuildPolicy::Normal));
    let program = Program::new(
        Authored::continuations(),
        Population::Three,
        residency,
        meanings.clone(),
        derived.clone(),
    )?;
    // Different authored projections keep the two source heads physically
    // distinct while their predicate decisions belong to one common class.
    for rule in [RuleIndex(0), RuleIndex(1)] {
        let mapping = &program.prepared.program().rules()[rule.0].original_steps;
        let head = mapping[0].as_ref().unwrap().logical;
        let tail = mapping[1].as_ref().unwrap().logical;
        let head_region = program.prepared.program().logical_regions()[head.0];
        let region = program.prepared.program().logical_regions()[tail.0];
        assert_ne!(
            head_region, region,
            "an Extend is not a fusable row continuation: it owns the region the fault crosses"
        );
        let instructions = &program.prepared.program().regions()[region.0].instructions;
        assert_eq!(instructions.len(), 1);
        let continuation = instructions
            .iter()
            .find(|instruction| instruction.logical == tail)
            .unwrap();
        assert!(
            program.prepared.walk_class(continuation).is_none(),
            "an Extend continuation is never a source-walk class"
        );
    }
    let plain = program.prepared.program().rules()[3].emit.as_ref().unwrap();
    assert!(
        program.prepared.program().regions()[plain.0]
            .selection
            .is_some(),
        "the singleton consumer must reach the real stored-selection alternative"
    );
    let mut symbols = Symbols::default();
    let mut file = program.open(&mut symbols)?;
    let read = |relation| program.read(relation);
    for _ in 0..2 {
        let failed = file.rule(RuleIndex(0), demand(), &read, &mut symbols)?;
        assert!(
            matches!(
                failed.emit,
                RootOutcome::Failed(OperatingError::WrongOperation)
            ),
            "the reached Append semantic error is terminal and cannot become a declined source route"
        );
    }
    let RootOutcome::Complete(transferred) =
        file.rule(RuleIndex(1), demand(), &read, &mut symbols)?.emit
    else {
        panic!("a failed continuation cannot poison its healthy source-class sibling")
    };
    assert_eq!(transferred.cells, cells(&[9, 7, 7, 7]));
    assert_eq!(transferred.count, RowCount(2));
    let RootOutcome::Complete(selected) =
        file.rule(RuleIndex(3), demand(), &read, &mut symbols)?.emit
    else {
        panic!("the same class must also serve the selected-publication consumer")
    };
    assert_eq!(
        selected.cells,
        cells(&[2, 2, 3, 3]),
        "authored repeated kept columns remain repeated"
    );
    let semantic = meanings.snapshot();
    assert_eq!(
        semantic.failed,
        Calls(1),
        "failure must stop at its first actual continuation row"
    );
    assert_eq!(
        semantic.appended,
        Calls(4),
        "each healthy Extend rule appends its two rows once; retries never recompute them"
    );
    let work = file.walk_counts();
    let populations = match residency {
        Residency::Stable => 1,
        Residency::Unstable => 3,
    };
    assert_eq!(
        work.passes.0, populations,
        "only certified physical populations may share a pass"
    );
    assert_eq!(work.source_rows.0, 3 * populations);
    assert_eq!(work.predicate_tests.0, 3 * populations);
    assert_eq!(work.result_slots.0, populations);
    let reads = program.reads();
    let builds = derived.snapshot();
    let mut prefix = vec![];
    let refused = file.emit_rows(RuleIndex(2), &read, &mut symbols, &mut |row| {
        prefix.extend_from_slice(row);
        Err(OperatingError::NotRead)
    });
    assert!(matches!(refused, Err(OperatingError::NotRead)));
    assert_eq!(prefix, cells(&[9, 7]));
    let mut replay = vec![];
    file.emit_rows(RuleIndex(2), &read, &mut symbols, &mut |row| {
        replay.extend_from_slice(row);
        Ok(())
    })?;
    assert_eq!(
        replay, transferred.cells,
        "callback retry replays the full committed answer"
    );
    let RootOutcome::Complete(repeated) =
        file.rule(RuleIndex(1), demand(), &read, &mut symbols)?.emit
    else {
        panic!("collected retry must keep its terminal answer")
    };
    assert_eq!(repeated, transferred);
    assert_eq!(meanings.snapshot(), semantic);
    assert_eq!(file.walk_counts(), work);
    assert_eq!(program.reads(), reads);
    assert_eq!(derived.snapshot(), builds);
    let owner = Rc::clone(&file.owner);
    file.finish()?;
    assert_eq!(owner.derived.working_bytes(), 0);
    assert_eq!(
        transferred.cells, replay,
        "transferred output survives file retirement"
    );
    Ok(())
}
#[test]
fn shared_source_continuation_failure_and_result_consumers_preserve_one_certified_population() {
    exercise(Residency::Stable)
        .expect("actual shared class, continuations and selected/replayed consumers");
}
#[test]
fn uncertified_resident_identity_keeps_equal_source_bytes_in_distinct_populations() {
    exercise(Residency::Unstable).expect("real unstable source barrier");
}
