//! The support of the `lifetime` cases.
use super::super::super::super::service::*;
use super::super::fixtures::plans::Authored;
use super::super::fixtures::semantics::{Derived, Meanings};
use super::super::fixtures::{Population, Program, Residency, demand};
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use std::borrow::Cow;
use std::sync::Arc;

/// One action on the open files of a script.
#[derive(Clone, Copy, Debug)]
pub(super) enum Step {
    /// Delivers this rule's root in this file.
    Deliver { file: usize, rule: RuleIndex },
    /// Forwards rule 2's rows in this file to a sink that takes them, or refuses the first.
    Forward { file: usize, refuse: bool },
    /// Delivers this rule again in this file and compares it with its first complete delivery.
    Redeliver { file: usize, rule: RuleIndex },
    /// Leases rule 1's head class from file 0's walk and holds the lease.
    Hold,
    /// Demands rule 1's head class from file 0's walk again.
    Demand,
    /// Reads the held lease's positions again.
    Reread,
}

/// What a step answered.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Answer {
    /// The delivered root's rows.
    Rows(usize),
    Failed(OperatingError),
    /// The forwarded rows' emission was complete, or not.
    Forwarded {
        complete: bool,
    },
    Refused(OperatingError),
    /// The repeated delivery equals the first, or not.
    Repeated {
        same: bool,
    },
    /// The held lease's positions.
    Held(usize),
    /// Whether a demand found a batch.
    Demanded {
        found: bool,
    },
    /// Whether the held lease still reads the positions it first read.
    Reread {
        same: bool,
    },
    /// Any other root outcome.
    Other,
}

/// A step's answer and the batches each open file retains after it.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Seen {
    pub(super) answer: Answer,
    pub(super) retained: Cow<'static, [usize]>,
}

/// What a script observed: the batches each file retained when opened, every step, each file's
/// walk passes and source rows, and whether each file's finish answered a frontier readiness.
#[derive(Debug, PartialEq)]
pub(super) struct Trace {
    pub(super) opened: Cow<'static, [usize]>,
    pub(super) steps: Cow<'static, [Seen]>,
    pub(super) counts: Cow<'static, [(u64, u64)]>,
    pub(super) readiness: Cow<'static, [bool]>,
}

/// Opens `files` files of the continuation program and runs the steps over them.
pub(super) fn script(
    residency: Residency,
    files: usize,
    steps: &[Step],
) -> Result<Trace, OperatingError> {
    let program = Program::new(
        Authored::continuations(),
        Population::Three,
        residency,
        Arc::new(Meanings::default()),
        Arc::new(Derived::new()),
    )?;
    let class = program.head_class(RuleIndex(1))?;
    let mut symbols = (0..files).map(|_| Symbols::default()).collect::<Vec<_>>();
    let mut open = Vec::new();
    for symbols in &mut symbols {
        open.push(program.open(symbols)?);
    }
    let retained = |open: &[FileRun<'_>]| -> Cow<'static, [usize]> {
        open.iter().map(|file| file.walk.retained().0).collect()
    };
    let opened = retained(&open);
    let mut first = Vec::new();
    let mut held = None;
    let mut positions = Vec::new();
    let mut seen = Vec::new();
    for step in steps {
        let answer = match *step {
            Step::Deliver { file, rule } => {
                match open[file]
                    .rule(
                        rule,
                        demand(),
                        &|relation| program.read(relation),
                        &mut symbols[file],
                    )?
                    .emit
                {
                    RootOutcome::Complete(table) => {
                        let rows = table.count.0;
                        if !first.iter().any(|(key, _)| *key == (file, rule)) {
                            first.push(((file, rule), table));
                        }
                        Answer::Rows(rows)
                    }
                    RootOutcome::Failed(error) => Answer::Failed(error),
                    _ => Answer::Other,
                }
            }
            Step::Forward { file, refuse } => open[file]
                .emit_rows(
                    RuleIndex(2),
                    &|relation| program.read(relation),
                    &mut symbols[file],
                    &mut |_| {
                        if refuse {
                            Err(OperatingError::NotRead)
                        } else {
                            Ok(())
                        }
                    },
                )
                .map_or_else(Answer::Refused, |emission| Answer::Forwarded {
                    complete: matches!(
                        emission,
                        ryo_execution_domain::domain::row_emission::types::RowEmission::Complete
                    ),
                }),
            Step::Redeliver { file, rule } => {
                let again = open[file]
                    .rule(
                        rule,
                        demand(),
                        &|relation| program.read(relation),
                        &mut symbols[file],
                    )?
                    .emit;
                Answer::Repeated {
                    same: matches!(again, RootOutcome::Complete(table)
                        if first.iter().any(|(key, earlier)| *key == (file, rule) && *earlier == table)),
                }
            }
            Step::Hold => {
                let lease = open[0]
                    .walk
                    .lease_any(class)
                    .ok_or(OperatingError::NotRead)?;
                positions = lease.result().positions.0.to_vec();
                held = Some(lease);
                Answer::Held(positions.len())
            }
            Step::Demand => Answer::Demanded {
                found: open[0].walk.lease_any(class).is_some(),
            },
            Step::Reread => Answer::Reread {
                same: held
                    .as_ref()
                    .is_some_and(|held| held.result().positions.0.to_vec() == positions),
            },
        };
        seen.push(Seen {
            answer,
            retained: retained(&open),
        });
    }
    drop(held);
    let counts = open
        .iter()
        .map(|file| {
            let counts = file.walk_counts();
            (counts.passes.0, counts.source_rows.0)
        })
        .collect();
    let mut readiness = Vec::new();
    for file in open {
        readiness.push(file.finish()?.is_some());
    }
    Ok(Trace {
        opened,
        steps: Cow::Owned(seen),
        counts,
        readiness: Cow::Owned(readiness),
    })
}
