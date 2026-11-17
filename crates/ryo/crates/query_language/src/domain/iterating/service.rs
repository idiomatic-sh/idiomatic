//! The `iterating` service: a `worklist` call, its columns resolved and its kinds checked, lowered to one step.

use ryo_relations::domain::relations::types::CellKind;

use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::{ArgumentFault, Builder, Sourced};
use crate::domain::planning::service::push;
use crate::domain::resolving::service::{
    bad_argument, column_of, input_argument, position, positions_of, word_of, words_of,
};
use crate::domain::syntax::constants::slot_name;
use crate::domain::syntax::types::ArgumentSlot;
use crate::domain::traversing::service::graph_output_columns;
use ryo_values::domain::plans::types as plans;
use ryo_values::domain::query_tokens::types::QuerySpelling;

/// Lowers a `worklist` call: one row for each edge a run follows, with the destination's state after the edge, as the
/// step `name`.
///
/// # Errors
///
/// An argument the call leaves out or names no column of its input; a root, source and destination of different kinds;
/// an order that is not a number; a root state, before-state, after-state, lower and upper state of different kinds; root and
/// edge partitions of different kinds; a duplicate output name.
pub(crate) fn call_worklist(
    builder: &mut Builder<'_>,
    name: &ryo_values::domain::query_tokens::types::StepName,
    call: &ryo_values::domain::query_syntax::types::Call,
) -> Result<(), LoweringError> {
    let Sourced {
        input: from,
        fields: roots,
    } = input_argument(builder, call, ArgumentSlot::From)?;
    let Sourced {
        input: edges,
        fields,
    } = input_argument(builder, call, ArgumentSlot::Edges)?;
    let Sourced {
        input: lattice,
        fields: orders,
    } = input_argument(builder, call, ArgumentSlot::Lattice)?;
    let named = |columns: &[_], slot| -> Result<_, LoweringError> {
        position(columns, QuerySpelling(&word_of(call, slot)?.0), call.span)
    };
    let root = named(&roots, ArgumentSlot::Root)?;
    let state = named(&roots, ArgumentSlot::State)?;
    let source = named(&fields, ArgumentSlot::Source)?;
    let destination = named(&fields, ArgumentSlot::Destination)?;
    let order = named(&fields, ArgumentSlot::Order)?;
    let before = named(&fields, ArgumentSlot::Before)?;
    let after = named(&fields, ArgumentSlot::After)?;
    let lower = named(&orders, ArgumentSlot::Lower)?;
    let upper = named(&orders, ArgumentSlot::Upper)?;
    let entity = column_of(&roots, root).kind;
    if column_of(&fields, source).kind != entity || column_of(&fields, destination).kind != entity {
        return Err(bad_argument(
            call,
            Some(ArgumentSlot::Root),
            ArgumentFault::MatchingGraphEntities,
        ));
    }
    if column_of(&fields, order).kind != CellKind::Number {
        return Err(bad_argument(
            call,
            Some(ArgumentSlot::Order),
            ArgumentFault::NumericGraphOrder,
        ));
    }
    let kind = column_of(&roots, state).kind;
    if [
        column_of(&fields, before),
        column_of(&fields, after),
        column_of(&orders, lower),
        column_of(&orders, upper),
    ]
    .iter()
    .any(|column| column.kind != kind)
    {
        return Err(bad_argument(
            call,
            Some(ArgumentSlot::State),
            ArgumentFault::MatchingStates,
        ));
    }
    let names = if call
        .arguments
        .iter()
        .any(|argument| &*argument.name.0 == slot_name(ArgumentSlot::Keys))
    {
        words_of(call, ArgumentSlot::Keys)?
    } else {
        Vec::new()
    };
    let root_keys = positions_of(builder, &roots, &names)?;
    let edge_keys = positions_of(builder, &fields, &names)?;
    if root_keys
        .iter()
        .zip(&edge_keys)
        .any(|(left, right)| column_of(&roots, *left).kind != column_of(&fields, *right).kind)
    {
        return Err(bad_argument(
            call,
            Some(ArgumentSlot::Keys),
            ArgumentFault::MatchingGraphKeys,
        ));
    }
    let kept = graph_output_columns(
        &roots,
        &fields,
        &root_keys,
        root,
        &[source, destination, order, before],
        kind,
        call,
    )?;
    push(
        builder,
        Some(name),
        plans::Operation::Worklist {
            from,
            edges,
            lattice,
            root,
            state,
            source,
            destination,
            order,
            before,
            after,
            lower,
            upper,
            root_keys,
            edge_keys,
        },
        kept,
    );
    Ok(())
}
