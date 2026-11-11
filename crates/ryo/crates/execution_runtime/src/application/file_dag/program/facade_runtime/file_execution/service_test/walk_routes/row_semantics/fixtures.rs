//! The support of the `row_semantics` cases.
use super::super::super::super::service::*;
use super::super::super::fixtures::Root;
use super::super::fixtures::plans::Authored;
use super::super::fixtures::routes::RowRoute;
use super::super::fixtures::semantics::{Derived, Meanings};
use super::super::fixtures::{Population, Program, Residency, demand};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::executable_program::fusion::types::Fusibility;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::plans::service::kind_of;
use std::sync::Arc;

/// What a per-row or request-owned route beside a shared head observed.
#[derive(Debug, PartialEq)]
pub(super) struct Observed {
    pub(super) kind: ryo_values::domain::query_operators::types::OperationKind,
    /// Whether the routed instruction is a member of the source walk's class.
    pub(super) class_member: bool,
    pub(super) fusibility: Option<Fusibility>,
    pub(super) instructions: usize,
    pub(super) head: Root,
    pub(super) routed: Result<Root, OperatingError>,
    pub(super) walk_counts_unchanged: bool,
    pub(super) retry_same: bool,
    pub(super) head_unpoisoned: bool,
}

/// Runs a shared head over three function rows and the route as a second rule, the route
/// twice and the head again.
pub(super) fn exercise(route: RowRoute) -> Result<Observed, OperatingError> {
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
    let kind = kind_of(&instruction.operation());
    let class_member = program.prepared.walk_class(instruction).is_some();
    let fusibility = compiled.fusibility(logical);
    let instructions = compiled.regions()[region.0].instructions().len();

    let mut symbols = Symbols::default();
    let mut file = program.open(&mut symbols)?;
    let read = |relation| program.read(relation);
    let head = |file: &mut FileRun<'_>, symbols: &mut Symbols| {
        file.rule(RuleIndex(0), demand(), &read, symbols)
            .map(|outcome| Root::from(outcome.emit))
    };
    let routed = |file: &mut FileRun<'_>, symbols: &mut Symbols| {
        file.rule(RuleIndex(1), demand(), &read, symbols)
            .map(|outcome| Root::from(outcome.emit))
    };
    let first = head(&mut file, &mut symbols)?;
    let counts = file.walk_counts();
    let answer = routed(&mut file, &mut symbols);
    let walk_counts_unchanged = file.walk_counts() == counts;
    let retry_same = routed(&mut file, &mut symbols) == answer;
    let head_unpoisoned = head(&mut file, &mut symbols)? == first;
    file.finish()?;
    Ok(Observed {
        kind,
        class_member,
        fusibility,
        instructions,
        head: first,
        routed: answer,
        walk_counts_unchanged,
        retry_same,
        head_unpoisoned,
    })
}
