//! The support of the `fusion` cases.
use super::super::fixtures::plans::Authored;
use super::super::fixtures::routes::{Combine, Feed, Fusion};
use super::super::fixtures::semantics::{Derived, MeaningSnapshot, Meanings};
use super::super::fixtures::{Population, Program, Residency, demand};
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::executable_program::fusion::types::Fusibility;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use std::borrow::Cow;
use std::sync::Arc;

/// What one run of a combining program decided: rows or error, the faults it reached, how the
/// projection after each of its two heads was classed, and whether its file's finish answered a
/// frontier readiness.
#[derive(Debug, PartialEq)]
pub(super) struct Decision {
    pub(super) answer: Cow<'static, str>,
    pub(super) reached: MeaningSnapshot,
    pub(super) classes: [Option<Fusibility>; 2],
    pub(super) readiness: bool,
}

impl Decision {
    /// Whether two runs answered alike and reached the same faults, however they were classed.
    pub(super) fn agrees(&self, other: &Self) -> bool {
        self.answer == other.answer && self.reached == other.reached
    }
}

/// How a named capture program classes each named producer's projection (with its region's
/// instruction count) and its consumer's first step.
#[derive(Debug, PartialEq)]
pub(super) struct Capture {
    pub(super) projections: [(Option<Fusibility>, usize); 2],
    pub(super) consumer: Option<Fusibility>,
}

/// Compiles the named continuation program and classes its producers' projections and consumer.
pub(super) fn capture() -> Result<Capture, OperatingError> {
    let program = Program::new(
        Authored::named_continuation(),
        Population::Three,
        Residency::Stable,
        Arc::new(Meanings::default()),
        Arc::new(Derived::new()),
    )?;
    let compiled = program.prepared.program();
    let projection = |rule: RuleIndex| -> Result<(Option<Fusibility>, usize), OperatingError> {
        let projection = compiled.rules()[rule.0].original_steps[1]
            .as_ref()
            .map_err(Clone::clone)?
            .logical;
        let region = compiled.logical_regions()[projection.0];
        Ok((
            compiled.fusibility(projection),
            compiled.regions()[region.0].instructions().len(),
        ))
    };
    // Its consumer reads a named observation: a source, never a continuation.
    let consumer = compiled.rules()[2].original_steps[0]
        .as_ref()
        .map_err(Clone::clone)?
        .logical;
    Ok(Capture {
        projections: [projection(RuleIndex(0))?, projection(RuleIndex(1))?],
        consumer: compiled.fusibility(consumer),
    })
}

pub(super) fn decide(
    combine: Combine,
    left: Feed,
    right: Feed,
    fusion: Fusion,
) -> Result<Decision, OperatingError> {
    let meanings = Arc::new(Meanings::default());
    let program = Program::new(
        Authored::combined(combine, left, right, fusion),
        Population::Three,
        Residency::Stable,
        meanings.clone(),
        Arc::new(Derived::new()),
    )?;
    // The projection after each head is the only thing the second reader keeps apart.
    let compiled = program.prepared.program();
    let mut classes = [None; 2];
    for (class, head) in classes.iter_mut().zip([0usize, 3]) {
        let projection = compiled.rules()[0].original_steps[head + 1]
            .as_ref()
            .map_err(Clone::clone)?
            .logical;
        *class = compiled.fusibility(projection);
    }
    let mut symbols = Symbols::default();
    let mut file = program.open(&mut symbols)?;
    let read = |relation| program.read(relation);
    let answer = match file.rule(RuleIndex(0), demand(), &read, &mut symbols)?.emit {
        RootOutcome::Complete(table) => format!("complete {:?} {}", table.cells, table.count.0),
        RootOutcome::Failed(error) => format!("failed {error:?}"),
        _ => "other".to_owned(),
    };
    let readiness = file.finish()?.is_some();
    let mut reached = meanings.snapshot();
    // Projection calls are the continuation itself; every other call is observable order.
    reached.projects = super::super::fixtures::semantics::Calls(0);
    Ok(Decision {
        answer: Cow::Owned(answer),
        reached,
        classes,
        readiness,
    })
}
