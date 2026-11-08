//! Completed source data and authored requests; every execution enters the public owner.
use ryo_execution_runtime::outbound::operators::interface::OperatorsPort as _;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use crate::boundary_fixture::authored_request;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileExecution;
use ryo_execution::inbound::use_cases::execution::interface::WalkPassCount;
use ryo_execution::inbound::use_cases::execution::interface::WalkSourceRows;
use ryo_execution_runtime::application::file_dag::program::file_session::types::FileInput;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceLanguage;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceRows;
use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RuleOutcome;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_language::domain::{completion::fixtures::{completion, name_kind}, elements::{fixtures::{kind_names, real_count, text_at}, types::Elements}, kind_index::fixtures::index_kinds, symbols::types::{ByteSlice, Symbols}};
use ryo_language::domain::builders::fixtures::{enter_node, leave_node, table_builder, token};
use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl;
use ryo_relations::domain::{
    relations::{
        fixtures::{kinds, node_id},
        types::{FileId, KindBits, Offset, Position, Relation, Span, Value},
    },
    terms::types::NameText,
};
use ryo_values::domain::{
    plan_parts::types::{
        ColumnIndex, Input, LiteralText, Operand, ParameterId, ParameterName, PlanComparison,
        QueryLiteral, Test,
    },
    plans::types::Operation,
    query_bindings::types::{ParameterBinding, ParameterType, PlanParameter},
    rows::types::{RowCount, Table},
};
use ryo_values::selects;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
#[derive(Debug, Default)]
pub(super) struct SemanticCalls(AtomicUsize);
impl ryo_execution_runtime::outbound::operators::interface::OperatorsPort for SemanticCalls {
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        if matches!(&call, OperatorCall::Evaluate { operation: ryo_values::domain::plans::types::Operation::Select { .. }, .. }) {
            let _previous = self.0.fetch_add(1, Ordering::Relaxed);
        }
        OperatorsUseCaseImpl::new(std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case())).operators(call)
    }
}
impl SemanticCalls {
    pub(super) fn selects(&self) -> usize {
        self.0.load(Ordering::Relaxed)
    }
}
pub(super) fn engine() -> (PrepareUseCaseImpl, Arc<SemanticCalls>) {
    let semantics = Arc::new(SemanticCalls::default());
    (
        PrepareUseCaseImpl::new(
            semantics.clone(),
            Arc::new(ryo_execution_store::outbound::derived::factory::semantics()),
            Arc::new(ryo_execution_runtime::outbound::digest::factory::blake3()),
        ),
        semantics,
    )
}
#[derive(Clone, Copy)]
pub(super) enum Selector {
    Kind(&'static str),
    Parameter(&'static str),
    BadRegex,
    DynamicRegex,
    RegexBeforeAbsentKind,
    AbsentKindBeforeRegex,
}
pub(super) fn compared(test: Test, column: u32, right: Operand) -> PlanComparison {
    PlanComparison {
        test,
        left: Operand::Column(ColumnIndex(column)),
        right,
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    }
}
pub(super) fn text(text: &str) -> Operand {
    Operand::Literal(QueryLiteral::Text(LiteralText(text.into())))
}
pub(super) fn selection(selector: Selector) -> Vec<PlanComparison> {
    let kind = |name| compared(Test::Equal, 1, text(name));
    let regex = compared(Test::Matches, 2, Operand::Column(ColumnIndex(2)));
    match selector {
        Selector::Kind(name) => vec![kind(name)],
        Selector::Parameter(_) => {
            vec![compared(Test::Equal, 2, Operand::Parameter(ParameterId(0)))]
        }
        Selector::BadRegex => vec![kind("word"), compared(Test::Matches, 2, text("["))],
        Selector::DynamicRegex => vec![kind("word"), regex],
        Selector::RegexBeforeAbsentKind => vec![regex, kind("absent-kind")],
        Selector::AbsentKindBeforeRegex => vec![kind("absent-kind"), regex],
    }
}
pub(super) fn prepared(
    engine: &PrepareUseCaseImpl,
    selectors: &[Selector],
) -> PreparedExecutionProgram {
    let mut input = authored_request(selectors.len());
    for (index, selector) in selectors.iter().copied().enumerate() {
        crate::boundary_fixture::plans(&mut input)[index].steps[0].operation = Operation::Select {
            from: Input::Relation(Relation::SyntaxTree),
            comparisons: selection(selector),
            kept: vec![],
        };
        if let Selector::Parameter(value) = selector {
            crate::boundary_fixture::plans(&mut input)[index].parameters.push(PlanParameter {
                name: ParameterName("name".into()),
                kind: ParameterType::Text,
            });
            crate::boundary_fixture::bindings(&mut input)[index] = vec![ParameterBinding::Text(LiteralText(value.into()))];
        }
    }
    engine
        .prepare(input)
        .expect("prepare actual shared-walk request")
}
pub(super) fn tree(file: FileId, source: &[u8]) -> (Elements, Symbols) {
    assert_eq!(source.len(), 3);
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(source));
    enter_node(&mut builder, KindBits(1), Offset(0));
    token(&mut builder, KindBits(2), Offset(0), Offset(1));
    token(&mut builder, KindBits(3), Offset(1), Offset(2));
    token(&mut builder, KindBits(2), Offset(2), Offset(3));
    leave_node(&mut builder, Offset(3));
    {
        let mut completed = completion(&mut elements, &mut symbols, ByteSlice(source), file);
        for (kind, name) in [(1, "file"), (2, "word"), (3, "other")] {
            name_kind(&mut completed, KindBits(kind), NameText(name));
        }
    }
    index_kinds(&mut elements);
    assert_eq!(real_count(&elements).0, 4);
    assert!(
        kind_names(&elements).len() > 3,
        "fully completed vocabulary enables the physical shared pass"
    );
    (elements, symbols)
}
pub(super) fn input<'a, 'facts>(
    file: FileId,
    source: &'a [u8],
    elements: &'a Elements,
    intents: &'a [FileRuleIntent],
    read: &'a dyn Fn(Relation) -> Result<SourceRows<'facts>, OperatingError>,
) -> FileInput<'a, 'facts> {
    FileInput {
        language: SourceLanguage::Recognized,
        resident_stability: ResidentStability::Stable,
        file,
        evaluation: &[],
        publication: &[],
        intents,
        source: ryo_values::domain::files::types::SourceView(source),
        elements,
        read,
        semantic: ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Absent,
        kinds: ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport::Absent,
    }
}
pub(super) fn expected(file: FileId, elements: &Elements, positions: &[u32]) -> Table {
    let mut cells = vec![];
    for position in positions {
        let kind = if *position == 2 { 3 } else { 2 };
        cells.extend([
            Value::Node(node_id(file, Position(*position))),
            Value::Symbol(kind_names(elements)[kind]),
            Value::Symbol(text_at(elements, Position(*position))),
            Value::File(file),
            Value::Span(Span {
                start: Offset(position - 1),
                end: Offset(*position),
            }),
        ]);
    }
    Table {
        kinds: kinds(Relation::SyntaxTree).to_vec(),
        count: RowCount(positions.len()),
        cells,
    }
}
pub(super) fn complete(outcome: RuleOutcome) -> Table {
    match outcome.emit {
        RootOutcome::Complete(table) => table,
        RootOutcome::Failed(error) => panic!("native publication failed: {error:?}"),
        RootOutcome::Unknown(error) => panic!("native publication unknown: {error:?}"),
        RootOutcome::Partial { table, affected } => {
            panic!("native publication partial: {table:?}; affected {affected:?}")
        }
        RootOutcome::Skipped(_) => panic!("native publication was skipped"),
        RootOutcome::NoQuery => panic!("native publication has no query"),
        RootOutcome::Deferred { phase } => panic!("native publication deferred to {phase:?}"),
    }
}
pub(super) fn failed(outcome: RuleOutcome, expected: OperatingError) {
    let RootOutcome::Failed(error) = outcome.emit else {
        panic!("typed failed root required")
    };
    assert_eq!(error, expected);
}
pub(super) fn counts(file: &impl FileExecution, passes: u64, rows: u64) {
    let actual = file.walk_counts();
    assert_eq!(actual.passes, WalkPassCount(passes));
    assert_eq!(actual.source_rows, WalkSourceRows(rows));
}
pub(super) fn regex_error(pattern: &str) -> OperatingError {
    use ryo_matching::inbound::use_cases::matching::{
        errors::MatchingError, interface::MatchingUseCase, types::RegexSource,
        use_case::MatchingUseCaseImpl,
    };
    let error = MatchingUseCaseImpl::new()
        .compile_regex(RegexSource(pattern))
        .err()
        .expect("intentionally malformed regular expression");
    let (MatchingError::InvalidRegex { reason } | MatchingError::BadGlob { reason, .. }) = error;
    OperatingError::InvalidRegex { reason }
}
