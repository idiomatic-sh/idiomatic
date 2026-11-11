//! A normalized named union is not the relation population with the same cell kinds.
use super::super::super::super::service::*;
use super::super::super::super::*;
use super::super::fixtures::plans::Authored;
use super::super::fixtures::semantics::{Derived, Meanings};
use super::super::fixtures::{Population, Program, Residency, cells, demand};
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::rows::types::RowCount;
use std::{rc::Rc, sync::Arc};

fn named_population_barrier_cannot_reuse_a_direct_relation_class_or_erase_replay() {
    let meanings = Arc::new(Meanings::default());
    let derived = Arc::new(Derived::new());
    let program = Program::new(
        Authored::named(),
        Population::Three,
        Residency::Stable,
        meanings.clone(),
        derived.clone(),
    )
    .unwrap();
    let root = *program.prepared.program().rules()[2].emit.as_ref().unwrap();
    let region = &program.prepared.program().regions()[root.0];
    assert!(
        matches!(
            region.inputs().first(),
            Some(
                ryo_execution_domain::domain::executable_program::types::InputSource::Observation(
                    _
                )
            )
        ),
        "this consumer must have the real resolved observation boundary"
    );
    assert!(
        program
            .prepared
            .walk_class(&region.instructions()[0])
            .is_none()
    );
    let direct = *program.prepared.program().rules()[3].emit.as_ref().unwrap();
    assert!(
        program
            .prepared
            .walk_class(&program.prepared.program().regions()[direct.0].instructions()[0])
            .is_some()
    );
    let mut symbols = Symbols::default();
    let mut file = program.open(&mut symbols).unwrap();
    let read = |relation| program.read(relation);
    let RootOutcome::Complete(named) = file
        .rule(RuleIndex(2), demand(), &read, &mut symbols)
        .unwrap()
        .emit
    else {
        panic!("actual current named union must feed the dependent consumer")
    };
    // Expected values come from the authored producer predicates and facts,
    // never from prepared classes, dispatched events or emitted census totals.
    assert_eq!(named.cells, cells(&[1, 3]));
    assert_eq!(named.count, RowCount(2));
    let semantic = meanings.snapshot();
    assert!(
        semantic.unions.0 > 0,
        "the typed union must consume both actual producer publications"
    );
    let work = file.walk_counts();
    assert!(work.passes.0 > 0);
    assert_eq!(
        work.result_slots.0, 3,
        "only the three direct source predicates own class results"
    );
    assert_eq!(
        work.predicate_tests.0, 9,
        "each reached distinct comparison is decided once per source row"
    );
    let builds = derived.snapshot();
    let mut refused_prefix = vec![];
    assert!(matches!(
        file.emit_rows(RuleIndex(3), &read, &mut symbols, &mut |row| {
            refused_prefix.extend_from_slice(row);
            Err(OperatingError::NotRead)
        }),
        Err(OperatingError::NotRead)
    ));
    assert_eq!(refused_prefix, cells(&[1]));
    let mut direct = vec![];
    file.emit_rows(RuleIndex(3), &read, &mut symbols, &mut |row| {
        direct.extend_from_slice(row);
        Ok(())
    })
    .unwrap();
    assert_eq!(
        direct,
        cells(&[1, 2, 3]),
        "the direct population includes the row absent from the named union"
    );
    // A direct StoredSelection may reborrow its certified provider rows at
    // delivery. That is not another predicate pass; only the computed named
    // answer below is required to replay without another provider lookup.
    let reads = program.reads();
    for _ in 0..2 {
        let RootOutcome::Complete(replay) = file
            .rule(RuleIndex(2), demand(), &read, &mut symbols)
            .unwrap()
            .emit
        else {
            panic!("named result must replay independently of direct callback refusal")
        };
        assert_eq!(replay, named);
    }
    assert_eq!(file.walk_counts(), work);
    assert_eq!(meanings.snapshot(), semantic);
    assert_eq!(program.reads(), reads);
    assert_eq!(derived.snapshot(), builds);
    let owner = Rc::clone(&file.owner);
    file.finish().unwrap();
    assert_eq!(owner.derived().working_bytes(), 0);
    assert_eq!(named.cells, cells(&[1, 3]));
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    NamedPopulationBarrierCannotReuseADirectRelationClassOrEraseReplay,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("named_population_barrier_cannot_reuse_a_direct_relation_class_or_erase_replay"),
    word: Word::Succeeds,
    input: || Scenario::NamedPopulationBarrierCannotReuseADirectRelationClassOrEraseReplay,
    expect: Expect::Value(()),
}];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::NamedPopulationBarrierCannotReuseADirectRelationClassOrEraseReplay => {
            named_population_barrier_cannot_reuse_a_direct_relation_class_or_erase_replay();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
