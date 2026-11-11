//! The support of the `local_caches` cases.
use super::super::ordered_dfs::fixtures::execute_bound_request;
use crate::application::file_dag::fixtures::operator_contract::inputs::table_inputs;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operating::fixtures::ExecutionObservation;
use ryo_language::domain::symbols::types::Symbols;
use ryo_language::domain::symbols::{service::intern_derived, types::ByteSlice};
use ryo_relations::domain::relations::types::{CellKind, Relation, Value as Cell};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::plan_parts::types::{
    LiteralText, Operand, PlanComparison, QueryLiteral, Test,
};
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::rows::service::push_row;
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input as PlanInput, StepId, StepScope},
    plans::types::{Operation, PlanStep},
    rows::types::Table,
};

pub(super) const FROM: PlanInput = PlanInput::Relation(Relation::Function);

pub(super) fn select(test: Test, pattern: &str) -> Operation {
    Operation::Select {
        from: FROM,
        kept: vec![],
        comparisons: vec![PlanComparison {
            left: Operand::Column(ColumnIndex(0)),
            right: Operand::Literal(QueryLiteral::Text(LiteralText(pattern.into()))),
            test,
            origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
        }],
    }
}

/// The regexes and globs an invocation compiled into its local caches, with the answer's row count and the working
/// bytes left once the file finished: one text row, the
/// operation, and an undemanded literal `shadow` step whose compiled slot a dynamic pattern must not take by equal bytes.
pub(super) fn local_caches(
    operation: Operation,
    text: &str,
    bindings: &[ParameterBinding],
    shadow: &Operation,
) -> Result<(usize, usize, usize, usize), OperatingError> {
    let mut symbols = Symbols::default();
    let mut input = Table {
        kinds: vec![CellKind::Symbol],
        ..Table::default()
    };
    let symbol = intern_derived(&mut symbols, ByteSlice(b""), NameText(text));
    push_row(&mut input, &[Cell::Symbol(symbol)]);
    let mut authored = table_inputs(operation, &[&input], bindings, &mut symbols);
    let mut shadow = shadow.clone();
    ryo_values::domain::plans::service::each_input(&mut shadow, &mut |input| {
        *input = PlanInput::Step(StepId(0));
    });
    authored.plan.steps.push(PlanStep {
        scope: StepScope::PerFile,
        operation: shadow,
    });
    let mut observed = ExecutionObservation::default();
    let ran = execute_bound_request(
        authored.plan,
        bindings.to_vec(),
        &authored.hosted,
        &authored.facts,
        &mut symbols,
        b"",
        false,
        &mut observed,
    )?;
    Ok((
        ran.answer?.count.0,
        observed.regexes,
        observed.globs,
        ran.retained.0,
    ))
}

/// `operation`, a select, with its one comparison's right side replaced by `right`.
pub(super) fn comparing(mut operation: Operation, right: Operand) -> Operation {
    if let Operation::Select { comparisons, .. } = &mut operation {
        comparisons[0].right = right;
    }
    operation
}
