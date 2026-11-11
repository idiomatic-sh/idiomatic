//! The support of the `lifetime` cases.
use super::super::super::super::*;
use super::super::fixtures::plans::Authored;
use super::super::fixtures::semantics::{Derived, Meanings};
use super::super::fixtures::{Population, Program, Residency, demand};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::Value as Cell;
use std::sync::Arc;

pub(super) fn program(residency: Residency) -> Program {
    Program::new(
        Authored::continuations(),
        Population::Three,
        residency,
        Arc::new(Meanings::default()),
        Arc::new(Derived::new()),
    )
    .unwrap()
}

pub(super) fn deliver(
    file: &mut FileRun<'_>,
    program: &Program,
    rule: RuleIndex,
    symbols: &mut Symbols,
) -> RootOutcome {
    file.rule(rule, demand(), &|relation| program.read(relation), symbols)
        .unwrap()
        .emit
}

pub(super) fn forward(
    file: &mut FileRun<'_>,
    program: &Program,
    symbols: &mut Symbols,
    each: &mut dyn FnMut(&[Cell]) -> Result<(), OperatingError>,
) -> Result<ryo_execution_domain::domain::row_emission::types::RowEmission, OperatingError> {
    file.emit_rows(
        RuleIndex(2),
        &|relation| program.read(relation),
        symbols,
        each,
    )
}
