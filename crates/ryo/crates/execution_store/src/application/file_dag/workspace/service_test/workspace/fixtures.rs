//! The support of the `workspace` cases.
use super::super::super::*;
use ryo_derived::domain::request::types::DerivedKind;
use ryo_execution_domain::domain::{
    derived_views::types::{contract::value::PreparedViewUse, *},
    operation_schema::types::InputShape,
    operator::types::OperatorContext,
};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value};
use ryo_values::domain::rows::types::RowCount;
use std::{rc::Rc, sync::Arc};

pub(super) fn table(values: &[i64], width: usize) -> Table {
    Table {
        kinds: vec![CellKind::Number; width],
        cells: values
            .iter()
            .map(|value| Value::Number(Integer(*value)))
            .collect(),
        count: RowCount(values.len() / width),
    }
}

pub(super) fn use_(
    kind: DerivedKind,
    request: ViewRequest,
    schemas: &[&[CellKind]],
    len: u8,
) -> PreparedViewUse {
    let schemas = schemas
        .iter()
        .map(|kinds| InputShape { kinds, tree: None })
        .collect::<Vec<_>>();
    PreparedViewUse {
        plan: {
            let plan = crate::application::file_dag::derived::prepare::view(
                std::sync::Arc::new(crate::application::file_dag::fixtures::derived()),
                &request,
                &schemas,
            )
            .unwrap();
            assert_eq!(plan.kind, kind);
            Arc::new(plan)
        },
        inputs: [
            ryo_values::domain::plan_parts::types::InputNumber(0),
            ryo_values::domain::plan_parts::types::InputNumber(1),
        ],
        relations: [None, None],
        len: InputsUsed(len),
    }
}

pub(super) fn context<'a>(
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

pub(super) fn owner() -> Rc<crate::application::file_dag::relations::RelationOwner> {
    Rc::new(crate::application::file_dag::relations::RelationOwner::new(
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        std::sync::Arc::new(crate::application::file_dag::fixtures::derived()),
    ))
}
