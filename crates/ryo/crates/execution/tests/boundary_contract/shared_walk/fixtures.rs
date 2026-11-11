//! Completed source data and authored requests; every execution enters the public owner.
use crate::boundary_fixture::fixtures::authored_request;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use ryo_execution_runtime::application::file_dag::program::file_session::types::FileInput;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceLanguage;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceRows;
use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RuleOutcome;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_execution_runtime::outbound::operators::fixtures::shared_walk::SemanticCalls;
use ryo_language::domain::builders::fixtures::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::{
    completion::fixtures::{completion, name_kind},
    elements::{
        fixtures::{kind_names, text_at},
        types::Elements,
    },
    kind_index::fixtures::index_kinds,
    symbols::types::{ByteSlice, Symbols},
};
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
use std::sync::Arc;
pub(crate) fn engine() -> (PrepareUseCaseImpl, Arc<SemanticCalls>) {
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
pub(crate) enum Selector {
    Kind(&'static str),
    Parameter(&'static str),
    BadRegex,
    DynamicRegex,
    RegexBeforeAbsentKind,
    AbsentKindBeforeRegex,
}
pub(crate) fn compared(test: Test, column: u32, right: Operand) -> PlanComparison {
    PlanComparison {
        test,
        left: Operand::Column(ColumnIndex(column)),
        right,
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    }
}
pub(crate) fn text(text: &str) -> Operand {
    Operand::Literal(QueryLiteral::Text(LiteralText(text.into())))
}
pub(crate) fn selection(selector: Selector) -> Vec<PlanComparison> {
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
pub(crate) fn prepared(
    engine: &PrepareUseCaseImpl,
    selectors: &[Selector],
) -> PreparedExecutionProgram {
    let mut input = authored_request(selectors.len());
    for (index, selector) in selectors.iter().copied().enumerate() {
        crate::boundary_fixture::fixtures::plans(&mut input)[index].steps[0].operation =
            Operation::Select {
                from: Input::Relation(Relation::SyntaxTree),
                comparisons: selection(selector),
                kept: vec![],
            };
        if let Selector::Parameter(value) = selector {
            crate::boundary_fixture::fixtures::plans(&mut input)[index]
                .parameters
                .push(PlanParameter {
                    name: ParameterName("name".into()),
                    kind: ParameterType::Text,
                });
            crate::boundary_fixture::fixtures::bindings(&mut input)[index] =
                vec![ParameterBinding::Text(LiteralText(value.into()))];
        }
    }
    engine
        .prepare(input)
        .expect("prepare actual shared-walk request")
}
/// A three-byte file's tree: a node over three tokens, its whole vocabulary named.
pub(crate) fn tree(file: FileId, source: &[u8; 3]) -> (Elements, Symbols) {
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
    (elements, symbols)
}
pub(crate) fn input<'a, 'facts>(
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
pub(crate) fn expected(file: FileId, elements: &Elements, positions: &[u32]) -> Table {
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
pub(crate) fn complete(outcome: RuleOutcome) -> Table {
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
/// The error a rule's root failed with; `None` when the root did not fail.
pub(crate) fn failed(outcome: RuleOutcome) -> Option<OperatingError> {
    if let RootOutcome::Failed(error) = outcome.emit {
        Some(error)
    } else {
        None
    }
}
pub(crate) fn regex_error(pattern: &str) -> OperatingError {
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

pub(crate) fn no_catalog(_: Relation) -> Result<SourceRows<'static>, OperatingError> {
    panic!("completed SyntaxTree is supplied by the public file input")
}
