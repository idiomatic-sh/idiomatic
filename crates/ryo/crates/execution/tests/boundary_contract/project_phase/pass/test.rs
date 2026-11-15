//! The steps after the files through the project phase's one call, recorded on the interpreter they replace: an occurrence
//! its caller refuses reads neither the store nor the project; a rule's frontier is read from the store once however often
//! the rule occurs; a refused store read stops its own occurrence only, wherever its entry is read; and the project is read
//! once for the phase. Each expectation is the interpreter's answer, recorded on main at 262997fccb.
use crate::boundary_fixture::fixtures::{authored_request, corpus, engine};
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::Partials;
use ryo_execution::inbound::use_cases::execution::interface::PhasePartials;
use ryo_execution::inbound::use_cases::execution::interface::PhaseRules;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::interface::ProjectBatch;
use ryo_execution::inbound::use_cases::execution::interface::ProjectRootExecution;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::interface::{PhaseStop, RootOutcome};
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution_runtime::outbound::facts::fixtures::project_phase_source_facts::ProjectFacts;
use ryo_execution_runtime::outbound::facts::fixtures::project_phase_source_facts::{
    RefusingKept as Kept, first_entry,
};
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::fixtures::kinds;
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as StepInput, StepId, StepScope};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::runs::types::{RuleApplicability, SkipReason, Skipped};
use ryo_values::domain::stored_facts::types::Bucket;
use std::borrow::Cow;
use std::sync::atomic::{AtomicUsize, Ordering};

/// One phase over a rule whose file was skipped: its frontier keeps no rows of it.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// One occurrence, which its caller refuses before it is handed over.
    RefusedBeforeHandover,
    /// Two occurrences of the one rule.
    TwiceInOnePhase,
    /// The store refuses the frontier entry the union reads first.
    StoreRefusesTheFirstRead,
    /// The store refuses the frontier entry the union reads second.
    StoreRefusesTheSecondRead,
    /// Two rules, the store refusing the second's frontier.
    StoreRefusesTheLaterRule,
}

type Input = Scenario;
/// What the phase answered and how often it read the store and the project, as text.
type Value = Cow<'static, str>;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_occurrence_its_caller_refuses_reads_neither_the_store_nor_the_project"),
        word: Word::Succeeds,
        input: || Scenario::RefusedBeforeHandover,
        expect: Expect::Value(Cow::Borrowed(
            "complete [stopped Caller(\"refused before its handover\")]; store reads 0; project reads 0",
        )),
    },
    Case {
        name: CaseName("a_rule_s_frontier_is_read_once_however_often_the_rule_occurs"),
        word: Word::Succeeds,
        input: || Scenario::TwiceInOnePhase,
        expect: Expect::Value(Cow::Borrowed(
            "complete [unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" }, unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" }]; store reads 2; project reads 0",
        )),
    },
    Case {
        name: CaseName("a_refused_store_read_of_the_first_entry_read_stops_the_occurrence"),
        word: Word::Succeeds,
        input: || Scenario::StoreRefusesTheFirstRead,
        expect: Expect::Value(Cow::Borrowed(
            "complete [stopped Frontiers(Facts(Store { reason: \"the kept frontier is unreadable\" }))]; store reads 1; project reads 0",
        )),
    },
    Case {
        name: CaseName("a_refused_store_read_of_the_second_entry_read_stops_the_occurrence"),
        word: Word::Succeeds,
        input: || Scenario::StoreRefusesTheSecondRead,
        expect: Expect::Value(Cow::Borrowed(
            "complete [stopped Frontiers(Facts(Store { reason: \"the kept frontier is unreadable\" }))]; store reads 1; project reads 0",
        )),
    },
    Case {
        name: CaseName("a_store_refusal_stops_only_the_occurrence_of_its_rule"),
        word: Word::Succeeds,
        input: || Scenario::StoreRefusesTheLaterRule,
        expect: Expect::Value(Cow::Borrowed(
            "complete [unknown ObservationIncomplete { reason: \"every source partition was explicitly skipped\" }, stopped Frontiers(Facts(Store { reason: \"the kept frontier is unreadable\" }))]; store reads 3; project reads 0",
        )),
    },
];

/// A select in each file of the file column of the functions.
fn files_of_functions() -> PlanStep {
    let file = kinds(Relation::Function)
        .iter()
        .position(|kind| *kind == CellKind::File)
        .and_then(|at| u32::try_from(at).ok())
        .unwrap_or(u32::MAX);
    PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Select {
            from: StepInput::Relation(Relation::Function),
            comparisons: vec![],
            kept: vec![ColumnIndex(file)],
        },
    }
}

/// A union after the files of the steps `left` and `right`.
fn united(left: u32, right: u32) -> PlanStep {
    PlanStep {
        scope: StepScope::CrossFile,
        operation: Operation::Union {
            left: StepInput::Step(StepId(left)),
            right: StepInput::Step(StepId(right)),
        },
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let rules = if matches!(scenario, Scenario::StoreRefusesTheLaterRule) {
        2
    } else {
        1
    };
    let mut authored = authored_request(rules);
    for rule in 0..rules {
        authored.rules[rule].applicability = RuleApplicability::AcceptedSyntax;
        let plan = &mut crate::boundary_fixture::fixtures::plans(&mut authored)[rule];
        // Two entries in each file, the union after the files reading them in the scenario's order.
        plan.steps = vec![files_of_functions(), files_of_functions(), united(0, 1)];
        if matches!(scenario, Scenario::StoreRefusesTheSecondRead) {
            plan.steps[2] = united(1, 0);
        }
        plan.frontier = vec![StepInput::Step(StepId(0)), StepInput::Step(StepId(1))];
        plan.emit.step = StepId(2);
    }
    let skipped = authored
        .rules
        .iter()
        .map(|rule| Skipped {
            rule: rule.id.clone(),
            path: FilePath("input.py".into()),
            reason: SkipReason::SyntaxRejected,
        })
        .collect::<Vec<_>>();
    let prepared = engine().prepare(authored).map_err(|_| ())?;
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None.into());
    let first = [Bucket(0), Bucket(10)];
    let refused = match scenario {
        Scenario::RefusedBeforeHandover | Scenario::TwiceInOnePhase => None,
        Scenario::StoreRefusesTheFirstRead | Scenario::StoreRefusesTheSecondRead => {
            Some(first_entry(first[0]))
        }
        Scenario::StoreRefusesTheLaterRule => Some(first_entry(first[1])),
    };
    let stored = Kept {
        refused,
        reads: AtomicUsize::new(0),
    };
    let project = ProjectFacts::new(Relation::ModuleIdentity);
    let project_symbols = Symbols::default();
    let occurrences = match scenario {
        Scenario::TwiceInOnePhase => vec![RuleIndex(0), RuleIndex(0)],
        Scenario::StoreRefusesTheLaterRule => vec![RuleIndex(0), RuleIndex(1)],
        Scenario::RefusedBeforeHandover
        | Scenario::StoreRefusesTheFirstRead
        | Scenario::StoreRefusesTheSecondRead => vec![RuleIndex(0)],
    };
    let inapplicable = vec![
        ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles::default();
        rules
    ];
    let refusals = vec![
        ryo_execution::inbound::use_cases::execution::interface::RefusedFiles::default();
        rules
    ];
    let phase = request.project_phase().map_err(|_| ())?;
    let mut symbols = Symbols::default();
    let refuse_first = matches!(scenario, Scenario::RefusedBeforeHandover);
    let batch = phase.evaluate_rules(
        PhaseRules {
            rules: &occurrences,
            first: &first[..rules],
            corpus: &corpus,
            covered: &[],
            skipped: &skipped,
            inapplicable: &inapplicable,
            refused: &refusals,
            facts: &stored,
            project: &project,
            project_symbols: &project_symbols,
            arenas: &[],
        },
        &mut symbols,
        |number| {
            if refuse_first && number.0 == 0 {
                Err("refused before its handover")
            } else {
                Ok(())
            }
        },
        |_| {
            Ok(PhasePartials {
                evaluation: Partials::default(),
                publication: Partials::default(),
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
        "{answered}; store reads {}; project reads {}",
        stored.reads.load(Ordering::SeqCst),
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
