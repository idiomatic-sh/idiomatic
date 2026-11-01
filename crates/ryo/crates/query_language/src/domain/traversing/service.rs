//! The `traversing` service: the calls that walk an axis or close a relation (`axis`, `closure` and `cycles`), lowered to steps.

use ryo_language::domain::relations::types::{CellKind, Relation};
use ryo_language::domain::terms::types::Vocabulary;

use crate::domain::lowering::constants::{HierarchyColumn, heading};
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::{ArgumentFault, Builder, PlannedColumn, Sourced, Uniqueness};
use crate::domain::planning::service::push;
use crate::domain::resolving::service::{
    bad_argument, column_of, input_argument, position, positions_of, word_of, words_of,
};
use crate::domain::syntax::constants::direction_named;
use crate::domain::syntax::types::ArgumentSlot;
use ryo_values::domain::plan_parts::types as parts;
use ryo_values::domain::plans::types as plans;
use ryo_values::domain::plans::types::QueryOperator;
use ryo_values::domain::program::types::{self as syntax, QuerySpelling};

/// Lowers an `axis` call: the rows of its input joined to the elements the relation's edges reach from them, as the step `name`.
pub(crate) fn call_axis(
    builder: &mut Builder<'_>,
    name: &syntax::StepName,
    call: &syntax::Call,
) -> Result<(), LoweringError> {
    let Sourced {
        input: from,
        mut fields,
    } = input_argument(builder, call, ArgumentSlot::From)?;
    let Sourced {
        input: edges,
        fields: edge_fields,
    } = input_argument(builder, call, ArgumentSlot::Relation)?;
    let Sourced {
        input: domain,
        fields: domain_fields,
    } = input_argument(builder, call, ArgumentSlot::Domain)?;
    let direction =
        direction_named(&word_of(call, ArgumentSlot::Direction)?.0).ok_or_else(|| {
            bad_argument(
                call,
                Some(ArgumentSlot::Direction),
                ArgumentFault::ExpectedDirection,
            )
        })?;
    // A sibling is the child just before or after under the same parent: only the tree's own edges keep that order.
    if matches!(
        direction,
        parts::Direction::PreviousSibling | parts::Direction::NextSibling
    ) && edges != parts::Input::Relation(Relation::SyntaxChild)
    {
        return Err(bad_argument(
            call,
            Some(ArgumentSlot::Relation),
            ArgumentFault::SiblingOverTree,
        ));
    }
    let operation = plans::Operation::Axis {
        from,
        edges,
        domain,
        direction,
        id: position(
            &fields,
            QuerySpelling(heading(HierarchyColumn::Id)),
            call.span,
        )?,
        parent: position(
            &edge_fields,
            QuerySpelling(heading(HierarchyColumn::Parent)),
            call.span,
        )?,
        child: position(
            &edge_fields,
            QuerySpelling(heading(HierarchyColumn::Child)),
            call.span,
        )?,
        domain_id: position(
            &domain_fields,
            QuerySpelling(heading(HierarchyColumn::Id)),
            call.span,
        )?,
        domain_filter: Vec::new(),
        domain_view: Vec::new(),
        output: Vec::new(),
    };
    fields.push(PlannedColumn {
        name: syntax::ColumnName(Box::from(heading(HierarchyColumn::Related))),
        kind: CellKind::Node,
        unique: Uniqueness::Shared,
        words: Vocabulary::Open,
    });
    push(builder, Some(name), operation, fields);
    Ok(())
}

/// Lowers a `closure` or `cycles` call over the source and target columns of its edges, as the step `name`.
pub(crate) fn call_closure(
    builder: &mut Builder<'_>,
    name: &syntax::StepName,
    call: &syntax::Call,
    operator: QueryOperator,
) -> Result<(), LoweringError> {
    let Sourced {
        input: edges,
        fields,
    } = input_argument(builder, call, ArgumentSlot::Edges)?;
    let source = position(
        &fields,
        QuerySpelling(&word_of(call, ArgumentSlot::Source)?.0),
        call.span,
    )?;
    let destination = position(
        &fields,
        QuerySpelling(&word_of(call, ArgumentSlot::Destination)?.0),
        call.span,
    )?;
    if operator == QueryOperator::Closure {
        let keys = if call
            .arguments
            .iter()
            .any(|argument| &*argument.name.0 == "keys")
        {
            positions_of(builder, &fields, &words_of(call, ArgumentSlot::Keys)?)?
        } else {
            Vec::new()
        };
        for (at, key) in keys.iter().enumerate() {
            if keys[..at].contains(key) || *key == source || *key == destination {
                return Err(LoweringError::DuplicateOutput {
                    name: column_of(&fields, *key).name.0.to_string(),
                    span: call.span,
                });
            }
        }
        let kept: Vec<_> = keys
            .iter()
            .chain([&source, &destination])
            .map(|column| {
                let mut field = column_of(&fields, *column).clone();
                field.unique = Uniqueness::Shared;
                field
            })
            .collect();
        push(
            builder,
            Some(name),
            plans::Operation::Closure {
                edges,
                keys,
                source,
                destination,
            },
            kept,
        );
    } else {
        // The entities that can reach themselves, each as a row with the entity in both columns.
        let kept = vec![
            column_of(&fields, source).clone(),
            column_of(&fields, destination).clone(),
        ];
        push(
            builder,
            Some(name),
            plans::Operation::Cycles {
                edges,
                source,
                destination,
            },
            kept,
        );
    }
    Ok(())
}
