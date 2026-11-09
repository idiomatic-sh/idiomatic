//! File-start declarations are enforced through every public execution route.
use super::*;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::PreparedExecution;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution::inbound::use_cases::prepare::types::PrepareRequest;
use ryo_execution_runtime::application::file_dag::program::file_session::types::CaptureContext;
use ryo_execution::inbound::use_cases::execution::interface::Demand;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_relations::domain::relations::types::{Words, WorkerNumber};
use ryo_values::domain::runs::types::ApplicabilityRefusal;
use ryo_execution::inbound::use_cases::execution::interface::{
    Partial, PreparedExecutionProgram, RequestExecution,
};

pub(crate) fn input<'a, 'facts>(
    intents: &'a [FileRuleIntent],
    elements: &'a ryo_language::domain::elements::types::Elements,
    read: &'a dyn Fn(Relation) -> Result<SourceRows<'facts>, OperatingError>,
) -> FileInput<'a, 'facts> {
    FileInput {
        language: SourceLanguage::Recognized,
        resident_stability: ResidentStability::Stable,
        file: FileId(0),
        evaluation: &[],
        publication: &[],
        intents,
        source: ryo_values::domain::files::types::SourceView(b"abc"),
        elements,
        read,
        semantic: ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Absent,
        kinds: ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport::Absent,
    }
}

/// `relation`, read in part in file 0 with any of its rows possibly missing.
pub(crate) fn partial(relation: Relation) -> Partial {
    Partial {
        relation,
        cause: ryo_values::domain::stored_facts::types::PartialCause("a reader stopped".into()),
        lacking: ryo_execution::inbound::use_cases::metadata::interface::Lacking::Any,
        at: ryo_execution::inbound::use_cases::metadata::interface::PartialPlace::File(FileId(0)),
    }
}

/// What rule 0 of `program` answered in file 0, run alone with its diagnostic collected, whose `read` gives each relation its
/// table in `rows`, with the partial reads `evaluation` (what the rule's evaluation sees) and `publication` (what its answer is
/// published under); and how many reads the file made.
pub(crate) fn answered(
    program: &PreparedExecutionProgram,
    rows: &[(Relation, Table)],
    evaluation: &[Partial],
    publication: &[Partial],
) -> (Result<RootOutcome, OperatingError>, usize) {
    let intents = [FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::Disabled,
    }];
    answered_rule(program, rows, (evaluation, publication), &intents, RuleIndex(0))
}

/// What `rule` of `program` answered in file 0 opened with `intents`, as [`answered`] says it.
pub(crate) fn answered_rule(
    program: &PreparedExecutionProgram,
    rows: &[(Relation, Table)],
    (evaluation, publication): (&[Partial], &[Partial]),
    intents: &[FileRuleIntent],
    rule: RuleIndex,
) -> (Result<RootOutcome, OperatingError>, usize) {
    let corpus = crate::boundary_fixture::corpus();
    let request = ExecutionUseCaseImpl::new().execution(program, &corpus, None);
    let reads = Cell::new(0);
    let read = |relation| {
        reads.set(reads.get() + 1);
        rows.iter()
            .find(|(each, _)| *each == relation)
            .map(|(_, table)| SourceRows::Computed(table))
            .ok_or(OperatingError::NotRead)
    };
    let elements = ryo_language::domain::elements::types::Elements::default();
    let mut symbols = Symbols::default();
    let mut file = request
        .open_file(
            FileInput {
                evaluation,
                publication,
                ..input(intents, &elements, &read)
            },
            &mut symbols,
        )
        .expect("open file 0");
    let outcome = file.rule(rule, demand(), &mut symbols).map(|outcome| outcome.emit);
    let _finished = file.finish();
    request.finish();
    (outcome, reads.get())
}

/// One rule's run through the one-call file route: its emitted root when it completed, the root as text, and how its run went on.
pub(crate) struct Ran {
    pub(crate) complete: Option<Table>,
    pub(crate) emitted: String,
    pub(crate) after: String,
}

/// File `file` run through `request`'s one-call route with `intents`, over `worker`'s facts and the tree `elements`: each rule's
/// run, then how the file ended.
pub(crate) fn ran_file(
    request: &RequestExecution,
    file: FileId,
    intents: &[FileRuleIntent],
    (elements, symbols): (&ryo_language::domain::elements::types::Elements, &mut Symbols),
    worker: &dyn ryo_execution_runtime::outbound::facts::interface::FactsWorker,
) -> (Vec<Ran>, String) {
    use ryo_execution::inbound::use_cases::execution::interface::{
        EmittedRoot, FileEnd, FileRunInput, RuleAfter,
    };
    let first = vec![ryo_values::domain::stored_facts::types::Bucket(0); intents.len()];
    let (mut arena, mut places, mut words) = (Vec::new(), Vec::new(), Words::default());
    let rules = request.run_file(
        FileRunInput {
            language: SourceLanguage::Recognized,
            file,
            evaluation: &[],
            publication: &[],
            intents,
            source: ryo_values::domain::files::types::SourceView(b"abc"),
            elements,
            semantic: ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Absent,
            kinds: ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport::Absent,
            first: &first,
        },
        symbols,
        CaptureContext {
            worker: WorkerNumber(0),
            arena: ryo_execution::inbound::use_cases::execution::interface::Arena(&mut arena),
            places: &mut places,
            words: &mut words,
        },
        worker,
    );
    let ran = rules
        .rules
        .into_iter()
        .map(|ran| {
            let (complete, emitted) = match ran.emit {
                EmittedRoot::Delivered(RootOutcome::Complete(table)) => {
                    (Some(table.clone()), format!("complete {table:?}"))
                }
                EmittedRoot::Delivered(RootOutcome::Partial { table, affected }) => {
                    (None, format!("partial {table:?} {affected:?}"))
                }
                EmittedRoot::Delivered(RootOutcome::Failed(error)) => (None, format!("failed {error:?}")),
                EmittedRoot::Delivered(RootOutcome::Unknown(error)) => (None, format!("unknown {error:?}")),
                EmittedRoot::Delivered(RootOutcome::Deferred { phase }) => (None, format!("deferred {phase:?}")),
                EmittedRoot::Delivered(RootOutcome::Skipped(_)) => (None, "skipped".into()),
                EmittedRoot::Delivered(RootOutcome::NoQuery) => (None, "no query".into()),
                EmittedRoot::Withheld => (None, "withheld".into()),
            };
            let after = match ran.after {
                RuleAfter::Ran { captures } => format!("ran, {} captures", captures.len()),
                RuleAfter::Unreached => "unreached".into(),
                RuleAfter::Stopped(stop) => format!("stopped {stop:?}"),
            };
            Ran {
                complete,
                emitted,
                after,
            }
        })
        .collect();
    let end = match rules.end {
        FileEnd::Finished(_) => "finished".to_owned(),
        FileEnd::Unfinished => "unfinished".to_owned(),
        FileEnd::Refused(stop) => format!("refused {stop:?}"),
    };
    (ran, end)
}

#[test]
fn every_prepared_rule_requires_exactly_one_file_start_intent() {
    let engine = engine();
    let program = prepared_rules(&engine, 2);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let read = |_| panic!("malformed intent vectors cannot read a provider");
    let mut symbols = Symbols::default();
    for intents in [
        &[][..],
        &[FileRuleIntent::Undemanded][..],
        &[FileRuleIntent::Undemanded; 3][..],
    ] {
        assert!(matches!(
            request.open_file(input(intents, &elements, &read), &mut symbols),
            Err(OperatingError::ShapeMismatch)
        ));
    }
    request.finish();
}

#[test]
fn absent_skipped_and_refused_rules_cannot_acquire_any_root() {
    let engine = engine();
    let program = prepared(&engine);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let read = |_| panic!("undeclared roots cannot read a provider");
    for intent in [
        FileRuleIntent::Undemanded,
        FileRuleIntent::Skipped(ApplicabilityReason::SyntaxRejected),
        FileRuleIntent::Refused(ApplicabilityRefusal::SyntaxUndetermined),
        FileRuleIntent::Refused(ApplicabilityRefusal::MetadataUnavailable),
    ] {
        let intents = [intent];
        let mut symbols = Symbols::default();
        let mut file = request
            .open_file(input(&intents, &elements, &read), &mut symbols)
            .unwrap();
        assert!(matches!(
            file.rule(RuleIndex(0), demand(), &mut symbols),
            Err(OperatingError::NotRun)
        ));
        assert!(matches!(
            file.emit_rows(RuleIndex(0), &mut symbols, &mut |_| panic!("no delivery")),
            Err(OperatingError::NotRun)
        ));
        let (mut arena, mut places, mut words) = (vec![], vec![], Words::default());
        assert!(matches!(
            file.capture_frontiers(
                RuleIndex(0),
                ryo_values::domain::stored_facts::types::Bucket(0),
                &mut symbols,
                CaptureContext {
                    worker: WorkerNumber(0),
                    arena: ryo_execution::inbound::use_cases::execution::interface::Arena(
                        &mut arena
                    ),
                    places: &mut places,
                    words: &mut words
                },
            ),
            Err(OperatingError::NotRun)
        ));
        assert!(matches!(
            file.commit_frontier(
                RuleIndex(0),
                ryo_values::domain::plan_parts::types::InputNumber(0)
            ),
            Err(OperatingError::NotRun)
        ));
        file.finish().unwrap();
    }
    request.finish();
}

#[test]
fn rule_demand_is_immutable_and_capture_cannot_be_added_after_open() {
    let engine = engine();
    let program = prepared(&engine);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let reads = Cell::new(0);
    let table = source();
    let read = |relation| {
        assert_eq!(relation, Relation::Function);
        reads.set(reads.get() + 1);
        Ok(SourceRows::Computed(&table))
    };
    let intents = [FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::Disabled,
    }];
    let mut symbols = Symbols::default();
    let mut file = request
        .open_file(input(&intents, &elements, &read), &mut symbols)
        .unwrap();
    for changed in [
        RuleDemand {
            emit: EmitDemand::Disabled,
            observations: ObservationDemand::NotRequired,
        },
        RuleDemand {
            emit: EmitDemand::Enabled,
            observations: ObservationDemand::Required,
        },
    ] {
        assert!(matches!(
            file.rule(RuleIndex(0), changed, &mut symbols),
            Err(OperatingError::NotRun)
        ));
    }
    let (mut arena, mut places, mut words) = (vec![], vec![], Words::default());
    assert!(matches!(
        file.capture_frontiers(
            RuleIndex(0),
            ryo_values::domain::stored_facts::types::Bucket(0),
            &mut symbols,
            CaptureContext {
                worker: WorkerNumber(0),
                arena: ryo_execution::inbound::use_cases::execution::interface::Arena(&mut arena),
                places: &mut places,
                words: &mut words
            },
        ),
        Err(OperatingError::NotRun)
    ));
    assert_eq!(reads.get(), 0, "refused root changes do not admit a source");
    assert!(matches!(
        file.rule(RuleIndex(0), demand(), &mut symbols)
            .unwrap()
            .emit,
        RootOutcome::Complete(_)
    ));
    assert!(reads.get() > 0);
    file.finish().unwrap();
    request.finish();
}

pub(crate) fn named_request() -> PrepareRequest {
    use ryo_values::domain::observations::types::{
        BoundObservationInput, ObservationBinding, ObservationBindingName, ObservationColumn,
        ObservationColumnName, ObservationName, ObservationProducer, PlanObservation,
    };
    use ryo_values::domain::plan_parts::types::ObservationBindingId;
    use ryo_values::domain::records::types::ObservationReadinessLayout;
    let mut authored = authored_request(2);
    let columns = vec![ObservationColumn {
        name: ObservationColumnName("node".into()),
        kind: CellKind::Node,
    }];
    let producer = &mut crate::boundary_fixture::plans(&mut authored)[0];
    producer.steps.push(PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Select {
            from: Input::Relation(Relation::Identifier),
            comparisons: vec![],
            kept: vec![ColumnIndex(0)],
        },
    });
    producer.steps.push(PlanStep {
        scope: StepScope::CrossFile,
        operation: Operation::Select {
            from: Input::Step(StepId(1)),
            comparisons: vec![],
            kept: vec![],
        },
    });
    producer.emit.step = StepId(2);
    producer.frontier = vec![Input::Step(StepId(0)), Input::Step(StepId(1))];
    producer.observations.push(PlanObservation {
        name: ObservationName("nodes".into()),
        source: Input::Step(StepId(0)),
        columns: columns.clone(),
        scope: StepScope::PerFile,
    });
    crate::boundary_fixture::plans(&mut authored)[1].steps[0].scope = StepScope::CrossFile;
    crate::boundary_fixture::plans(&mut authored)[1].steps[0].operation = Operation::Select {
        from: Input::BoundObservation(ObservationBindingId(0)),
        comparisons: vec![],
        kept: vec![],
    };
    crate::boundary_fixture::plans(&mut authored)[1]
        .observation_inputs
        .push(BoundObservationInput {
            name: ObservationBindingName("input".into()),
            columns: columns.clone(),
            scope: StepScope::PerFile,
        });
    let producer_id = authored.rules[0].id.clone();
    authored.rules[1].observations.push(ObservationBinding {
        name: ObservationBindingName("input".into()),
        columns,
        producers: vec![ObservationProducer::Output {
            rule: producer_id,
            output: ObservationName("nodes".into()),
        }],
    });
    authored.observations =
        ObservationIdentity::Named(ObservationReadinessLayout(Digest([91; 16])));
    authored
}

mod delivery;
