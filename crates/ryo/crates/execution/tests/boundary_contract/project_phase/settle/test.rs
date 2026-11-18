//! The rows a difference after the files keeps where a partial operand or a file skipped for the rule could change them: each
//! occurrence answers from its own partials, whichever occurrence of the rule came first, and a file the rule skipped withholds
//! the rows it keys.
use crate::boundary_fixture::fixtures::{authored_request, corpus, engine, plans};
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::Partials;
use ryo_execution::inbound::use_cases::execution::interface::PhasePartials;
use ryo_execution::inbound::use_cases::execution::interface::PhaseRules;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::interface::ProjectBatch;
use ryo_execution::inbound::use_cases::execution::interface::ProjectRootExecution;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::interface::{LogicalCounts, OperatingError};
use ryo_execution::inbound::use_cases::execution::interface::{ObserverPort, Observing};
use ryo_execution::inbound::use_cases::execution::interface::{PhaseStop, RootOutcome};
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::metadata::interface::{
    FilePartial, Lacking, Partial, PartialPlace,
};
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution_runtime::outbound::facts::fixtures::project_phase_source_facts::ProjectFacts;
use ryo_execution_runtime::outbound::facts::fixtures::project_phase_source_facts::RefusingKept as Kept;
use ryo_execution_runtime::outbound::observer::types::FedTotals;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{FileId, Relation};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as StepInput, StepId, StepScope};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::runs::types::RuleApplicability;
use ryo_values::domain::stored_facts::types::{Bucket, PartialCause};
use std::borrow::Cow;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The partials of each occurrence of one rule whose emit reads the files of the project's module identities less none of them.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// Two occurrences: the first reads the module identities in part in the file of their row, the second whole.
    PartialThenComplete,
    /// Two occurrences: the first reads them whole, the second in part.
    CompleteThenPartial,
    /// One occurrence, which skipped the file of their row.
    Skipped,
    /// Two occurrences, the first in part and the second whole, an observer refusing the first occurrence's counts.
    RefusedCountsThenComplete,
}

/// What one occurrence's partials say of the file of the project's row.
#[derive(Clone, Copy)]
enum Said {
    /// Nothing.
    Whole,
    /// Its module identities are read in part.
    InPart,
    /// The rule skipped it.
    Skipped,
}

type Input = Scenario;
/// What each occurrence answered and how often the phase read the project, as text.
type Value = Cow<'static, str>;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_partial_occurrence_does_not_narrow_a_later_complete_one"),
        word: Word::Succeeds,
        input: || Scenario::PartialThenComplete,
        expect: Expect::Value(Cow::Borrowed(
            "complete [partial RowCount(0) [Some(FileId(0))], complete RowCount(1)]; project reads 1",
        )),
    },
    Case {
        name: CaseName("a_complete_occurrence_does_not_widen_a_later_partial_one"),
        word: Word::Succeeds,
        input: || Scenario::CompleteThenPartial,
        expect: Expect::Value(Cow::Borrowed(
            "complete [complete RowCount(1), partial RowCount(0) [Some(FileId(0))]]; project reads 1",
        )),
    },
    Case {
        name: CaseName("a_file_the_rule_skipped_withholds_the_rows_a_difference_keys_by_it"),
        word: Word::Succeeds,
        input: || Scenario::Skipped,
        expect: Expect::Value(Cow::Borrowed(
            "complete [complete RowCount(0)]; project reads 1",
        )),
    },
    Case {
        name: CaseName("an_occurrence_that_fails_leaves_no_narrowed_rows_for_the_next"),
        word: Word::Succeeds,
        input: || Scenario::RefusedCountsThenComplete,
        // The first occurrence stops where the observer refuses its counts; the second, whole, answers every row, as it does
        // after a partial occurrence that succeeded.
        expect: Expect::Value(Cow::Borrowed(
            "complete [stopped Operating(NotRun), complete RowCount(1)]; project reads 1",
        )),
    },
];

/// An observer that refuses the first counts it is told and keeps none: the first occurrence it observes fails.
#[derive(Default)]
struct RefusesFirst {
    /// How many counts it was told.
    told: AtomicUsize,
}

impl ObserverPort for RefusesFirst {
    fn fed(&self, _: RuleIndex, _: LogicalCounts) -> Result<(), OperatingError> {
        if self.told.fetch_add(1, Ordering::SeqCst) == 0 {
            Err(OperatingError::NotRun)
        } else {
            Ok(())
        }
    }
    fn totals(&self) -> Result<FedTotals, OperatingError> {
        Ok(FedTotals(Vec::new()))
    }
    fn handed(
        &self,
        _: ryo_values::domain::runs::types::FileHandover,
    ) -> Result<(), OperatingError> {
        Ok(())
    }
    fn handovers(
        &self,
    ) -> Result<ryo_execution_runtime::outbound::observer::types::FileHandovers, OperatingError>
    {
        Ok(ryo_execution_runtime::outbound::observer::types::FileHandovers::default())
    }
}

/// A step after the files.
fn after_files(operation: Operation) -> PlanStep {
    PlanStep {
        scope: StepScope::CrossFile,
        operation,
    }
}

/// The partials an occurrence takes when its partials say `said` of the file of the project's row.
fn partials(said: Said) -> Partials {
    let file = PartialPlace::File(FileId(0));
    match said {
        Said::Whole => Partials::default(),
        Said::InPart => Partials {
            entries: vec![FilePartial {
                file,
                partial: Partial {
                    relation: Relation::ModuleIdentity,
                    cause: PartialCause("missing project partition".into()),
                    lacking: Lacking::Any,
                    at: file,
                },
            }],
            unavailable_files: vec![],
        },
        Said::Skipped => Partials {
            entries: vec![],
            unavailable_files: vec![FileId(0)],
        },
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let mut authored = authored_request(1);
    authored.rules[0].applicability = RuleApplicability::AcceptedSyntax;
    let plan = &mut plans(&mut authored)[0];
    // The files of the module identities, those with none of them, the files less those, and what a later step reads of that:
    // every file of them.
    plan.steps = vec![
        after_files(Operation::Select {
            from: StepInput::Relation(Relation::ModuleIdentity),
            comparisons: vec![],
            kept: vec![ColumnIndex(0)],
        }),
        after_files(Operation::Antijoin {
            left: StepInput::Step(StepId(0)),
            right: StepInput::Step(StepId(0)),
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
        }),
        after_files(Operation::Difference {
            left: StepInput::Step(StepId(0)),
            right: StepInput::Step(StepId(1)),
        }),
        after_files(Operation::Select {
            from: StepInput::Step(StepId(2)),
            comparisons: vec![],
            kept: vec![ColumnIndex(0)],
        }),
    ];
    // The project hands its module identities over.
    plan.frontier = vec![StepInput::Relation(Relation::ModuleIdentity)];
    plan.emit.step = StepId(3);
    let said: &[Said] = match scenario {
        Scenario::PartialThenComplete => &[Said::InPart, Said::Whole],
        Scenario::CompleteThenPartial => &[Said::Whole, Said::InPart],
        Scenario::Skipped => &[Said::Skipped],
        Scenario::RefusedCountsThenComplete => &[Said::InPart, Said::Whole],
    };
    let observing: Observing<dyn ObserverPort> = match scenario {
        Scenario::RefusedCountsThenComplete => {
            Observing::Observer(std::sync::Arc::new(RefusesFirst::default()))
        }
        Scenario::PartialThenComplete | Scenario::CompleteThenPartial | Scenario::Skipped => {
            Observing::Unobserved
        }
    };
    let prepared = engine().prepare(authored).map_err(|_| ())?;
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, observing);
    let stored = Kept {
        refused: None,
        reads: AtomicUsize::new(0),
    };
    let project = ProjectFacts::new(Relation::ModuleIdentity);
    let project_symbols = Symbols::default();
    let occurrences = vec![RuleIndex(0); said.len()];
    let inapplicable =
        vec![ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles::default()];
    let refusals =
        vec![ryo_execution::inbound::use_cases::execution::interface::RefusedFiles::default()];
    let phase = request.project_phase().map_err(|_| ())?;
    let mut symbols = Symbols::default();
    let batch = phase.evaluate_rules(
        PhaseRules {
            rules: &occurrences,
            first: &[Bucket(0)],
            corpus: &corpus,
            covered: &[],
            skipped: &[],
            inapplicable: &inapplicable,
            refused: &refusals,
            facts: &stored,
            project: &project,
            project_symbols: &project_symbols,
            arenas: &[],
        },
        &mut symbols,
        |_| Ok::<(), &str>(()),
        |number| {
            let each = partials(said.get(number.0).copied().unwrap_or(Said::Whole));
            Ok(PhasePartials {
                evaluation: each.clone(),
                publication: each,
            })
        },
        |_| Ok(()),
        |_, outcome, _, _, _| Ok(outcome.emit),
    );
    let answered = match batch {
        ProjectBatch::Complete(done) => format!("complete {}", described(&done)),
        ProjectBatch::Refused { completed, error } => {
            format!("refused {error:?} after {}", described(&completed))
        }
    };
    Ok(Cow::Owned(format!(
        "{answered}; project reads {}",
        project.reads.load(Ordering::SeqCst)
    )))
}

/// Each occurrence's answer, as text: its emit's kind and rows, or why the phase stopped it.
fn described(done: &[Result<RootOutcome, PhaseStop<&str>>]) -> String {
    let each = done.iter().map(|outcome| match outcome {
        Ok(RootOutcome::Complete(table)) => format!("complete {:?}", table.count),
        Ok(RootOutcome::Partial { table, affected }) => {
            format!("partial {:?} {affected:?}", table.count)
        }
        Ok(RootOutcome::Failed(error)) => format!("failed {error:?}"),
        Ok(RootOutcome::Unknown(error)) => format!("unknown {error:?}"),
        Ok(RootOutcome::Deferred { phase }) => format!("deferred {phase:?}"),
        Ok(RootOutcome::Skipped(_)) => "skipped".to_owned(),
        Ok(RootOutcome::NoQuery) => "no query".to_owned(),
        Err(stop) => format!("stopped {stop:?}"),
    });
    format!("[{}]", each.collect::<Vec<_>>().join(", "))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
