//! A derived operation consumes a shared source head and never joins its class.
use super::super::super::fixtures::Root;
use super::super::fixtures::plans::Authored;
use super::super::fixtures::semantics::{Derived, Meanings};
use super::super::fixtures::{Population, Program, Residency, demand};
use crate::application::file_dag::program::native_walk::types::RetainedBatches;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use std::sync::Arc;

/// What a grouped rule and its plain sibling over one shared head observed.
#[derive(Debug, PartialEq)]
struct Observed {
    /// Whether equal authored predicates make one class across both rules.
    one_class: bool,
    /// Whether the derived GroupCount after the head is a source-walk class.
    group_count_in_a_class: bool,
    counted_rows: Option<usize>,
    plain_rows: Option<usize>,
    passes: u64,
    source_rows: u64,
    predicate_tests: u64,
    retained: RetainedBatches,
}

/// The source population and its residency.
type Input = (Population, Residency);
type Value = Observed;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("derived_group_count_barrier_follows_a_shared_head_without_a_second_pass"),
    word: Word::Succeeds,
    input: || (Population::Three, Residency::Stable),
    expect: Expect::Value(Observed {
        one_class: true,
        group_count_in_a_class: false,
        // Authored rows 2 and 3 pass the predicate, one group each.
        counted_rows: Some(2),
        plain_rows: Some(2),
        // Both consumers share one source pass.
        passes: 1,
        source_rows: 3,
        predicate_tests: 3,
        // Both consumers delivered, so the batch retired.
        retained: RetainedBatches(0),
    }),
}];

fn rows(root: Root) -> Option<usize> {
    match root {
        Root::Complete { rows, .. } => Some(rows),
        _ => None,
    }
}

/// Runs a grouped rule and a plain sibling over one authored head.
fn subject((population, residency): Input) -> Result<Value, Failure> {
    let program = Program::new(
        Authored::grouped(),
        population,
        residency,
        Arc::new(Meanings::default()),
        Arc::new(Derived::new()),
    )?;
    let grouped = program.head_class(RuleIndex(0))?;
    let one_class = grouped == program.head_class(RuleIndex(1))?;
    let tail = program.prepared.program().rules()[0].original_steps[1]
        .as_ref()
        .map_err(Clone::clone)?
        .logical;
    let region = program.prepared.program().logical_regions()[tail.0];
    let instruction = program.prepared.program().regions()[region.0]
        .instructions()
        .iter()
        .find(|instruction| instruction.logical() == tail)
        .ok_or(OperatingError::NotRun)?;
    let group_count_in_a_class = program.prepared.walk_class(instruction).is_some();
    let mut symbols = Symbols::default();
    let mut file = program.open(&mut symbols)?;
    let read = |relation| program.read(relation);
    let counted = Root::from(file.rule(RuleIndex(0), demand(), &read, &mut symbols)?.emit);
    let plain = Root::from(file.rule(RuleIndex(1), demand(), &read, &mut symbols)?.emit);
    let work = file.walk_counts();
    let retained = file.walk.retained();
    file.finish()?;
    Ok(Observed {
        one_class,
        group_count_in_a_class,
        counted_rows: rows(counted),
        plain_rows: rows(plain),
        passes: work.passes.0,
        source_rows: work.source_rows.0,
        predicate_tests: work.predicate_tests.0,
        retained,
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
