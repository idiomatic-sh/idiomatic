//! Native batch semantics over already-demanded inputs. This service owns no scheduling.
use crate::domain::operating::{
    errors::OperatingError,
    types::{Emit, OperatorContext, StartCount, TreeShape},
};
use crate::domain::operator::contract::{resources::ResourceProgram, types::OperatorContract};
use crate::domain::table_cells::types::TreeForm;
use crate::domain::table_model::{
    service::{check_columns, count_of, kinds_of, positions, read_row},
    types::Rows,
};
use crate::domain::tree_join::service::join_tree;
use crate::domain::tree_walk::service::{entity_of, give_walk, start_walk, walk_kinds, walk_row};
use ryo_values::domain::plan_parts::types::ColumnIndex;
pub(crate) fn evaluate(
    semantics: &dyn crate::domain::operator::contract::types::PrimitiveSemantics,
    contract: &OperatorContract,
    mode: crate::domain::operator::contract::types::EvaluationMode,
    workspace: &super::super::workspace::InvocationWorkspace,
    inputs: &[Rows<'_>],
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
) -> Result<bool, OperatingError> {
    match &contract.resources {
        ResourceProgram::Join(join) if join.reach.is_none() => {
            let [left, Rows::Tree(tree), ..] = inputs else {
                return Ok(false);
            };
            // Empty-left native execution performs neither acquisition nor RHS reads.
            if count_of(left).0 == 0 {
                return Ok(true);
            }
            let use_ = super::super::invocation::prepared(
                contract,
                mode,
                ryo_derived::domain::contract::types::DerivedKind::NativeLookup,
            )?;
            let rows = [
                crate::domain::operator::contract::views::BorrowedRows::from(left),
                crate::domain::operator::contract::views::BorrowedRows::Tree(*tree),
            ];
            let Some(acquired) = workspace.inputs(&rows).try_native_lookup(use_)? else {
                return Ok(false);
            };
            let ryo_derived::domain::contract::types::DerivedView::NativeLookup(finding) =
                acquired.view()?
            else {
                return Err(OperatingError::WrongOperation);
            };
            join_tree(
                semantics,
                &join.left_on,
                tree,
                left,
                *finding,
                TreeShape {
                    comparisons: &contract.predicates.comparisons,
                    view: &join.view,
                    output: &join.output,
                },
                context,
                emit,
            )?;
            Ok(true)
        }
        ResourceProgram::Axis(axis) => {
            let [left, Rows::Tree(tree), domain, ..] = inputs else {
                return Ok(false);
            };
            let native_domain = matches!(domain,Rows::Tree(each) if each.form==TreeForm::Elements)
                && entity_of(&axis.domain_view, axis.domain_id) == ColumnIndex(0);
            if tree.form != TreeForm::Edges
                || (axis.parent, axis.child) != (ColumnIndex(0), ColumnIndex(1))
                || !(!axis.has_filter && axis.domain_view.is_empty() || native_domain)
            {
                return Ok(false);
            }
            check_columns(kinds_of(left), &[axis.id])?;
            let reached = if let Rows::Tree(each) = domain
                && each.form == TreeForm::Elements
            {
                each
            } else {
                tree
            };
            let mut schema = ryo_values::domain::rows::types::Table::default();
            walk_kinds(
                kinds_of(left),
                kinds_of(&Rows::Tree(*reached)),
                &axis.output,
                &mut schema,
            )?;
            let mut walk = start_walk(
                &contract.identity,
                tree,
                domain,
                Some(StartCount(count_of(left).0)),
                context,
            )?;
            let mut buffer = std::mem::take(&mut context.scratch.row);
            let mut answer = std::mem::take(&mut context.scratch.other);
            let result: Result<(), OperatingError> = (|| {
                for at in positions(count_of(left)) {
                    let row =
                        read_row(left, at, &mut buffer).ok_or(OperatingError::ShapeMismatch)?;
                    walk_row(&mut walk, row, context, emit, &mut answer)?;
                }
                Ok(())
            })();
            context.scratch.row = buffer;
            context.scratch.other = answer;
            give_walk(walk, context);
            result?;
            Ok(true)
        }
        _ => Ok(false),
    }
}
