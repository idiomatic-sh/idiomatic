//! Real request compilation and file opening must not allocate dormant siblings.
use super::super::super::service::*;
use super::super::fixtures::*;
use crate::application::file_dag::execution::service::CommonOperatorEngine;
use crate::application::file_dag::program::facade_runtime::service::ProgramServices;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::prepared_request::types::ObservationIdentity;
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
use ryo_execution_store::application::file_dag::relations::types::ReuseEligibility;
use ryo_execution_store::application::file_dag::service::ScopePartials;
use ryo_language::domain::{files::types::FilePath, symbols::types::Symbols};
use ryo_relations::domain::relations::{
    types::{CellKind, FileId, Integer, Relation},
};
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    plan_parts::types::{ColumnIndex, Input as PlanInput, RepeatedRows, StepId, StepScope},
    plans::types::{Operation, PlanStep},
    runs::types::RuleId,
    stored_facts::types::Digest,
};
use ryo_values::domain::{
    observations::types::{
        BoundObservationInput, ObservationBinding, ObservationBindingName, ObservationColumn,
        ObservationColumnName, ObservationName, ObservationProducer, PlanObservation,
    },
    plan_parts::types::{ObservationBindingId, Operand, QueryLiteral},
    records::types::ObservationReadinessLayout,
};
use ryo_execution_domain::domain::executable_program::file_demand::constants::NO_FILE_DEMAND;
use ryo_execution_domain::domain::executable_program::file_demand::types::{
    FileDemand, FrontierSelection, RootSelection,
};
use std::borrow::Cow;
use std::sync::Arc;

/// The request: how many producers feed the consumer's observation binding.
#[derive(Clone, Copy, Debug)]
struct Request {
    producers: usize,
}

/// The first producer's file demand under no demand, full demand and observation-only capture:
/// the readers of no and full demand, the roots of full and observation-only demand, and the
/// observations observation-only capture selects.
#[derive(Debug, PartialEq)]
struct Selected {
    readers: [usize; 2],
    roots: [usize; 2],
    observing_observations: usize,
}

/// The opened file's state before any rule runs.
#[derive(Debug, PartialEq)]
struct Opened {
    roots: usize,
    prepared_named: usize,
    prepared_roots: usize,
    slots: usize,
    dormant_obligations: usize,
}

/// The disabled sibling's refusal and its reads, every relation read, the demanded rule's
/// diagnostic rows and observation, and the slots and evaluations after it ran.
#[derive(Debug, PartialEq)]
struct Ran {
    sibling: Option<OperatingError>,
    sibling_reads: usize,
    reads: Cow<'static, [Relation]>,
    /// Whether the diagnostic rows are the node column of the function rows read.
    emits_the_read_nodes: bool,
    observation_complete: bool,
    slots: usize,
    evaluations: u64,
}

#[derive(Debug, PartialEq)]
struct Prepared {
    selected: Selected,
    opened: Opened,
    ran: Ran,
}

type Input = Request;
type Value = Prepared;
type Failure = OperatingError;

/// What opening and running the first of any number of producers observes.
const PREPARED: Prepared = Prepared {
    selected: Selected {
        readers: [0, 3],
        roots: [3, 1],
        observing_observations: 0,
    },
    opened: Opened {
        roots: 1,
        prepared_named: 1,
        prepared_roots: 1,
        slots: 0,
        dormant_obligations: 0,
    },
    ran: Ran {
        sibling: Some(OperatingError::NotRun),
        sibling_reads: 0,
        reads: Cow::Borrowed(&[Relation::Function; 4]),
        emits_the_read_nodes: true,
        observation_complete: true,
        slots: 2,
        evaluations: 2,
    },
};

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "opening_and_preparing_one_rule_uses_only_its_reached_metadata_and_preserves_aliases",
        ),
        word: Word::Succeeds,
        input: || Request { producers: 128 },
        expect: Expect::Value(PREPARED),
    },
    Case {
        name: CaseName("two_producers_open_the_same_reached_metadata_as_many"),
        word: Word::Succeeds,
        input: || Request { producers: 2 },
        expect: Expect::Value(PREPARED),
    },
];

/// Prepares the request, opens one file demanding the first producer only, and runs a disabled
/// sibling and the demanded rule.
fn subject(request: Input) -> Result<Value, Failure> {
    let columns = vec![ObservationColumn {
        name: ObservationColumnName("node".into()),
        kind: CellKind::Node,
    }];
    let mut rules = Vec::new();
    let mut plans = Vec::new();
    for literal in 0..request.producers {
        let mut authored = rule();
        authored.id = RuleId(format!("opening-{literal}").into());
        rules.push(authored);
        let mut producer = plan(Scenario::ProjectRetry);
        producer.steps = vec![
            producer.steps[0].clone(),
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Extend {
                    from: PlanInput::Step(StepId(0)),
                    values: vec![Operand::Literal(QueryLiteral::Number(Integer(
                        literal as i64,
                    )))],
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
        producer.emit.step = StepId(2);
        producer.observations = vec![PlanObservation {
            name: ObservationName("nodes".into()),
            source: PlanInput::Step(StepId(2)),
            columns: columns.clone(),
            scope: StepScope::PerFile,
        }];
        plans.push(producer);
    }
    let mut consumer = rule();
    consumer.id = RuleId("opening-consumer".into());
    consumer.observations = vec![ObservationBinding {
        name: ObservationBindingName("input".into()),
        columns: columns.clone(),
        producers: rules
            .iter()
            .map(|rule| ObservationProducer::Output {
                rule: rule.id.clone(),
                output: ObservationName("nodes".into()),
            })
            .collect(),
    }];
    rules.push(consumer);
    let mut consumer_plan = plans[0].clone();
    consumer_plan.steps = vec![PlanStep {
        scope: StepScope::CrossFile,
        operation: Operation::Select {
            from: PlanInput::BoundObservation(ObservationBindingId(0)),
            comparisons: vec![],
            kept: vec![],
        },
    }];
    consumer_plan.observations.clear();
    consumer_plan.observation_inputs = vec![BoundObservationInput {
        name: ObservationBindingName("input".into()),
        columns,
        scope: StepScope::PerFile,
    }];
    consumer_plan.emit.step = StepId(0);
    plans.push(consumer_plan);
    let compiled = ryo_execution_domain::domain::prepared_request::service::assemble(
        plans,
        vec![vec![]; rules.len()],
        &rules,
    )?;
    let derived = ryo_execution_store::application::file_dag::fixtures::program_derived_port();
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
        compiled,
        &rules,
        &vec![Digest([0; 16]); rules.len()],
        &[],
        Arc::clone(&derived),
        ObservationIdentity::Named(ObservationReadinessLayout(Digest([33; 16]))),
    )?;
    let roots = prepared
        .file_demand(RuleIndex(0))?;
    let none = roots.selected(NO_FILE_DEMAND);
    let full = roots.selected(FileDemand {
        diagnostic: RootSelection::Required,
        observations: RootSelection::Required,
        capture: FrontierSelection::All,
    });
    let observing = roots.selected(FileDemand {
        capture: FrontierSelection::Observations,
        ..NO_FILE_DEMAND
    });
    let selected = Selected {
        readers: [none.readers.len(), full.readers.len()],
        roots: [full.roots.len(), observing.roots.len()],
        observing_observations: observing.observations.len(),
    };
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(Arc::new(
            crate::application::file_dag::fixtures::operators(),
        ))),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("opening.py".into()),
            size: FileSize(0),
            modified: Nanoseconds(0),
        })],
    ));
    let mut intents = vec![FileRuleIntent::Undemanded; rules.len()];
    intents[0] = FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::Required,
        capture: CaptureIntent::DiagnosticAndObservations,
    };
    let mut symbols = Symbols::default();
    let mut file = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: &[],
            publication: &[],
        },
        &intents,
        services,
        ryo_values::domain::files::types::SourceView(b""),
        ReuseEligibility::Eligible,
        &mut symbols,
        None,
        None,
    )?;
    let opened = Opened {
        roots: file.roots.len(),
        prepared_named: file.prepared_named.len(),
        prepared_roots: file.prepared_roots.len(),
        slots: file.execution.state_counts().slots.0,
        dormant_obligations: file.execution.state_counts().dormant_obligations.0,
    };
    let rows = function_rows(&[2]);
    let reads = std::cell::RefCell::new(Vec::new());
    let read = |relation| {
        reads.borrow_mut().push(relation);
        Ok(Rows::Computed(&rows))
    };
    let sibling = file
        .rule(
            RuleIndex(1),
            RuleDemand {
                emit: EmitDemand::Enabled,
                observations: ObservationDemand::Required,
            },
            &read,
            &mut symbols,
        )
        .err();
    let sibling_reads = reads.borrow().len();
    let outcome = file
        .rule(
            RuleIndex(0),
            RuleDemand {
                emit: EmitDemand::Enabled,
                observations: ObservationDemand::Required,
            },
            &read,
            &mut symbols,
        )?;
    let RootOutcome::Complete(actual) = outcome.emit else {
        return Err(OperatingError::NotRun);
    };
    let ran = Ran {
        sibling,
        sibling_reads,
        reads: reads.borrow().clone().into(),
        emits_the_read_nodes: actual.cells
            == rows
                .cells
                .chunks_exact(rows.kinds.len())
                .map(|row| row[0])
                .collect::<Vec<_>>(),
        observation_complete: matches!(outcome.observations[0].outcome, RootOutcome::Complete(_)),
        slots: file.execution.state_counts().slots.0,
        evaluations: file.readiness_counts().evaluations.0,
    };
    // The finished file's frontier readiness is not this case's observation.
    let _readiness = file.finish()?;
    Ok(Prepared {
        selected,
        opened,
        ran,
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
