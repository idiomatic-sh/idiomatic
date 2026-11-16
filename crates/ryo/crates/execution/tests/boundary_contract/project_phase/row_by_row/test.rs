//! A step after the files whose operator reads its operands row by row refuses a column past its input, or of the wrong kind,
//! at the first row that reaches it: over no rows it answers the layout its rows keep, and a projection reading it reads no
//! column of it, so it is refused once the step ran without a refusal of its own. Every answer is the interpreter's at
//! checkpoint-350 (64a99968a1), run on the same input through the project phase.
use crate::boundary_fixture::fixtures::{authored_request, corpus, engine};
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::Partials;
use ryo_execution::inbound::use_cases::execution::interface::PhasePartials;
use ryo_execution::inbound::use_cases::execution::interface::PhaseRules;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::interface::ProjectBatch;
use ryo_execution::inbound::use_cases::execution::interface::ProjectRootExecution;
use ryo_execution::inbound::use_cases::execution::interface::RootOutcome;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution_runtime::outbound::facts::fixtures::activation_project::NoFacts;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::hosted::types::{
    HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow,
};
use ryo_values::domain::plan_parts::types::{
    CapturePattern, ColumnIndex, HostedNumber, Input, RepeatedRows, StepId, StepScope,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::row_texts::types::Cell as HostCell;
use ryo_values::domain::runs::types::{RuleApplicability, SkipReason, Skipped};
use ryo_values::domain::stored_facts::types::Bucket;

/// What the interpreter at checkpoint-350 answered for each case, run there and recorded.
const AFTER_THE_FILES: &str = concat!(
    "capture-col99 populated=false downstream=false: complete [Symbol, Number, Symbol] RowCount(0)\n",
    "capture-col99 populated=false downstream=true: failed Table(NoSuchColumn { index: 0 })\n",
    "capture-col99 populated=true downstream=false: failed Table(NoSuchColumn { index: 99 })\n",
    "capture-col99 populated=true downstream=true: failed Table(NoSuchColumn { index: 99 })\n",
    "words-col99 populated=false downstream=false: complete [Symbol, Number, Symbol] RowCount(0)\n",
    "words-col99 populated=false downstream=true: failed Table(NoSuchColumn { index: 0 })\n",
    "words-col99 populated=true downstream=false: failed Table(NoSuchColumn { index: 99 })\n",
    "words-col99 populated=true downstream=true: failed Table(NoSuchColumn { index: 99 })\n",
    "spanbounds-col99 populated=false downstream=false: complete [Symbol, Number, Number, Number] RowCount(0)\n",
    "spanbounds-col99 populated=false downstream=true: failed Table(NoSuchColumn { index: 0 })\n",
    "spanbounds-col99 populated=true downstream=false: failed Table(NoSuchColumn { index: 99 })\n",
    "spanbounds-col99 populated=true downstream=true: failed Table(NoSuchColumn { index: 99 })\n",
    "spanbounds-text populated=false downstream=false: complete [Symbol, Number, Number, Number] RowCount(0)\n",
    "spanbounds-text populated=false downstream=true: failed Table(NoSuchColumn { index: 0 })\n",
    "spanbounds-text populated=true downstream=false: failed ShapeMismatch\n",
    "spanbounds-text populated=true downstream=true: failed ShapeMismatch\n",
);

/// `operation` made after the files.
fn after(operation: Operation) -> PlanStep {
    PlanStep {
        scope: StepScope::CrossFile,
        operation,
    }
}

/// The hosted `text(value: Text, n: Number)`, with one row or none.
fn text(populated: bool) -> HostRelation {
    HostRelation {
        name: HostName("text".into()),
        columns: vec![
            HostColumn {
                name: HostColumnName("value".into()),
                kind: HostKind::Text,
            },
            HostColumn {
                name: HostColumnName("n".into()),
                kind: HostKind::Number,
            },
        ],
        rows: if populated {
            vec![HostRow(vec![
                HostCell::Text("x".into()),
                HostCell::Number(Integer(1)),
            ])]
        } else {
            vec![]
        },
    }
}

/// What the project phase answers for a rule whose every step runs after the files: the hosted text, `operation` over it,
/// and, when `downstream`, a projection of `operation`'s column 0, emitted.
fn answered(operation: Operation, populated: bool, downstream: bool) -> String {
    let mut authored = authored_request(1);
    authored.rules[0].applicability = RuleApplicability::AcceptedSyntax;
    authored.hosted = vec![text(populated)];
    let plan = &mut crate::boundary_fixture::fixtures::plans(&mut authored)[0];
    plan.steps = vec![
        after(Operation::Hosted {
            relation: HostedNumber(0),
        }),
        after(operation),
    ];
    if downstream {
        plan.steps.push(after(Operation::Project {
            from: Input::Step(StepId(1)),
            columns: vec![ColumnIndex(0)],
            distinct: RepeatedRows::Kept,
        }));
    }
    plan.emit.step = StepId(u32::try_from(plan.steps.len() - 1).expect("a few steps"));
    plan.frontier = Vec::new();
    let skipped = vec![Skipped {
        rule: authored.rules[0].id.clone(),
        path: FilePath("input.py".into()),
        reason: SkipReason::SyntaxRejected,
    }];
    let prepared = engine().prepare(authored).expect("prepare the rule");
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None.into());
    let project_symbols = Symbols::default();
    let inapplicable =
        vec![ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles::default()];
    let refused =
        vec![ryo_execution::inbound::use_cases::execution::interface::RefusedFiles::default()];
    let phase = request.project_phase().expect("the project phase");
    let mut symbols = Symbols::default();
    let batch = phase.evaluate_rules(
        PhaseRules {
            rules: &[RuleIndex(0)],
            first: &[Bucket(0)],
            corpus: &corpus,
            covered: &[],
            skipped: &skipped,
            inapplicable: &inapplicable,
            refused: &refused,
            facts: &NoFacts,
            project: &NoFacts,
            project_symbols: &project_symbols,
            arenas: &[],
        },
        &mut symbols,
        |_| Ok::<(), &str>(()),
        |_| {
            Ok(PhasePartials {
                evaluation: Partials::default(),
                publication: Partials::default(),
            })
        },
        |_| Ok(()),
        |_, outcome, _, _, _| Ok(outcome.emit),
    );
    match batch {
        ProjectBatch::Complete(done) => done
            .iter()
            .map(|outcome| match outcome {
                Ok(RootOutcome::Complete(table)) => {
                    format!("complete {:?} {:?}", table.kinds, table.count)
                }
                Ok(RootOutcome::Failed(error)) => format!("failed {error:?}"),
                Ok(RootOutcome::Partial { table, .. }) => format!("partial {:?}", table.kinds),
                Ok(RootOutcome::Unknown(error)) => format!("unknown {error:?}"),
                Ok(RootOutcome::Deferred { phase }) => format!("deferred {phase:?}"),
                Ok(_) => "other".into(),
                Err(stop) => format!("stopped {stop:?}"),
            })
            .collect::<Vec<_>>()
            .join(","),
        ProjectBatch::Refused { error, .. } => format!("refused {error:?}"),
    }
}

#[test]
fn row_by_row_refusals_and_their_readers_after_the_files_answer_as_the_interpreter() {
    let from = Input::Step(StepId(0));
    let steps = [
        (
            "capture-col99",
            Operation::Capture {
                from,
                path: ColumnIndex(99),
                pattern: CapturePattern("{x}".into()),
            },
        ),
        (
            "words-col99",
            Operation::Words {
                from,
                text: ColumnIndex(99),
            },
        ),
        (
            "spanbounds-col99",
            Operation::SpanBounds {
                from,
                span: ColumnIndex(99),
            },
        ),
        (
            "spanbounds-text",
            Operation::SpanBounds {
                from,
                span: ColumnIndex(0),
            },
        ),
    ];
    let mut shown = String::new();
    for (name, operation) in steps {
        for populated in [false, true] {
            for downstream in [false, true] {
                shown.push_str(&format!(
                    "{name} populated={populated} downstream={downstream}: {}\n",
                    answered(operation.clone(), populated, downstream)
                ));
            }
        }
    }
    assert_eq!(shown, AFTER_THE_FILES);
}
