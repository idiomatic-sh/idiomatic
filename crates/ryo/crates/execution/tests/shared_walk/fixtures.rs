//! Completed source data and authored requests; every execution enters the public owner.
use crate::boundary_fixture::authored_request;
use ryo_execution::inbound::use_cases::execution::{
    interface::{ExecutionUseCase, FileExecution, WalkPassCount, WalkSourceRows},
    types::*,
    use_case::ExecutionUseCaseImpl,
};
use ryo_language::domain::{completion::service::{completion, name_kind}, elements::{service::{kind_names, real_count, text_at}, types::Elements}, kind_index::service::index_kinds, symbols::types::{ByteSlice, Symbols}};
use ryo_language::domain::builders::service::{enter_node, leave_node, table_builder, token};
use ryo_operators::inbound::use_cases::operators::{
    interface::{OperatorAnswer, OperatorCall, OperatorsUseCase, PortableOperatorFault},
    use_case::OperatorsUseCaseImpl,
};
use ryo_relations::domain::{
    relations::{
        service::{kinds, node_id},
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
impl OperatorsUseCase for SemanticCalls {
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        if matches!(&call, OperatorCall::Select { .. }) {
            let _previous = self.0.fetch_add(1, Ordering::Relaxed);
        }
        OperatorsUseCaseImpl::new().operators(call)
    }
}
impl SemanticCalls {
    pub(super) fn selects(&self) -> usize {
        self.0.load(Ordering::Relaxed)
    }
}
pub(super) fn engine() -> (ExecutionUseCaseImpl, Arc<SemanticCalls>) {
    let semantics = Arc::new(SemanticCalls::default());
    (
        ExecutionUseCaseImpl::new(
            semantics.clone(),
            Arc::new(ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new()),
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
    engine: &ExecutionUseCaseImpl,
    selectors: &[Selector],
) -> PreparedExecutionProgram {
    let mut input = authored_request(selectors.len());
    for (index, selector) in selectors.iter().copied().enumerate() {
        input.plans[index].steps[0].operation = Operation::Select {
            from: Input::Relation(Relation::SyntaxTree),
            comparisons: selection(selector),
            kept: vec![],
        };
        if let Selector::Parameter(value) = selector {
            input.plans[index].parameters.push(PlanParameter {
                name: ParameterName("name".into()),
                kind: ParameterType::Text,
            });
            input.bindings[index] = vec![ParameterBinding::Text(LiteralText(value.into()))];
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
        resident_stability: facts::ResidentStability::Stable,
        file,
        evaluation: &[],
        publication: &[],
        intents,
        source,
        elements,
        read,
        semantic: None,
        kinds: None,
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
