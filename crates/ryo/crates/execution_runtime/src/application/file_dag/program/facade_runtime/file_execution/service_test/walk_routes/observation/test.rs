//! A normalized named union is not the relation population with the same cell kinds.
use super::super::fixtures::plans::Authored;
use super::super::fixtures::semantics::{Derived, Meanings};
use super::super::fixtures::{Population, Program, Residency, demand};
use ryo_relations::domain::relations::types::Value as Cell;
use super::super::super::fixtures::Root;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use std::borrow::Cow;
use std::{rc::Rc, sync::Arc};

/// What a named-population consumer beside a direct relation consumer observed.
#[derive(Debug, PartialEq)]
struct Observed {
    /// Whether the named consumer reads the resolved observation boundary.
    named_reads_the_observation: bool,
    named_in_a_class: bool,
    direct_in_a_class: bool,
    named: Root,
    /// Whether the typed union consumed the producers' publications.
    union_consumed: bool,
    walked: bool,
    result_slots: u64,
    predicate_tests: u64,
    refused: Option<OperatingError>,
    delivered_before_refusal: Cow<'static, [Cell]>,
    direct: Cow<'static, [Cell]>,
    replays: [Root; 2],
    replay_repeats_no_work: bool,
    working_bytes: usize,
}

/// The source population.
type Input = Population;
type Value = Observed;
type Failure = OperatingError;

const fn n(value: i64) -> Cell {
    Cell::Number(ryo_relations::domain::relations::types::Integer(value))
}

const NAMED: &[Cell] = &[n(1), n(3)];

// Expected values come from the authored producer predicates and facts, never from prepared
// classes, dispatched events or emitted census totals.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("named_population_barrier_cannot_reuse_a_direct_relation_class_or_erase_replay"),
    word: Word::Succeeds,
    input: || Population::Three,
    expect: Expect::Value(Observed {
        named_reads_the_observation: true,
        named_in_a_class: false,
        direct_in_a_class: true,
        named: Root::complete(NAMED, 2),
        union_consumed: true,
        walked: true,
        // Only the three direct source predicates own class results; each reached distinct
        // comparison is decided once per source row.
        result_slots: 3,
        predicate_tests: 9,
        refused: Some(OperatingError::NotRead),
        delivered_before_refusal: Cow::Borrowed(&[n(1)]),
        // The direct population includes the row absent from the named union.
        direct: Cow::Borrowed(&[n(1), n(2), n(3)]),
        // The named result replays independently of the direct callback refusal.
        replays: [Root::complete(NAMED, 2), Root::complete(NAMED, 2)],
        replay_repeats_no_work: true,
        working_bytes: 0,
    }),
}];

/// Runs a named union consumer (rule 2) and a direct relation consumer (rule 3).
fn subject(population: Input) -> Result<Value, Failure> {
    let meanings = Arc::new(Meanings::default());
    let derived = Arc::new(Derived::new());
    let program = Program::new(
        Authored::named(),
        population,
        Residency::Stable,
        meanings.clone(),
        derived.clone(),
    )?;
    let root = *program.prepared.program().rules()[2]
        .emit
        .as_ref()
        .map_err(Clone::clone)?;
    let region = &program.prepared.program().regions()[root.0];
    let named_reads_the_observation = matches!(
        region.inputs().first(),
        Some(ryo_execution_domain::domain::executable_program::types::InputSource::Observation(_))
    );
    let named_in_a_class = program
        .prepared
        .walk_class(&region.instructions()[0])
        .is_some();
    let direct = *program.prepared.program().rules()[3]
        .emit
        .as_ref()
        .map_err(Clone::clone)?;
    let direct_in_a_class = program
        .prepared
        .walk_class(&program.prepared.program().regions()[direct.0].instructions()[0])
        .is_some();
    let mut symbols = Symbols::default();
    let mut file = program.open(&mut symbols)?;
    let read = |relation| program.read(relation);
    let named = Root::from(file.rule(RuleIndex(2), demand(), &read, &mut symbols)?.emit);
    let semantic = meanings.snapshot();
    let work = file.walk_counts();
    let builds = derived.snapshot();
    let mut refused_prefix = vec![];
    let refused = file
        .emit_rows(RuleIndex(3), &read, &mut symbols, &mut |row| {
            refused_prefix.extend_from_slice(row);
            Err(OperatingError::NotRead)
        })
        .err();
    let mut direct_rows = vec![];
    file.emit_rows(RuleIndex(3), &read, &mut symbols, &mut |row| {
        direct_rows.extend_from_slice(row);
        Ok(())
    })?;
    // A direct StoredSelection may reborrow its certified provider rows at delivery. That is
    // not another predicate pass; only the computed named answer below is required to replay
    // without another provider lookup.
    let reads = program.reads();
    let first = Root::from(file.rule(RuleIndex(2), demand(), &read, &mut symbols)?.emit);
    let second = Root::from(file.rule(RuleIndex(2), demand(), &read, &mut symbols)?.emit);
    let replay_repeats_no_work = file.walk_counts() == work
        && meanings.snapshot() == semantic
        && program.reads() == reads
        && derived.snapshot() == builds;
    let owner = Rc::clone(&file.owner);
    file.finish()?;
    Ok(Observed {
        named_reads_the_observation,
        named_in_a_class,
        direct_in_a_class,
        named,
        union_consumed: semantic.unions.0 > 0,
        walked: work.passes.0 > 0,
        result_slots: work.result_slots.0,
        predicate_tests: work.predicate_tests.0,
        refused,
        delivered_before_refusal: Cow::Owned(refused_prefix),
        direct: Cow::Owned(direct_rows),
        replays: [first, second],
        replay_repeats_no_work,
        working_bytes: owner.derived().working_bytes().0,
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
