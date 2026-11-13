//! The support of the `continuation` cases.
use super::super::super::fixtures::Root;
use super::super::fixtures::plans::Authored;
use super::super::fixtures::semantics::{Calls, Derived, Meanings};
use super::super::fixtures::{Population, Program, Residency, demand};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::Value as Cell;
use std::borrow::Cow;
use std::{rc::Rc, sync::Arc};

/// What the source walk did: passes, source rows, predicate tests and result slots.
#[derive(Debug, PartialEq)]
pub(super) struct Walk {
    pub(super) passes: u64,
    pub(super) source_rows: u64,
    pub(super) predicate_tests: u64,
    pub(super) result_slots: u64,
}

/// What a shared source class with failing, transferred and selected consumers observed.
#[derive(Debug, PartialEq)]
pub(super) struct Observed {
    /// Of rules 0 and 1, those whose Extend continuation owns a one-instruction region apart
    /// from its head and outside any source-walk class.
    pub(super) continuations_apart: usize,
    pub(super) equivalent_extends_share_a_region: bool,
    pub(super) other_literal_has_its_own_region: bool,
    pub(super) failed: [Root; 2],
    pub(super) transferred: Root,
    pub(super) selected: Root,
    pub(super) failed_calls: Calls,
    pub(super) appended_calls: Calls,
    pub(super) walk: Walk,
    pub(super) refused: Option<OperatingError>,
    pub(super) delivered_before_refusal: Cow<'static, [Cell]>,
    pub(super) replay_is_the_answer: bool,
    pub(super) collected_retry_is_the_answer: bool,
    pub(super) retries_repeat_no_work: bool,
    pub(super) working_bytes: usize,
    pub(super) answer_survives_retirement: bool,
}

/// Runs a shared source head over three rows with an Extend that fails (rule 0), equivalent
/// healthy Extends (rules 1 and 2) and a selected publication (rule 3).
pub(super) fn exercise(residency: Residency) -> Result<Observed, OperatingError> {
    let meanings = Arc::new(Meanings::default());
    let derived = Arc::new(Derived::new());
    let program = Program::new(
        Authored::continuations(),
        Population::Three,
        residency,
        meanings.clone(),
        derived.clone(),
    )?;
    // Different authored projections keep the two source heads physically distinct while their
    // predicate decisions belong to one common class. An Extend is not a fusable row
    // continuation: it owns the region the fault crosses, and is never a source-walk class.
    let mut continuations_apart = 0;
    for rule in [RuleIndex(0), RuleIndex(1)] {
        let mapping = &program.prepared.program().rules()[rule.0].original_steps;
        let head = mapping[0].as_ref().map_err(Clone::clone)?.logical;
        let tail = mapping[1].as_ref().map_err(Clone::clone)?.logical;
        let head_region = program.prepared.program().logical_regions()[head.0];
        let region = program.prepared.program().logical_regions()[tail.0];
        let instructions = &program.prepared.program().regions()[region.0].instructions();
        let continuation = instructions
            .iter()
            .find(|instruction| instruction.logical() == tail)
            .ok_or(OperatingError::NotRun)?;
        continuations_apart += usize::from(
            head_region != region
                && instructions.len() == 1
                && program.prepared.walk_class(continuation).is_none(),
        );
    }
    // Rules 1 and 2 author the same Extend over the same head: one region serves both; an
    // Extend of another literal is a different region.
    let rules = program.prepared.program().rules();
    let emit = |rule: usize| rules[rule].emit.as_ref().map_err(Clone::clone);
    let equivalent_extends_share_a_region = emit(1)? == emit(2)?;
    let other_literal_has_its_own_region = emit(0)? != emit(1)?;
    let mut symbols = Symbols::default();
    let mut file = program.open(&mut symbols)?;
    let read = |relation| program.read(relation);
    // The reached Append semantic error is terminal and cannot become a declined source route.
    let first_failure = Root::from(file.rule(RuleIndex(0), demand(), &read, &mut symbols)?.emit);
    let second_failure = Root::from(file.rule(RuleIndex(0), demand(), &read, &mut symbols)?.emit);
    // A failed continuation cannot poison its healthy source-class sibling, and the same class
    // also serves the selected-publication consumer.
    let transferred = Root::from(file.rule(RuleIndex(1), demand(), &read, &mut symbols)?.emit);
    let selected = Root::from(file.rule(RuleIndex(3), demand(), &read, &mut symbols)?.emit);
    let semantic = meanings.snapshot();
    let work = file.walk_counts();
    let reads = program.reads();
    let builds = derived.snapshot();
    let mut prefix = vec![];
    let refused = file
        .emit_rows(RuleIndex(2), &read, &mut symbols, &mut |row| {
            prefix.extend_from_slice(row);
            Err(OperatingError::NotRead)
        })
        .err();
    let mut replay = vec![];
    file.emit_rows(RuleIndex(2), &read, &mut symbols, &mut |row| {
        replay.extend_from_slice(row);
        Ok(())
    })?;
    let answer = if let Root::Complete { cells, .. } = &transferred {
        cells.to_vec()
    } else {
        Vec::new()
    };
    let repeated = Root::from(file.rule(RuleIndex(1), demand(), &read, &mut symbols)?.emit);
    let retries_repeat_no_work = meanings.snapshot() == semantic
        && file.walk_counts() == work
        && program.reads() == reads
        && derived.snapshot() == builds;
    let owner = Rc::clone(&file.owner);
    file.finish()?;
    Ok(Observed {
        continuations_apart,
        equivalent_extends_share_a_region,
        other_literal_has_its_own_region,
        failed: [first_failure, second_failure],
        collected_retry_is_the_answer: repeated == transferred,
        transferred,
        selected,
        failed_calls: semantic.failed,
        appended_calls: semantic.appended,
        walk: Walk {
            passes: work.passes.0,
            source_rows: work.source_rows.0,
            predicate_tests: work.predicate_tests.0,
            result_slots: work.result_slots.0,
        },
        refused,
        delivered_before_refusal: Cow::Owned(prefix),
        replay_is_the_answer: replay == answer,
        retries_repeat_no_work,
        working_bytes: owner.derived().working_bytes().0,
        answer_survives_retirement: answer == replay,
    })
}
