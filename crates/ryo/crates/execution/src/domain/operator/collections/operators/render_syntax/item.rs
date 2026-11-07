//! One candidate node is rendered only after its input row reaches this operator.

use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::service::minted;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};

use ryo_language::domain::semantics::types::{RenderOutcome, RenderRequest};
use ryo_language::domain::symbols::service::query_intern_derived;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::types::Value;
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::rows::service::cell;

pub(crate) struct RenderSyntaxItem;

impl OperatorsItem for RenderSyntaxItem {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[crate::domain::operation_schema::types::InputShape<'_>],
        bindings: &[ParameterBinding],
    ) -> Result<OperatorContract, OperatingError> {
        crate::domain::operator::contract::service::prepare_for(
            OperationKind::RenderSyntax,
            operation,
            inputs,
            bindings,
        )
    }

    fn evaluate(
        &self,
        operation: &Operation,
        inputs: PreparedInputs<'_>,
        lease: &mut PrimitiveLease<'_>,
        emit: &mut dyn RowSink,
    ) -> Result<(), OperatingError> {
        let PreparedInputs::Views(inputs) = inputs else {
            return Err(OperatingError::WrongOperation);
        };
        let source = inputs.first().ok_or(OperatingError::NotRun)?;
        let Operation::RenderSyntax { node, context, .. } = operation else {
            return Err(OperatingError::WrongOperation);
        };
        let mut row = std::mem::take(&mut lease.workspace.row);
        let mut joined = std::mem::take(&mut lease.workspace.other);
        let outcome = (|| {
            let mut requests = Vec::with_capacity(source.count());
            for position in 0..source.count() {
                let values = source
                    .read(RowIndex(position), &mut row)
                    .ok_or(OperatingError::NotRun)?;
                let Value::Node(node) = cell(values, *node)? else {
                    return Err(OperatingError::ShapeMismatch);
                };
                requests.push(RenderRequest {
                    node,
                    context: *context,
                });
            }
            if requests.is_empty() {
                return Ok(());
            }
            let lookup =
                lease
                    .views
                    .semantic
                    .ok_or_else(|| OperatingError::SemanticIncomplete {
                        reason: super::constants::NO_SEMANTIC_SESSION.into(),
                    })?;
            let answers = lookup.render_batch(&requests, lease.views.source.0)?;
            if answers.len() != requests.len() {
                return Err(OperatingError::SemanticBatchShape);
            }
            // A provider's unsupported result invalidates this consuming rule.
            // Check the whole batch before publishing any derived row.
            for answer in &answers {
                if let RenderOutcome::Unsupported(reason) = answer {
                    return Err(OperatingError::SemanticIncomplete {
                        reason: format!(
                            "selected language cannot render this syntax (reason {})",
                            reason.0
                        ),
                    });
                }
            }
            let after = lease.views.phase;
            for (position, answer) in answers.into_iter().enumerate() {
                let text = match answer {
                    RenderOutcome::Known(text) => text.0,
                    RenderOutcome::NotApplicable => continue,
                    RenderOutcome::Unsupported(_) => unreachable!("checked before publication"),
                };
                let values = source
                    .read(RowIndex(position), &mut row)
                    .ok_or(OperatingError::NotRun)?;
                let made = query_intern_derived(
                    &mut lease.derived,
                    ByteSlice(lease.views.source.0),
                    NameText(&text),
                );
                joined.clear();
                joined.extend_from_slice(values);
                joined.push(minted(lease.derived.as_ref(), made, after));
                emit.row(lease, &joined)?;
            }
            Ok(())
        })();
        lease.workspace.row = row;
        lease.workspace.other = joined;
        outcome
    }
}

use ryo_values::domain::rows::types::RowIndex;
