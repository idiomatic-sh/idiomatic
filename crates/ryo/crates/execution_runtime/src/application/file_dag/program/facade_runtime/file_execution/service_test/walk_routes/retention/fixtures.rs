//! The support of the `retention` cases.
use super::super::super::super::service::*;
use super::super::fixtures::plans::Authored;
use super::super::fixtures::semantics::{Derived, Meanings};
use super::super::fixtures::{Population, Program, Residency, demand};
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::frontier_capture::types::CaptureScope;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::row_emission::types::RowEmission;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{FileId, Value as Cell};
use ryo_values::domain::stored_facts::types::Digest;
use std::sync::Arc;

pub(super) fn captured_program() -> Program {
    Program::new(
        Authored::captured(),
        Population::Three,
        Residency::Stable,
        Arc::new(Meanings::default()),
        Arc::new(Derived::new()),
    )
    .unwrap()
}

pub(super) fn relations_program() -> Program {
    Program::new(
        Authored::relations(),
        Population::Three,
        Residency::Stable,
        Arc::new(Meanings::default()),
        Arc::new(Derived::new()),
    )
    .unwrap()
}

pub(super) fn capture(
    file: &mut FileRun<'_>,
    program: &Program,
    rule: RuleIndex,
    symbols: &mut Symbols,
) -> Result<usize, OperatingError> {
    let mut arena = vec![];
    let mut places = vec![];
    let mut words = ryo_relations::domain::relations::types::Words::default();
    file.capture_frontiers(
        rule,
        ryo_values::domain::stored_facts::types::Bucket(0),
        &|relation| program.read(relation),
        symbols,
        CaptureScope {
            file: FileId(0),
            rule_digest: Digest([0; 16]),
            worker: ryo_relations::domain::relations::types::WorkerNumber(0),
            retained: ryo_execution_domain::domain::finish::types::Retained {
                arena: ryo_execution_domain::domain::finish::types::Arena(&mut arena),
                places: &mut places,
            },
            words: &mut words,
        },
    )
    .map(|captures| captures.len())
}

pub(super) fn rows(
    file: &mut FileRun<'_>,
    program: &Program,
    rule: RuleIndex,
    symbols: &mut Symbols,
    each: &mut dyn FnMut(&[Cell]) -> Result<(), OperatingError>,
) -> Result<RowEmission, OperatingError> {
    file.emit_rows(rule, &|relation| program.read(relation), symbols, each)
}

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
) -> Result<RowEmission, OperatingError> {
    file.emit_rows(
        RuleIndex(2),
        &|relation| program.read(relation),
        symbols,
        each,
    )
}
