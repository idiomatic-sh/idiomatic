//! The support of the cases of the views an invocation's admitted inputs demand: an owner over the fake derived port,
//! number tables, prepared view uses and an operator context.
use crate::application::file_dag::relations::service::RelationOwner;
use crate::application::file_dag::relations::types::ReuseEligibility;
use ryo_execution_domain::domain::derived_views::types::contract::value::PreparedViewUse;
use ryo_execution_domain::domain::derived_views::types::{InputsUsed, ViewRequest};
use ryo_execution_domain::domain::operation_schema::types::InputShape;
use ryo_execution_domain::domain::operator::types::OperatorContext;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value};
use ryo_values::domain::plan_parts::types::InputNumber;
use ryo_values::domain::rows::types::{RowCount, Table};
use std::rc::Rc;
use std::sync::Arc;

/// A relation owner over the fake derived port.
pub(crate) fn owner() -> Rc<RelationOwner> {
    Rc::new(RelationOwner::new(
        ReuseEligibility::Eligible,
        Arc::new(crate::application::file_dag::fixtures::derived()),
    ))
}

/// The number table of `values`, `width` cells a row.
pub(crate) fn table(values: &[i64], width: usize) -> Table {
    Table {
        kinds: vec![CellKind::Number; width],
        cells: values
            .iter()
            .map(|value| Value::Number(Integer(*value)))
            .collect(),
        count: RowCount(values.len() / width),
    }
}

/// The use of `request` over number inputs of `width` columns, reading the first `len` inputs.
pub(crate) fn use_(request: &ViewRequest, width: usize, len: u8) -> PreparedViewUse {
    let kinds = vec![CellKind::Number; width];
    let schemas = [InputShape {
        kinds: &kinds,
        tree: None,
    }; 2];
    PreparedViewUse {
        plan: Arc::new(
            crate::application::file_dag::derived::prepare::service::view(
                Arc::new(crate::application::file_dag::fixtures::derived()),
                request,
                &schemas[..usize::from(len.clamp(1, 2))],
            )
            .expect("the fake port prepares every request"),
        ),
        inputs: [InputNumber(0), InputNumber(1)],
        relations: [None, None],
        len: InputsUsed(len),
    }
}

/// An operator context over no source.
pub(crate) fn context<'a>(
    symbols: &'a mut Symbols,
    scratch: &'a mut ryo_execution_domain::domain::table_scratch::types::OperatorScratch,
    memo: &'a mut Vec<ryo_execution_domain::domain::operating::types::Memo>,
) -> OperatorContext<'a> {
    OperatorContext {
        phase: ryo_execution_domain::domain::operating::types::AfterTheFiles::Before,
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        matcher: ryo_execution_domain::domain::matching::types::PatternMatcher(
            &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
        ),
        bindings: &[],
        symbols: symbols.into(),
        source: ryo_values::domain::files::types::SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo,
        scratch,
    }
}
