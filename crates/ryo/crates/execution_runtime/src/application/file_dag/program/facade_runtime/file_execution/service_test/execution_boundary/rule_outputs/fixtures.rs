//! The support of the `rule_outputs` cases.
use super::super::super::super::service::*;
use super::super::super::fixtures::*;
use crate::application::file_dag::execution::service::CommonOperatorEngine;
use crate::application::file_dag::program::facade_runtime::service::ProgramServices;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::rule_intent::types::CaptureIntent;
use ryo_execution_domain::domain::rule_intent::types::DiagnosticDelivery;
use ryo_execution_domain::domain::rule_intent::types::EmitDemand;
use ryo_execution_domain::domain::rule_intent::types::FileRuleIntent;
use ryo_execution_domain::domain::rule_intent::types::ObservationDemand;
use ryo_execution_domain::domain::rule_intent::types::RuleDemand;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_execution_domain::domain::{
    observation_schedule::types::RuleSchedule,
    prepared_request::types::{Compiled, ObservationIdentity},
};
use ryo_execution_store::application::file_dag::derived::relation_owner::types::ScopePartials;
use ryo_execution_store::application::file_dag::relations::types::ReuseEligibility;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::Integer;
use ryo_relations::domain::relations::types::Value as Cell;
use ryo_relations::domain::relations::types::{CellKind, FileId};
use ryo_values::domain::hosted::types::HostKind;
use ryo_values::domain::plan_parts::types::HostedNumber;
use ryo_values::domain::row_texts::types::Cell as HostedCell;
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    plan_parts::types::{ColumnIndex, Input as PlanInput, RepeatedRows, StepId},
    plans::types::Operation,
    runs::types::RuleId,
    stored_facts::types::Digest,
};
use std::borrow::Cow;
use std::sync::Arc;

pub(super) fn output_requests(
    outputs: &[(&str, usize)],
    bad_first: bool,
) -> (
    Arc<crate::application::file_dag::program::preparation::types::PreparedProgram>,
    Arc<ProgramServices>,
) {
    use ryo_values::domain::hosted::types::{
        HostColumn, HostColumnName, HostName, HostRelation, HostRow,
    };
    let hosts = outputs
        .iter()
        .map(|(name, count)| HostRelation {
            name: HostName(format!("output-{name}").into()),
            columns: vec![HostColumn {
                name: HostColumnName("value".into()),
                kind: HostKind::Number,
            }],
            rows: (0..*count)
                .map(|_| HostRow(vec![HostedCell::Number(Integer(7))]))
                .collect(),
        })
        .collect::<Vec<_>>();
    let plans = outputs
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let mut plan = plan(PlanShape::ProjectRetry);
            plan.steps[0].operation = Operation::Hosted {
                relation: HostedNumber(index as u32),
            };
            plan.steps[1].operation = Operation::Project {
                from: PlanInput::Step(StepId(0)),
                columns: vec![ColumnIndex(if bad_first && index == 0 { 99 } else { 0 })],
                distinct: RepeatedRows::Kept,
            };
            plan
        })
        .collect();
    let rules = outputs
        .iter()
        .map(|(name, _)| {
            let mut rule = rule();
            rule.id = RuleId(format!("output-{name}").into());
            rule
        })
        .collect::<Vec<_>>();
    let derived = ryo_execution_store::application::file_dag::fixtures::program_derived_port();
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![]; outputs.len()],
            plans,
            bindings: vec![vec![]; outputs.len()],
        },
        &rules,
        &vec![Digest([0; 16]); outputs.len()],
        &hosts,
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    )
    .unwrap();
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(Arc::new(
            crate::application::file_dag::fixtures::operators(),
        ))),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("owned-outputs.py".into()),
            size: FileSize(0),
            modified: Nanoseconds(0),
        })],
    ));
    (prepared, services)
}

/// The address of a complete root's cells, if it is complete.
fn pointer(root: &RootOutcome) -> Option<*const Cell> {
    if let RootOutcome::Complete(table) = root {
        Some(table.cells.as_ptr())
    } else {
        None
    }
}

/// The program a row runs.
#[derive(Clone, Copy, Debug)]
pub(super) enum Program {
    /// Four rules of 4096, 1, 0 and 3 output rows.
    Sized,
    /// A rule refused for a missing column, then an independent rule.
    FailingThenFollowing,
}

/// One rule's output, after the file finished.
#[derive(Debug, PartialEq)]
pub(super) struct Output {
    /// The output's column kinds.
    pub(super) kinds: Cow<'static, [CellKind]>,
    /// The output.
    pub(super) root: Root,
    /// Whether the output still holds the allocation it held before the file finished.
    pub(super) allocation_kept: bool,
}

impl Output {
    fn after_finish(root: RootOutcome, before: Option<*const Cell>) -> Self {
        let kinds = if let RootOutcome::Complete(table) = &root {
            Cow::Owned(table.kinds.clone())
        } else {
            Cow::Borrowed(&[][..])
        };
        Self {
            kinds,
            allocation_kept: pointer(&root) == before,
            root: Root::from(root),
        }
    }
}

/// What the program answered.
#[derive(Debug, PartialEq)]
pub(super) enum Observed {
    /// Each rule's output, in rule order.
    Owned([Output; 4]),
    /// The failing rule, the following rule, and the failing rule run again.
    Isolated {
        failed: Root,
        next: Output,
        repeated: Root,
    },
}

/// Runs the program.
pub(super) fn exercise(program: Program) -> Observed {
    match program {
        Program::Sized => owned(),
        Program::FailingThenFollowing => isolated(),
    }
}

/// Runs four rules whose outputs hold 4096, 1, 0 and 3 rows, then finishes the file.
fn owned() -> Observed {
    let outputs = [("large", 4096), ("tiny", 1), ("empty", 0), ("small", 3)];
    let (prepared, services) = output_requests(&outputs, false);
    let mut symbols = Symbols::default();
    let mut file = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: &[],
            publication: &[],
        },
        &[FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        }; 4],
        services,
        ryo_values::domain::files::types::SourceView(b""),
        ReuseEligibility::Eligible,
        &mut symbols,
        None,
        None,
    )
    .unwrap();
    let read = |_| Err::<Rows<'_>, _>(OperatingError::NotRead);
    let mut tables = Vec::new();
    for index in 0..outputs.len() {
        let outcome = file
            .rule(
                RuleIndex(index),
                RuleDemand {
                    emit: EmitDemand::Enabled,
                    observations: ObservationDemand::NotRequired,
                },
                &read,
                &mut symbols,
            )
            .unwrap();
        tables.push(outcome.emit);
    }
    let pointers = tables.iter().map(pointer).collect::<Vec<_>>();
    file.finish().unwrap();
    Observed::Owned(
        tables
            .into_iter()
            .zip(pointers)
            .map(|(root, before)| Output::after_finish(root, before))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap(),
    )
}

/// Runs a failing rule, an independent rule, then the failing rule again, and finishes the file.
fn isolated() -> Observed {
    let (prepared, services) = output_requests(&[("failing", 2), ("following", 2)], true);
    let mut symbols = Symbols::default();
    let mut file = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: &[],
            publication: &[],
        },
        &[FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        }; 2],
        services,
        ryo_values::domain::files::types::SourceView(b""),
        ReuseEligibility::Eligible,
        &mut symbols,
        None,
        None,
    )
    .unwrap();
    let read = |_| Err::<Rows<'_>, _>(OperatingError::NotRead);
    let demand = || RuleDemand {
        emit: EmitDemand::Enabled,
        observations: ObservationDemand::NotRequired,
    };
    let failed = file
        .rule(RuleIndex(0), demand(), &read, &mut symbols)
        .unwrap();
    let next = file
        .rule(RuleIndex(1), demand(), &read, &mut symbols)
        .unwrap();
    let repeated = file
        .rule(RuleIndex(0), demand(), &read, &mut symbols)
        .unwrap();
    let before = pointer(&next.emit);
    file.finish().unwrap();
    Observed::Isolated {
        failed: Root::from(failed.emit),
        next: Output::after_finish(next.emit, before),
        repeated: Root::from(repeated.emit),
    }
}
