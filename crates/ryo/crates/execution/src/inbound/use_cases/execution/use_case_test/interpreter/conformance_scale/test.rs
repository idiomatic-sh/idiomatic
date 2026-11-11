//! The interpreter's own visit bounds over increasing authored input populations: the rows each input of the operation is fed
//! grow linearly with the population.
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use crate::inbound::use_cases::execution::interface::OperatingError;
use super::super::super::conformance::test::{author, compare_delivery, repeated};
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::*;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::values::numbers;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::expectations::SOURCE;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::{
    plan_parts::types::ColumnIndex,
    plans::types::Operation,
    query_operators::types::OperationKind,
};
fn former_unary_pilots_keep_linear_visits() -> Result<(), Failure> {
    for kind in [
        OperationKind::Select,
        OperationKind::Project,
        OperationKind::SourceMatches,
        OperationKind::SpanBounds,
    ] {
        let mut case = fixture(kind);
        if let Operation::Project { distinct, .. } = &mut case.operation {
            *distinct = ryo_values::domain::plan_parts::types::RepeatedRows::Kept;
        }
        let small = vec![repeated(&case.inputs[0], 32)];
        let large = vec![repeated(&case.inputs[0], 64)];
        let first = compare_delivery(
            &case.operation,
            &small,
            &[],
            SOURCE,
            &mut case.symbols,
            None,
        )
        .unwrap();
        let second = compare_delivery(
            &case.operation,
            &large,
            &[],
            SOURCE,
            &mut case.symbols,
            None,
        )
        .unwrap();
        let first_visits = visits(&case.operation, &small, &mut case.symbols);
        let second_visits = visits(&case.operation, &large, &mut case.symbols);
        assert!(first_visits[0] > 0);
        assert_eq!(second_visits[0], first_visits[0] * 2);
        assert_eq!(second.rows.len(), first.rows.len() * 2);
        let empty = compare_delivery(
            &case.operation,
            &[repeated(&case.inputs[0], 0)],
            &[],
            SOURCE,
            &mut case.symbols,
            None,
        )
        .unwrap();
        assert!(empty.rows.is_empty());
        assert_eq!(
            visits(
                &case.operation,
                &[repeated(&case.inputs[0], 0)],
                &mut case.symbols
            ),
            vec![0]
        );
    }
    Ok(())
}
fn former_binary_pilots_keep_linear_visits_for_unique_keys() -> Result<(), Failure> {
    for kind in [OperationKind::Join, OperationKind::Union] {
        let mut case = fixture(kind);
        if let Operation::Join { output, .. } = &mut case.operation {
            for column in output {
                column.column = ColumnIndex(0);
            }
        }
        let run = |count: i64, symbols: &mut Symbols| {
            let values = (0..count).collect::<Vec<_>>();
            compare_delivery(
                &case.operation,
                &[numbers(&values), numbers(&values)],
                &[],
                SOURCE,
                symbols,
                None,
            )
            .unwrap()
        };
        let measured = |count: i64, symbols: &mut Symbols| {
            let values = (0..count).collect::<Vec<_>>();
            visits(
                &case.operation,
                &[numbers(&values), numbers(&values)],
                symbols,
            )
        };
        let first = run(32, &mut case.symbols);
        let second = run(64, &mut case.symbols);
        assert_eq!((first.rows.len(), second.rows.len()), (32, 64));
        let (first_visits, second_visits) = (
            measured(32, &mut case.symbols),
            measured(64, &mut case.symbols),
        );
        for side in 0..2 {
            assert!(first_visits[side] > 0);
            assert_eq!(second_visits[side], first_visits[side] * 2);
        }
        let empty = run(0, &mut case.symbols);
        assert!(empty.rows.is_empty());
        assert_eq!(measured(0, &mut case.symbols), vec![0, 0]);
    }
    Ok(())
}
/// The scenarios of this group, one row each.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    FormerUnaryPilotsKeepLinearVisits,
    FormerBinaryPilotsKeepLinearVisitsForUniqueKeys,
}
type Input = Scenario;
type Value = ();
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(stringify!(former_unary_pilots_keep_linear_visits)),
        word: Word::Succeeds,
        input: || Scenario::FormerUnaryPilotsKeepLinearVisits,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(
            former_binary_pilots_keep_linear_visits_for_unique_keys
        )),
        word: Word::Succeeds,
        input: || Scenario::FormerBinaryPilotsKeepLinearVisitsForUniqueKeys,
        expect: Expect::Value(()),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::FormerUnaryPilotsKeepLinearVisits => former_unary_pilots_keep_linear_visits(),
        Scenario::FormerBinaryPilotsKeepLinearVisitsForUniqueKeys => {
            former_binary_pilots_keep_linear_visits_for_unique_keys()
        }
    }
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

/// The rows each input of `operation` was fed when it ran over `inputs` in one file, read through the runtime's fixture counts.
fn visits(
    operation: &Operation,
    inputs: &[ryo_values::domain::rows::types::Table],
    symbols: &mut Symbols,
) -> Vec<u64> {
    use crate::inbound::use_cases::execution::interface::{
        CaptureIntent, DiagnosticDelivery, ExecutionUseCase as _, FileRuleIntent,
        ObservationDemand, RuleIndex,
    };
    use ryo_execution_runtime::application::file_dag::program::file_session::interface::FileExecution as _;
    use ryo_execution_runtime::application::file_dag::program::file_session::types::{
        FileInput, KindSupport, SemanticSupport, SourceLanguage, SourceRows,
    };
    use ryo_execution_runtime::application::file_dag::program::request_execution::interface::ExecutionRequest as _;
    let authored = author(operation, inputs, &[], symbols);
    let emit = authored.plan.emit.step;
    let (_engine, program) = super::super::super::super::fixtures::requests::prepared(
        authored.plan,
        vec![],
        authored.hosted,
    )
    .expect("prepare the operation");
    let corpus = [ryo_values::domain::files::types::CorpusInput::Available(
        ryo_values::domain::files::types::CorpusEntry {
            path: ryo_language::domain::files::types::FilePath("operator-contract.py".into()),
            size: ryo_values::domain::files::types::FileSize(SOURCE.len() as u64),
            modified: ryo_values::domain::files::types::Nanoseconds(0),
            executable: ryo_values::domain::files::types::ExecutablePermission::Unknown,
        },
    )];
    let facts = authored.facts;
    let read = |relation| {
        facts
            .iter()
            .find(|(actual, _)| *actual == relation)
            .map(|(_, table)| SourceRows::Computed(table))
            .ok_or(OperatingError::NotRead)
    };
    let request = super::super::super::super::fixtures::requests::execution()
        .execution(&program, &corpus, None);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let mut file = request
        .open_file(
            FileInput {
                language: SourceLanguage::Recognized,
                resident_stability:
                    ryo_execution_runtime::outbound::facts::types::ResidentStability::Stable,
                file: ryo_relations::domain::relations::types::FileId(0),
                evaluation: &[],
                publication: &[],
                intents: &[FileRuleIntent::Run {
                    diagnostic: DiagnosticDelivery::Collected,
                    observations: ObservationDemand::NotRequired,
                    capture: CaptureIntent::Disabled,
                }],
                source: ryo_values::domain::files::types::SourceView(SOURCE),
                elements: &elements,
                read: &read,
                semantic: SemanticSupport::Absent,
                kinds: KindSupport::Absent,
            },
            symbols,
        )
        .expect("open the file");
    let _outcome = file.rule(
        RuleIndex(0),
        super::super::super::super::fixtures::requests::emitted(),
        symbols,
    );
    let fed = file
        .fixture_counts(RuleIndex(0))
        .expect("the rule's counts")
        .fed[usize::from(emit)]
    .0[..inputs.len()]
        .to_vec();
    let _finished = file.finish();
    request.finish();
    fed
}
