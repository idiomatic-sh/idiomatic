//! The support of the `invocation_state` cases.
use super::super::super::super::service::*;
use super::super::super::fixtures::*;
use super::super::super::scratch_owner::fixtures::{Left, execute, hosted};
use crate::application::file_dag::execution::service::CommonOperatorEngine;
use crate::application::file_dag::program::facade_runtime::service::ProgramServices;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operating::types::Memo;
use ryo_execution_domain::domain::operator::types::Emit as RowSink;
#[cfg(test)]
use ryo_execution_domain::domain::operator::types::OperatorContext;
use ryo_execution_domain::domain::rule_intent::types::CaptureIntent;
use ryo_execution_domain::domain::rule_intent::types::DiagnosticDelivery;
use ryo_execution_domain::domain::rule_intent::types::FileRuleIntent;
use ryo_execution_domain::domain::rule_intent::types::ObservationDemand;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_execution_domain::domain::{
    observation_schedule::types::RuleSchedule,
    prepared_request::types::{Compiled, ObservationIdentity},
};
use ryo_execution_store::application::file_dag::relations::types::ReuseEligibility;
use ryo_execution_store::application::file_dag::derived::relation_owner::types::ScopePartials;
use ryo_language::domain::{files::types::FilePath, symbols::types::Symbols};
use ryo_relations::domain::relations::types::{FileId, Integer, Value as Cell};
use ryo_values::domain::hosted::types::HostKind;
use ryo_values::domain::plan_parts::types::HostedNumber;
use ryo_values::domain::row_texts::types::Cell as HostedCell;
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    plan_parts::types::{ColumnIndex, Input as PlanInput, RepeatedRows, StepId, StepScope},
    plans::types::{Operation, PlanStep},
    stored_facts::types::Digest,
};
use std::borrow::Cow;
use std::sync::Arc;

pub(super) struct MemoSink {
    pub(super) starts: Vec<usize>,
    pub(super) rows: Vec<Cell>,
}

impl RowSink for MemoSink {
    fn row(
        &mut self,
        context: &mut OperatorContext<'_>,
        row: &[Cell],
    ) -> Result<(), OperatingError> {
        self.starts.push(context.memo.len());
        context.memo.push(Memo::Holds);
        self.rows.extend_from_slice(row);
        Ok(())
    }
}

/// The program a row runs.
#[derive(Clone, Copy, Debug)]
pub(super) enum Program {
    /// Registered evaluations chained through select and project.
    Chained,
    /// A closure over a large then a small partition.
    Partitioned,
}

/// What the program answered.
#[derive(Debug, PartialEq)]
pub(super) enum Observed {
    /// The memo length at each delivered row, and the rows.
    Chained {
        starts: Cow<'static, [usize]>,
        rows: Cow<'static, [Cell]>,
    },
    /// The closure's rows, the rows of its largest partition, whether the partition buffer kept
    /// the capacity of both peaks (64 and up), and what the run left behind.
    Closure {
        rows: usize,
        partition_rows: usize,
        capacity_holds_both_peaks: bool,
        left: Left,
    },
}

/// Runs the program.
pub(super) fn exercise(program: Program) -> Observed {
    match program {
        Program::Chained => chained(),
        Program::Partitioned => partitioned(),
    }
}

/// Runs a hosted, select, project chain whose sink records the memo length at each row.
fn chained() -> Observed {
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
            operation: Operation::Select {
                from: PlanInput::Step(StepId(0)),
                comparisons: vec![],
                kept: vec![],
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Project {
                from: PlanInput::Step(StepId(1)),
                columns: vec![ColumnIndex(0)],
                distinct: RepeatedRows::Kept,
            },
        },
    ];
    plan.emit.step = StepId(2);
    let host = hosted(
        &[HostKind::Number],
        (1..=3)
            .map(|number| vec![HostedCell::Number(Integer(number))])
            .collect(),
    );
    let derived = ryo_execution_store::application::file_dag::fixtures::program_derived_port();
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
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(Arc::new(
            crate::application::file_dag::fixtures::operators(),
        ))),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("chained-state.py".into()),
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
            diagnostic: DiagnosticDelivery::Collected,
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
    let read = |_| Err::<Rows<'_>, _>(OperatingError::NotRead);
    let mut sink = MemoSink {
        starts: vec![],
        rows: vec![],
    };
    forward(&mut file, &read, &mut symbols, &mut sink).unwrap();
    file.finish().unwrap();
    Observed::Chained {
        starts: Cow::Owned(sink.starts),
        rows: Cow::Owned(sink.rows),
    }
}
/// Closes a large partition of 32 edges, then a small one of one edge.
fn partitioned() -> Observed {
    let mut rows = (0..32)
        .map(|at| {
            vec![
                HostedCell::Number(Integer(at)),
                HostedCell::Number(Integer(at + 1)),
                HostedCell::Number(Integer(0)),
            ]
        })
        .collect::<Vec<_>>();
    rows.push(vec![
        HostedCell::Number(Integer(100)),
        HostedCell::Number(Integer(101)),
        HostedCell::Number(Integer(1)),
    ]);
    let (result, observed, left) = execute(
        Operation::Closure {
            edges: PlanInput::Step(StepId(0)),
            keys: vec![ColumnIndex(2)],
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
        },
        hosted(&[HostKind::Number; 3], rows),
        &mut Symbols::default(),
    );
    // execute calls normal file.finish and observes the real owner's required bytes.
    Observed::Closure {
        rows: result.unwrap().count.0,
        partition_rows: observed.closure_partition_rows,
        capacity_holds_both_peaks: observed.closure_partition_capacity >= 64,
        left,
    }
}
