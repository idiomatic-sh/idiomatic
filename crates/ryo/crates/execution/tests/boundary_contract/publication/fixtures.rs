//! The support of the `publication` cases and of the groups nested in it.
use super::super::*;
use ryo_execution::inbound::use_cases::execution::interface::PreparedExecutionProgram;
use ryo_values::domain::plan_parts::types::LiteralText;

pub(crate) fn program(engine: &PrepareUseCaseImpl) -> PreparedExecutionProgram {
    let mut input = authored_request(1);
    boundary_fixture::fixtures::plans(&mut input)[0].steps = vec![PlanStep {
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
    boundary_fixture::fixtures::plans(&mut input)[0].emit.step = StepId(0);
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

/// Facts holding `table` as the function relation's rows, and no other relation.
pub(crate) fn function_rows(table: &Table) -> TableFacts {
    TableFacts::new(&[(Relation::Function, table)])
}
