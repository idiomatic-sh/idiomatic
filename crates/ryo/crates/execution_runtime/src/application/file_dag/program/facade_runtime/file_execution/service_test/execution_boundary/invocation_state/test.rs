//! Chained registered evaluations reuse downstream invocation state and release their peaks.
use super::super::super::super::service::*;
use super::super::super::fixtures::*;
use super::super::super::scratch_owner::fixtures::{execute, hosted};
use super::fixtures::MemoSink;
use crate::application::file_dag::execution::service::CommonOperatorEngine;
use crate::application::file_dag::program::facade_runtime::service::ProgramServices;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::CaptureIntent;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::DiagnosticDelivery;
use ryo_execution_domain::domain::rule_intent::types::FileRuleIntent;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::ObservationDemand;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_execution_domain::domain::{
    observation_schedule::types::RuleSchedule,
    prepared_request::types::{Compiled, ObservationIdentity},
};
use ryo_execution_store::application::file_dag::relations::types::ReuseEligibility;
use ryo_execution_store::application::file_dag::service::ScopePartials;
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
use std::sync::Arc;
fn chained_registered_evaluations_reuse_downstream_invocation_state() {
    let mut plan = plan(Scenario::ProjectRetry);
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
    assert_eq!(sink.starts, [0, 1, 2]);
    assert_eq!(
        sink.rows,
        (1..=3)
            .map(|number| Cell::Number(Integer(number)))
            .collect::<Vec<_>>()
    );
    file.finish().unwrap();
}
fn partition_observations_keep_large_then_small_peak_and_release() {
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
    let (result, observed) = execute(
        Operation::Closure {
            edges: PlanInput::Step(StepId(0)),
            keys: vec![ColumnIndex(2)],
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
        },
        hosted(&[HostKind::Number; 3], rows),
        &mut Symbols::default(),
    );
    assert_eq!(result.unwrap().count.0, 32 * 33 / 2 + 1);
    assert_eq!(observed.closure_partition_rows, 32);
    assert!(observed.closure_partition_capacity >= 64);
    // execute calls normal file.finish and checks the real owner's required bytes are zero.
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Proof {
    ChainedRegisteredEvaluationsReuseDownstreamInvocationState,
    PartitionObservationsKeepLargeThenSmallPeakAndRelease,
}

type Input = Proof;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("chained_registered_evaluations_reuse_downstream_invocation_state"),
        word: Word::Succeeds,
        input: || Proof::ChainedRegisteredEvaluationsReuseDownstreamInvocationState,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("partition_observations_keep_large_then_small_peak_and_release"),
        word: Word::Succeeds,
        input: || Proof::PartitionObservationsKeepLargeThenSmallPeakAndRelease,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Proof::ChainedRegisteredEvaluationsReuseDownstreamInvocationState => {
            chained_registered_evaluations_reuse_downstream_invocation_state();
            Ok(())
        }
        Proof::PartitionObservationsKeepLargeThenSmallPeakAndRelease => {
            partition_observations_keep_large_then_small_peak_and_release();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
