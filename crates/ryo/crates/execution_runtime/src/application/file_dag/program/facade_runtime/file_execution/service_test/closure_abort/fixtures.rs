//! The support of the `closure_abort` cases.
use super::super::super::service::*;
use super::super::fixtures::*;
use crate::application::file_dag::execution::service::CommonOperatorEngine;
use crate::application::file_dag::program::facade_runtime::service::ProgramServices;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::CaptureIntent;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::DiagnosticDelivery;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::EmitDemand;
use ryo_execution_domain::domain::rule_intent::types::FileRuleIntent;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::ObservationDemand;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::RuleDemand;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_execution_domain::domain::{
    observation_schedule::types::RuleSchedule,
    prepared_request::types::{Compiled, ObservationIdentity},
};
use ryo_execution_store::application::file_dag::relations::types::ReuseEligibility;
use ryo_execution_store::application::file_dag::service::ScopePartials;
use ryo_language::domain::{files::types::FilePath, symbols::types::Symbols};
use ryo_relations::domain::relations::types::{FileId, Integer, Value as Cell};
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    plan_parts::types::{ColumnIndex, Input as PlanInput, StepId, StepScope},
    plans::types::{Operation, PlanStep},
    stored_facts::types::Digest,
};
use ryo_values::domain::{
    hosted::types::HostKind,
    plan_parts::types::{HostedNumber, LiteralText, Operand, PlanComparison, QueryLiteral, Test},
    row_texts::types::Cell as HostCell,
};
use std::{rc::Rc, sync::Arc};

/// Where the later partition's failure comes from.
#[derive(Clone, Copy, Debug)]
pub(super) enum Refusal {
    CollectedPattern,
    DeliveredPattern,
    Callback,
}

/// The first row a sink was delivered: its key, its hosted text and its two endpoints.
#[derive(Debug, PartialEq)]
pub(super) struct Delivered {
    pub(super) key: Cell,
    pub(super) text: std::borrow::Cow<'static, str>,
    pub(super) endpoints: [Cell; 2],
}

/// A refusal: a pattern the regex compiler refused (its explanation is the compiler's), or
/// another error in full.
#[derive(Debug, PartialEq)]
pub(super) enum Refused {
    InvalidRegex,
    Error(OperatingError),
}

impl From<OperatingError> for Refused {
    fn from(error: OperatingError) -> Self {
        if matches!(error, OperatingError::InvalidRegex { .. }) {
            Self::InvalidRegex
        } else {
            Self::Error(error)
        }
    }
}

/// What a closure whose later partition fails observed.
#[derive(Debug, PartialEq)]
pub(super) struct Observed {
    /// Whether the closure and its select share one physical region.
    pub(super) fused: bool,
    pub(super) refused: Option<Refused>,
    pub(super) rows_delivered: usize,
    pub(super) first_row: Option<Delivered>,
    pub(super) partition_builds: usize,
    pub(super) order_first_on_keys: bool,
    pub(super) requests: usize,
    pub(super) rest_are_adjacency: bool,
    pub(super) backing_released: bool,
    pub(super) working_bytes: usize,
    pub(super) publication_bytes: usize,
}

/// Runs a closure over three keyed partitions whose select fails on the second (a bad pattern)
/// or whose sink refuses the first row, fused with its select or not.
pub(super) fn exercise(refusal: Refusal, fused: bool) -> Observed {
    let mut plan = plan(PlanShape::ProjectRetry);
    plan.steps = vec![
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Hosted {
                relation: HostedNumber(0),
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Closure {
                edges: PlanInput::Step(StepId(0)),
                keys: vec![ColumnIndex(2), ColumnIndex(3)],
                source: ColumnIndex(0),
                destination: ColumnIndex(1),
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: PlanInput::Step(StepId(1)),
                comparisons: vec![PlanComparison {
                    test: Test::Matches,
                    left: Operand::Literal(QueryLiteral::Text(LiteralText("a".into()))),
                    right: Operand::Column(ColumnIndex(1)),
                    origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                }],
                kept: vec![
                    ColumnIndex(0),
                    ColumnIndex(1),
                    ColumnIndex(2),
                    ColumnIndex(3),
                ],
            },
        },
    ];
    plan.emit.step = StepId(2);
    if fused {
        let Operation::Select { kept, .. } = &mut plan.steps[2].operation else {
            panic!("the fixture builds a Select")
        };
        kept.clear(); // All four columns still pass through; this is the actual fusible shape.
    }
    let patterns = if matches!(refusal, Refusal::Callback) {
        ["a", "a", "a"]
    } else {
        ["a", "[", "z"]
    };
    let host = super::super::scratch_owner::fixtures::hosted(
        &[
            HostKind::Number,
            HostKind::Number,
            HostKind::Number,
            HostKind::Text,
        ],
        patterns
            .into_iter()
            .enumerate()
            .map(|(key, pattern)| {
                vec![
                    HostCell::Number(Integer(1)),
                    HostCell::Number(Integer(2)),
                    HostCell::Number(Integer(key as i64)),
                    HostCell::Text(pattern.into()),
                ]
            })
            .collect(),
    );
    let recorded = Arc::new(
        ryo_execution_store::application::file_dag::derived::owner::fixtures::resources::RecordedDerived::default(
        ),
    );
    let derived: Arc<dyn ryo_execution_store::outbound::derived::interface::DerivedPort> =
        recorded.clone();
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![]],
            plans: vec![plan],
            bindings: vec![vec![]],
        },
        &[rule()],
        &[Digest([0; 16])],
        &[host],
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    )
    .unwrap();
    let program = prepared.program();
    let physical = |step: usize| {
        let logical = program.rules()[0].original_steps[step]
            .as_ref()
            .unwrap()
            .logical;
        program.logical_regions()[logical.0]
    };
    // The test observes the actual fused or materialized compiled boundary.
    let fused_boundary = physical(1) == physical(2);
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(Arc::new(
            crate::application::file_dag::fixtures::operators(),
        ))),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("closure-abort.py".into()),
            size: FileSize(0),
            modified: Nanoseconds(0),
        })],
    ));
    let mut symbols = Symbols::default();
    let mut file = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: &[],
            publication: &[],
        },
        &[FileRuleIntent::Run {
            diagnostic: if matches!(refusal, Refusal::CollectedPattern) {
                DiagnosticDelivery::Collected
            } else {
                DiagnosticDelivery::Forwarded
            },
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        }],
        services,
        ryo_values::domain::files::types::SourceView(b""),
        ReuseEligibility::Eligible,
        &mut symbols,
        None,
        None,
    )
    .unwrap();
    let owner = Rc::clone(&file.owner);
    let read = |_| Err::<Rows<'_>, _>(OperatingError::NotRead);
    let mut delivered = Vec::new();
    let refused = match refusal {
        Refusal::CollectedPattern => {
            let result = file
                .rule(
                    RuleIndex(0),
                    RuleDemand {
                        emit: EmitDemand::Enabled,
                        observations: ObservationDemand::NotRequired,
                    },
                    &read,
                    &mut symbols,
                )
                .unwrap();
            if let RootOutcome::Failed(error) = result.emit {
                Some(Refused::from(error))
            } else {
                None
            }
        }
        Refusal::DeliveredPattern | Refusal::Callback => file
            .emit_rows(RuleIndex(0), &read, &mut symbols, &mut |row| {
                delivered.push(row.to_vec());
                if matches!(refusal, Refusal::Callback) {
                    Err(OperatingError::WrongBinding)
                } else {
                    Ok(())
                }
            })
            .err()
            .map(Refused::from),
    };
    // A later semantic failure is atomic for callbacks too; completed publication delivery
    // stops at the first sink refusal.
    let first_row = delivered.first().map(|row| {
        let text = match row.get(1) {
            Some(Cell::Symbol(symbol)) => ryo_language::domain::symbols::service::text_of(
                &symbols,
                ryo_language::domain::symbols::types::ByteSlice(b""),
                *symbol,
            )
            .map_or_else(String::new, |text| text.0.to_owned()),
            _ => String::new(),
        };
        Delivered {
            key: row[0],
            text: std::borrow::Cow::Owned(text),
            endpoints: [row[2], row[3]],
        }
    });
    let (builds, last_backing) = owner.derived().scoped_observation();
    // Only a real fused continuation can stop partition construction at its semantic failure;
    // the shared partition order is a separate family from actual scoped adjacency construction.
    let calls = recorded.builds();
    let order_first_on_keys = matches!(calls.first(), Some(ryo_derived::domain::request::types::DerivedRequest::PartitionOrder { keys })
        if keys == &[ColumnIndex(2), ColumnIndex(3)]);
    let rest_are_adjacency = calls.iter().skip(1).all(|request| {
        matches!(
            request,
            ryo_derived::domain::request::types::DerivedRequest::ClosureAdjacency {
                source: ColumnIndex(0),
                destination: ColumnIndex(1)
            }
        )
    });
    // A failed invocation releases its current partition backing.
    let backing_released = last_backing.upgrade().is_none();
    file.finish().unwrap();
    Observed {
        fused: fused_boundary,
        refused,
        rows_delivered: delivered.len(),
        first_row,
        partition_builds: builds,
        order_first_on_keys,
        requests: calls.len(),
        rest_are_adjacency,
        backing_released,
        working_bytes: owner.derived().working_bytes().0,
        publication_bytes: owner.derived().publication_ownership_bytes().0,
    }
}
