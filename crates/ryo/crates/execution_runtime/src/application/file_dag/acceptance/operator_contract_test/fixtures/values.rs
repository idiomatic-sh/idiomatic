//! Authored values, rows and the name table the conformance fixtures are built from.
use ryo_language::domain::semantics::types::RenderOutcome;
use ryo_language::domain::semantics::types::RenderRequest;
use ryo_language::domain::symbols::service::query_intern_derived;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::service::node_id;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::FileId;
use ryo_relations::domain::relations::types::Integer;
use ryo_relations::domain::relations::types::Offset;
use ryo_relations::domain::relations::types::Position;
use ryo_relations::domain::relations::types::Relation;
use ryo_relations::domain::relations::types::Span;
use ryo_relations::domain::relations::types::Value;
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::rows::service::push_row;
use ryo_values::domain::rows::types::Table;
use std::sync::Arc;
use super::expectations::SOURCE;

#[derive(Debug)]
pub(crate) struct KnownSyntax;
impl ryo_execution_domain::domain::operating::types::SemanticLookup for KnownSyntax {
    fn render_batch(
        &self,
        requests: &[RenderRequest],
        _: ryo_values::domain::files::types::SourceView<'_>,
    ) -> Result<Vec<RenderOutcome>, ryo_execution_domain::domain::operating::errors::OperatingError> {
        Ok(requests
            .iter()
            .map(|_| RenderOutcome::Known(ryo_language::domain::semantics::types::RenderedNode(Arc::from("a"))))
            .collect())
    }
}
pub(crate) const FROM: Input = Input::Relation(Relation::Function);
pub(crate) const RIGHT: Input = Input::Relation(Relation::Scope);
pub(crate) const DOMAIN: Input = Input::Relation(Relation::SyntaxTree);
pub(crate) fn n(value: i64) -> Value {
    Value::Number(Integer(value))
}
pub(crate) fn span(start: u32, end: u32) -> Value {
    Value::Span(Span {
        start: Offset(start),
        end: Offset(end),
    })
}
pub(super) fn node(position: u32) -> Value {
    Value::Node(node_id(FileId(0), Position(position)))
}
pub(super) fn table(kinds: &[CellKind], rows: &[Vec<Value>]) -> Table {
    let mut table = Table {
        kinds: kinds.to_vec(),
        ..Table::default()
    };
    for row in rows {
        push_row(&mut table, row);
    }
    table
}
pub(crate) fn numbers(values: &[i64]) -> Table {
    table(
        &[CellKind::Number],
        &values
            .iter()
            .map(|value| vec![n(*value)])
            .collect::<Vec<_>>(),
    )
}
pub(crate) fn matrix(rows: &[&[i64]]) -> Table {
    table(
        &vec![CellKind::Number; rows.first().map_or(0, |row| row.len())],
        &rows
            .iter()
            .map(|row| row.iter().copied().map(n).collect())
            .collect::<Vec<_>>(),
    )
}
pub(super) fn text(value: &str, symbols: &mut Symbols) -> Value {
    Value::Symbol(query_intern_derived(
        &mut symbols.into(),
        ByteSlice(SOURCE),
        NameText(value),
    ))
}
