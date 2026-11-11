//! S12: a project region's inputs, one read in part and one not applicable, merge into a partial answer.
use crate::boundary_fixture::fixtures::{authored_request, corpus, engine, source};
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::Partials;
use ryo_execution::inbound::use_cases::execution::interface::PhaseRules;
use ryo_execution::inbound::use_cases::execution::interface::ProgramFileSessionPort;
use ryo_execution::inbound::use_cases::execution::interface::ProgramProjectPhasePort;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_runtime::outbound::facts::fixtures::project_phase_merged::Buckets;
use ryo_execution_runtime::outbound::facts::fixtures::project_phase_source_facts::CapturedFacts;
use ryo_execution_runtime::outbound::facts::fixtures::project_phase_source_facts::ProjectFacts;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    fixtures::kinds,
    types::{CellKind, FileId, Relation, Value},
};
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::{
    files::types::CorpusInput,
    runs::types::{RuleApplicability, SkipReason, Skipped},
};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input, StepId, StepScope},
    plans::types::{Operation, PlanStep},
};
use std::sync::atomic::AtomicUsize;

/// One row of `relation` in file 0, every cell of its kind.
fn one_row(relation: Relation) -> ryo_values::domain::rows::types::Table {
    let mut table = source();
    table.kinds = kinds(relation).to_vec();
    table.cells = kinds(relation)
        .iter()
        .map(|kind| match kind {
            CellKind::Node => Value::Node(ryo_relations::domain::relations::fixtures::node_id(
                FileId(0),
                ryo_relations::domain::relations::types::Position(2),
            )),
            CellKind::Symbol => Value::Symbol(ryo_relations::domain::terms::types::Symbol(0)),
            CellKind::File => Value::File(FileId(0)),
            CellKind::Span => Value::Span(ryo_relations::domain::relations::types::Span {
                start: ryo_relations::domain::relations::types::Offset(0),
                end: ryo_relations::domain::relations::types::Offset(0),
            }),
            CellKind::Number => Value::Number(ryo_relations::domain::relations::types::Integer(0)),
            CellKind::Flag => Value::Flag(ryo_relations::domain::relations::types::Flag::Lowered),
        })
        .collect();
    table
}

/// S12: a project region over two frontiers, one read in part in file 0 and both skipped in file 1, merges them: its answer is
/// partial in both files, never only not applicable in file 1.
#[test]
fn a_partial_input_and_an_inapplicable_input_merge_into_a_partial_answer() {
    let engine = engine();
    let mut authored = authored_request(1);
    authored.rules[0].applicability = RuleApplicability::AcceptedSyntax;
    let rule_id = authored.rules[0].id.clone();
    let file_of = |relation| {
        u32::try_from(
            kinds(relation)
                .iter()
                .position(|kind| *kind == CellKind::File)
                .unwrap(),
        )
        .unwrap()
    };
    let select = |relation| PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Select {
            from: Input::Relation(relation),
            comparisons: vec![],
            kept: vec![ColumnIndex(file_of(relation))],
        },
    };
    crate::boundary_fixture::fixtures::plans(&mut authored)[0].steps = vec![
        select(Relation::Function),
        select(Relation::Comment),
        PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Union {
                left: Input::Step(StepId(0)),
                right: Input::Step(StepId(1)),
            },
        },
    ];
    crate::boundary_fixture::fixtures::plans(&mut authored)[0].frontier =
        vec![Input::Step(StepId(0)), Input::Step(StepId(1))];
    crate::boundary_fixture::fixtures::plans(&mut authored)[0]
        .emit
        .step = StepId(2);
    let prepared = engine.prepare(authored).unwrap();
    let mut corpus = corpus();
    let mut second = corpus[0].clone();
    let CorpusInput::Available(entry) = &mut second else {
        unreachable!()
    };
    entry.path = FilePath("rejected.py".into());
    corpus.push(second);
    let rows = [
        (Relation::Function, one_row(Relation::Function)),
        (Relation::Comment, one_row(Relation::Comment)),
    ];
    let partial = [
        ryo_execution::inbound::use_cases::execution::interface::Partial {
            relation: Relation::Comment,
            cause: ryo_values::domain::stored_facts::types::PartialCause("a reader stopped".into()),
            lacking: ryo_execution::inbound::use_cases::metadata::interface::Lacking::Any,
            at: ryo_execution::inbound::use_cases::metadata::interface::PartialPlace::File(FileId(
                0,
            )),
        },
    ];
    let mut answers = vec![];
    for read_in_part in [false, true] {
        let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None);
        let mut symbols = Symbols::default();
        let publication: &[_] = if read_in_part { &partial } else { &[] };
        let stored = Buckets(
            super::super::fixtures::captured_in_file_0(&request, &rows, publication, &mut symbols)
                .into_iter()
                .map(|capture| CapturedFacts {
                    captures: vec![(FileId(0), capture)],
                    reads: AtomicUsize::new(0),
                })
                .collect(),
        );
        let skipped = [Skipped {
            rule: rule_id.clone(),
            path: FilePath("rejected.py".into()),
            reason: SkipReason::SyntaxRejected,
        }];
        let project = ProjectFacts::new(Relation::ModuleIdentity);
        let project_symbols = Symbols::default();
        let phase = request.project_phase().unwrap();
        let outcome = crate::boundary_fixture::fixtures::project_root(
            &phase,
            PhaseRules {
                rules: &[RuleIndex(0)],
                first: &[Bucket(0)],
                corpus: &corpus,
                covered: &[],
                skipped: &skipped,
                inapplicable: &[Default::default()],
                refused: &[Default::default()],
                facts: &stored,
                project: &project,
                project_symbols: &project_symbols,
                arenas: &[],
            },
            &Partials::default(),
            &Partials::default(),
            &mut symbols,
        );
        let Ok(RootOutcome::Partial { table, affected }) = outcome.map(|outcome| outcome.emit)
        else {
            panic!("a rule skipped in file 1 answers in part")
        };
        assert_eq!(table.cells, [Value::File(FileId(0))]);
        answers.push(affected);
        phase.finish().unwrap();
        request.finish();
    }
    assert_eq!(
        answers,
        [
            vec![Some(FileId(1))],
            vec![Some(FileId(0)), Some(FileId(1))]
        ],
        "only not applicable in file 1, then also read in part in file 0"
    );
}
