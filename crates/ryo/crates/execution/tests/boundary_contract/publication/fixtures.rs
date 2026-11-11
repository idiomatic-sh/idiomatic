//! The support of the `publication` cases and of the groups nested in it.
use super::super::*;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use ryo_values::domain::plan_parts::types::LiteralText;

pub(crate) fn program(engine: &PrepareUseCaseImpl) -> PreparedExecutionProgram {
    let mut input = authored_request(1);
    crate::boundary_fixture::fixtures::plans(&mut input)[0].steps = vec![PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::FitsRendered {
            from: Input::Relation(Relation::Function),
            texts: vec![],
            literals: vec![LiteralText("x".into())],
            span: ColumnIndex(3),
            max_line: ryo_values::domain::query_operators::types::MaxLine(80),
            tab_width: ryo_values::domain::query_operators::types::TabWidth(4),
        },
    }];
    crate::boundary_fixture::fixtures::plans(&mut input)[0]
        .emit
        .step = StepId(0);
    engine.prepare(input).expect("actual FitsRendered program")
}

pub(crate) fn rows(second: (u32, u32)) -> Table {
    let mut table = source();
    let mut first = table.cells.clone();
    first[3] = Value::Span(Span {
        start: Offset(0),
        end: Offset(1),
    });
    let mut next = first.clone();
    next[0] = Value::Node(node_id(FileId(0), Position(3)));
    next[3] = Value::Span(Span {
        start: Offset(second.0),
        end: Offset(second.1),
    });
    table.cells = [first, next].concat();
    table.count = RowCount(2);
    table
}

/// A read of the function relation that answers `table`.
pub(crate) fn function_rows<'t>(
    table: &'t Table,
) -> impl Fn(Relation) -> Result<SourceRows<'t>, OperatingError> {
    move |relation| {
        assert_eq!(relation, Relation::Function);
        Ok(SourceRows::Computed(table))
    }
}

/// Recognized file 0 of the source `abc`, opened with `intents` and reading through `read`.
pub(crate) fn abc_file<'a, 'facts>(
    intents: &'a [FileRuleIntent],
    elements: &'a ryo_language::domain::elements::types::Elements,
    read: &'a dyn Fn(Relation) -> Result<SourceRows<'facts>, OperatingError>,
) -> FileInput<'a, 'facts> {
    FileInput {
        language: SourceLanguage::Recognized,
        resident_stability: ryo_execution_runtime::outbound::facts::types::ResidentStability::Stable,
        file: FileId(0),
        evaluation: &[],
        publication: &[],
        intents,
        source: ryo_values::domain::files::types::SourceView(b"abc"),
        elements,
        read,
        semantic: ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Absent,
        kinds: ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport::Absent,
    }
}
