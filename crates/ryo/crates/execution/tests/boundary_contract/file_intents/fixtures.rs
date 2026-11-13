//! The support of the `file_intents` cases and of the groups nested in it.
use super::super::*;
use ryo_execution::inbound::use_cases::execution::interface::{
    Partial, PreparedExecutionProgram, RequestExecution,
};

/// `relation`, read in part in file 0 with any of its rows possibly missing.
pub(crate) fn partial(relation: Relation) -> Partial {
    Partial {
        relation,
        cause: ryo_values::domain::stored_facts::types::PartialCause("a reader stopped".into()),
        lacking: ryo_execution::inbound::use_cases::metadata::interface::Lacking::Any,
        at: ryo_execution::inbound::use_cases::metadata::interface::PartialPlace::File(FileId(0)),
    }
}

/// What rule 0 of `program` answered in file 0, run alone with its diagnostic collected, over facts that hold each relation's
/// table in `rows`, with the partial reads `evaluation` (what the rule's evaluation sees) and `publication` (what its answer is
/// published under); and how many reads the file made.
pub(crate) fn answered(
    program: &PreparedExecutionProgram,
    rows: &[(Relation, Table)],
    evaluation: &[Partial],
    publication: &[Partial],
) -> (Result<RootOutcome, FileStop>, usize) {
    let intents = [FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::DiagnosticAndObservations,
    }];
    answered_rule(
        program,
        rows,
        (evaluation, publication),
        &intents,
        RuleIndex(0),
    )
}

/// What `rule` of `program` answered in file 0 run through `run_file` with `intents`, as [`answered`] says it: its emitted root,
/// or what stopped it.
pub(crate) fn answered_rule(
    program: &PreparedExecutionProgram,
    rows: &[(Relation, Table)],
    (evaluation, publication): (&[Partial], &[Partial]),
    intents: &[FileRuleIntent],
    rule: RuleIndex,
) -> (Result<RootOutcome, FileStop>, usize) {
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(program, &corpus, None.into());
    let mut facts = TableFacts::new(
        &rows
            .iter()
            .map(|(relation, table)| (*relation, table))
            .collect::<Vec<_>>(),
    );
    let elements = ryo_language::domain::elements::types::Elements::default();
    let mut symbols = Symbols::default();
    let mut rules = run_file(
        &request,
        FileRun {
            evaluation,
            publication,
            ..FileRun::of(intents, &elements)
        },
        &mut symbols,
        &mut facts,
    );
    let outcome = match rules.rules.iter().any(|each| each.rule == rule) {
        true => root(ran(&mut rules, rule.0)),
        false => Err(match rules.end {
            FileEnd::Refused(stop) => stop,
            FileEnd::Finished(_) | FileEnd::Unfinished => {
                FileStop::Operating(OperatingError::NotRun)
            }
        }),
    };
    request.finish();
    (outcome, facts.reads())
}

/// One rule's run through the one-call file route: its emitted root when it completed, the root as text, and how its run went on.
pub(crate) struct Ran {
    /// The rule that ran.
    pub(crate) rule: RuleIndex,
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
    (elements, symbols): (
        &ryo_language::domain::elements::types::Elements,
        &mut Symbols,
    ),
    worker: &mut dyn ryo_execution::inbound::use_cases::execution::interface::FactsWorker,
) -> (Vec<Ran>, String) {
    shown(run_file(
        request,
        FileRun {
            file,
            ..FileRun::of(intents, elements)
        },
        symbols,
        worker,
    ))
}

/// What the rules of a file run came to, each rule's run as text, then how the file ended.
pub(crate) fn shown(
    rules: ryo_execution::inbound::use_cases::execution::interface::FileRules,
) -> (Vec<Ran>, String) {
    use ryo_execution::inbound::use_cases::execution::interface::{EmittedRoot, RuleAfter};
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
                EmittedRoot::Delivered(RootOutcome::Failed(error)) => {
                    (None, format!("failed {error:?}"))
                }
                EmittedRoot::Delivered(RootOutcome::Unknown(error)) => {
                    (None, format!("unknown {error:?}"))
                }
                EmittedRoot::Delivered(RootOutcome::Deferred { phase }) => {
                    (None, format!("deferred {phase:?}"))
                }
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
                rule: ran.rule,
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
    let producer = &mut boundary_fixture::fixtures::plans(&mut authored)[0];
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
    boundary_fixture::fixtures::plans(&mut authored)[1].steps[0].scope = StepScope::CrossFile;
    boundary_fixture::fixtures::plans(&mut authored)[1].steps[0].operation = Operation::Select {
        from: Input::BoundObservation(ObservationBindingId(0)),
        comparisons: vec![],
        kept: vec![],
    };
    boundary_fixture::fixtures::plans(&mut authored)[1]
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
