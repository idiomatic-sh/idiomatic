//! The interpreter's own visit bounds over increasing authored input populations: the rows each input of the operation is fed
//! grow linearly with the population.
use super::super::super::conformance::test::{author, compare_delivery, repeated};
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::expectations::SOURCE;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::values::numbers;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::*;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, RepeatedRows},
    plans::types::Operation,
    query_operators::types::OperationKind,
};
use std::borrow::Cow;
/// What one population delivered: its size, the rows delivered, and the rows each input was fed.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Scale {
    population: usize,
    rows: usize,
    fed: Cow<'static, [u64]>,
}

/// The kind whose fixture runs over each population.
type Input = OperationKind;
/// Each population's delivery, in order.
type Value = Cow<'static, [Scale]>;
type Failure = OperatingError;

/// The populations each fixture runs over: none, 32 and 64.
const POPULATIONS: [usize; 3] = [0, 32, 64];

/// A population of `population` delivering `rows` rows, its inputs each fed `fed` rows.
const fn scale(population: usize, rows: usize, fed: &'static [u64]) -> Scale {
    Scale {
        population,
        rows,
        fed: Cow::Borrowed(fed),
    }
}

/// A fixture of three rows per repetition, delivering each row it is fed.
const THREE_PER_REPETITION: &[Scale] = &[
    scale(0, 0, &[0]),
    scale(32, 96, &[96]),
    scale(64, 192, &[192]),
];
/// A fixture of one row per repetition, delivering two rows for each it is fed.
const TWO_PER_ROW: &[Scale] = &[
    scale(0, 0, &[0]),
    scale(32, 64, &[32]),
    scale(64, 128, &[64]),
];
/// A fixture of one row per repetition, delivering each row it is fed.
const ONE_PER_ROW: &[Scale] = &[
    scale(0, 0, &[0]),
    scale(32, 32, &[32]),
    scale(64, 64, &[64]),
];
/// A binary fixture of unique keys, delivering one row per key and fed each key on both sides.
const ONE_PER_KEY: &[Scale] = &[
    scale(0, 0, &[0, 0]),
    scale(32, 32, &[32, 32]),
    scale(64, 64, &[64, 64]),
];

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(stringify!(former_unary_pilots_keep_linear_visits)),
        word: Word::Succeeds,
        input: || OperationKind::Select,
        expect: Expect::Value(Cow::Borrowed(THREE_PER_REPETITION)),
    },
    Case {
        name: CaseName("a_projection_that_keeps_repeats_keeps_linear_visits"),
        word: Word::Succeeds,
        input: || OperationKind::Project,
        expect: Expect::Value(Cow::Borrowed(THREE_PER_REPETITION)),
    },
    Case {
        name: CaseName("a_source_match_keeps_linear_visits"),
        word: Word::Succeeds,
        input: || OperationKind::SourceMatches,
        expect: Expect::Value(Cow::Borrowed(TWO_PER_ROW)),
    },
    Case {
        name: CaseName("span_bounds_keep_linear_visits"),
        word: Word::Succeeds,
        input: || OperationKind::SpanBounds,
        expect: Expect::Value(Cow::Borrowed(ONE_PER_ROW)),
    },
    Case {
        name: CaseName(stringify!(
            former_binary_pilots_keep_linear_visits_for_unique_keys
        )),
        word: Word::Succeeds,
        input: || OperationKind::Join,
        expect: Expect::Value(Cow::Borrowed(ONE_PER_KEY)),
    },
    Case {
        name: CaseName("a_union_of_unique_keys_keeps_linear_visits"),
        word: Word::Succeeds,
        input: || OperationKind::Union,
        expect: Expect::Value(Cow::Borrowed(ONE_PER_KEY)),
    },
];

/// Delivers the kind's fixture over each population: a unary fixture's input repeated that many
/// times (a projection keeping repeats), a binary fixture over the keys `0..population` on both
/// sides (a join outputting its key columns).
fn subject(kind: Input) -> Result<Value, Failure> {
    let mut case = fixture(kind);
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "only a projection and a join are adjusted; every other fixture runs as authored"
    )]
    match &mut case.operation {
        Operation::Project { distinct, .. } => *distinct = RepeatedRows::Kept,
        Operation::Join { output, .. } => {
            for column in output {
                column.column = ColumnIndex(0);
            }
        }
        _ => {}
    }
    let unary = case.inputs.len() == 1;
    let mut scales = Vec::new();
    for population in POPULATIONS {
        let inputs = if unary {
            vec![repeated(&case.inputs[0], population)]
        } else {
            let keys = (0..i64::try_from(population).unwrap_or(i64::MAX)).collect::<Vec<_>>();
            vec![numbers(&keys), numbers(&keys)]
        };
        let delivered = compare_delivery(
            &case.operation,
            &inputs,
            &[],
            SOURCE,
            &mut case.symbols,
            None,
        )?;
        scales.push(Scale {
            population,
            rows: delivered.rows.len(),
            fed: Cow::Owned(visits(&case.operation, &inputs, &mut case.symbols)?),
        });
    }
    Ok(Cow::Owned(scales))
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
) -> Result<Vec<u64>, OperatingError> {
    use crate::inbound::use_cases::execution::interface::{
        CaptureIntent, DiagnosticDelivery, ExecutionUseCase as _, FileRuleIntent,
        ObservationDemand, RuleIndex,
    };
    use ryo_execution_runtime::application::file_dag::program::file_session::interface::ProgramFileSessionPort as _;
    use ryo_execution_runtime::application::file_dag::program::file_session::types::{
        FileInput, KindSupport, SemanticSupport, SourceLanguage, SourceRows,
    };
    use ryo_execution_runtime::application::file_dag::program::request_execution::interface::ProgramRequestExecutionPort as _;
    let authored = author(operation, inputs, &[], symbols);
    let emit = authored.plan.emit.step;
    let (_engine, program) = super::super::super::super::fixtures::requests::prepared(
        authored.plan,
        vec![],
        authored.hosted,
    )?;
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
    let mut file = request.open_file(
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
    )?;
    let _outcome = file.rule(
        RuleIndex(0),
        super::super::super::super::fixtures::requests::emitted(),
        symbols,
    );
    let fed = file
        .fixture_counts(RuleIndex(0))
        .map(|counts| counts.fed[usize::from(emit)].0[..inputs.len()].to_vec())?;
    let _finished = file.finish();
    request.finish();
    Ok(fed)
}
