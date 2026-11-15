//! Completed source data and authored requests; every execution enters the public owner.
use crate::boundary_fixture::fixtures::authored_request;
use crate::boundary_fixture::fixtures::{FileRun, ran, root, run_file};
use ryo_execution::inbound::use_cases::execution::fixtures::TableFacts;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::FileRules;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::PreparedExecutionProgram;
use ryo_execution::inbound::use_cases::execution::interface::RequestExecution;
use ryo_execution::inbound::use_cases::execution::interface::RootOutcome;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
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
            Arc::new(ryo_execution::inbound::use_cases::execution::fixtures::derived()).into(),
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
/// File `file` of `source` with the tree `elements`, run through `run_file` with `intents` over facts that hold no relation:
/// the completed SyntaxTree comes from the tree, and a read of any catalog relation is refused.
pub(crate) fn run(
    request: &RequestExecution,
    (file, source): (FileId, &[u8]),
    elements: &Elements,
    intents: &[FileRuleIntent],
    symbols: &mut Symbols,
) -> FileRules {
    run_file(
        request,
        FileRun {
            file,
            source,
            ..FileRun::of(intents, elements)
        },
        symbols,
        &mut TableFacts::new(&[]),
    )
}
/// A rule's diagnostic collected, with no observation and no capture.
pub(crate) const COLLECTED: FileRuleIntent = FileRuleIntent::Run {
    diagnostic: ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery::Collected,
    observations: ryo_execution::inbound::use_cases::execution::interface::ObservationDemand::NotRequired,
    capture: ryo_execution::inbound::use_cases::execution::interface::CaptureIntent::DiagnosticAndObservations,
};
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
/// The table rule `rule` of `rules` delivered complete.
pub(crate) fn complete(rules: &mut FileRules, rule: usize) -> Table {
    match root(ran(rules, rule)) {
        Ok(RootOutcome::Complete(table)) => table,
        Ok(RootOutcome::Failed(error)) => panic!("native publication failed: {error:?}"),
        Ok(RootOutcome::Unknown(error)) => panic!("native publication unknown: {error:?}"),
        Ok(RootOutcome::Partial { table, affected }) => {
            panic!("native publication partial: {table:?}; affected {affected:?}")
        }
        Ok(RootOutcome::Skipped(_)) => panic!("native publication was skipped"),
        Ok(RootOutcome::NoQuery) => panic!("native publication has no query"),
        Ok(RootOutcome::Deferred { phase }) => panic!("native publication deferred to {phase:?}"),
        Err(stop) => panic!("native publication stopped: {stop:?}"),
    }
}
/// The error rule `rule` of `rules` failed with; `None` when its root did not fail.
pub(crate) fn failed(rules: &mut FileRules, rule: usize) -> Option<OperatingError> {
    if let Ok(RootOutcome::Failed(error)) = root(ran(rules, rule)) {
        Some(error)
    } else {
        None
    }
}
pub(crate) fn regex_error(pattern: &str) -> OperatingError {
    ryo_execution::inbound::use_cases::execution::fixtures::regex_refusal(pattern)
        .expect("intentionally malformed regular expression")
}
