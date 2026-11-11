//! The support of the `retention` cases.
use super::super::super::super::service::*;
use super::super::fixtures::plans::Authored;
use super::super::fixtures::semantics::{Derived, Meanings};
use super::super::fixtures::{Population, Program, Residency, demand, observed_demand};
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::frontier_capture::types::CaptureScope;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::row_emission::types::RowEmission;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::stored_facts::types::Digest;
use std::borrow::Cow;
use std::sync::Arc;

/// The authored program a script opens one file of.
#[derive(Clone, Copy, Debug)]
pub(super) enum Authoring {
    /// Two rules that owe rows and a capture of one shared head.
    Captured,
    /// The continuation rules, rule 2 forwarded by rows.
    Continuations,
    /// Two classes on one relation and one class on another, consumed by two rules.
    Relations,
}

/// One action on the script's file.
#[derive(Clone, Copy, Debug)]
pub(super) enum Step {
    /// Delivers this rule's root without observations.
    Deliver(RuleIndex),
    /// Delivers this rule's root with its observations required.
    Observe(RuleIndex),
    /// Emits this rule's rows to a sink that takes them, or refuses the first.
    Rows { rule: RuleIndex, refuse: bool },
    /// Captures this rule's frontiers.
    Capture(RuleIndex),
    /// Compares the shared head classes of two rules.
    SameClass(RuleIndex, RuleIndex),
}

/// What a step answered.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Answer {
    /// The delivered root's rows.
    Rows(usize),
    /// The delivered root's failure.
    Failed(OperatingError),
    /// The emitted rows' emission was complete, or not.
    Emitted {
        complete: bool,
    },
    /// The frontiers captured.
    Captured(usize),
    /// Whether the two rules share a head class.
    SameClass(bool),
    Refused(OperatingError),
    /// Any other root outcome.
    Other,
}

/// A step's answer, and the batches the file retains and the walk passes made after it.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Seen {
    pub(super) answer: Answer,
    pub(super) retained: usize,
    pub(super) passes: u64,
}

/// What a script observed: every step, and whether the file's finish answered a frontier readiness.
#[derive(Debug, PartialEq)]
pub(super) struct Trace {
    pub(super) steps: Cow<'static, [Seen]>,
    pub(super) readiness: bool,
}

fn capture(
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

/// Opens one file of the authored program and runs the steps over it.
pub(super) fn script(authoring: Authoring, steps: &[Step]) -> Result<Trace, OperatingError> {
    let authored = match authoring {
        Authoring::Captured => Authored::captured(),
        Authoring::Continuations => Authored::continuations(),
        Authoring::Relations => Authored::relations(),
    };
    let program = Program::new(
        authored,
        Population::Three,
        Residency::Stable,
        Arc::new(Meanings::default()),
        Arc::new(Derived::new()),
    )?;
    let mut symbols = Symbols::default();
    let mut file = program.open(&mut symbols)?;
    let read = |relation| program.read(relation);
    let mut seen = Vec::new();
    for step in steps {
        let delivered = |outcome: Result<RootOutcome, OperatingError>| {
            outcome.map_or_else(Answer::Refused, |outcome| match outcome {
                RootOutcome::Complete(table) => Answer::Rows(table.count.0),
                RootOutcome::Failed(error) => Answer::Failed(error),
                _ => Answer::Other,
            })
        };
        let answer = match *step {
            Step::Deliver(rule) => delivered(
                file.rule(rule, demand(), &read, &mut symbols)
                    .map(|outcome| outcome.emit),
            ),
            Step::Observe(rule) => delivered(
                file.rule(rule, observed_demand(), &read, &mut symbols)
                    .map(|outcome| outcome.emit),
            ),
            Step::Rows { rule, refuse } => file
                .emit_rows(rule, &read, &mut symbols, &mut |_| {
                    if refuse {
                        Err(OperatingError::NotRead)
                    } else {
                        Ok(())
                    }
                })
                .map_or_else(Answer::Refused, |emission| Answer::Emitted {
                    complete: matches!(emission, RowEmission::Complete),
                }),
            Step::Capture(rule) => capture(&mut file, &program, rule, &mut symbols)
                .map_or_else(Answer::Refused, Answer::Captured),
            Step::SameClass(left, right) => {
                Answer::SameClass(program.head_class(left)? == program.head_class(right)?)
            }
        };
        seen.push(Seen {
            answer,
            retained: file.walk.retained().0,
            passes: file.walk_counts().passes.0,
        });
    }
    Ok(Trace {
        steps: Cow::Owned(seen),
        readiness: file.finish()?.is_some(),
    })
}
