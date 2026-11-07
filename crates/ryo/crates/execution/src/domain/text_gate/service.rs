//! The `text_gate` service: which elements of a tree their text admits, with no IO and no ports.

use ryo_language::domain::elements::types::Elements;
use ryo_relations::domain::relations::constants::NO_SYMBOL;
use ryo_relations::domain::relations::types::{CellKind, Value};
use ryo_relations::domain::terms::types::Symbol;

use crate::domain::comparing::service::holds_some;
use crate::domain::kind_sets::service::names_a_kind;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::KindNaming;
use crate::domain::operating::types::{NameCount, OperatorContext};
use crate::domain::table_cells::types::{ColumnPlaces, TreeForm, TreeRows};
use crate::domain::table_model::types::Verdict;
use crate::domain::table_scratch::types::{ ComparisonNumbers, ElementIndex, KindsBy, TextVerdicts};
use ryo_values::domain::plan_parts::service::each_column;
use ryo_values::domain::plan_parts::types::{ColumnIndex, PlanComparison, Operand};

/// The comparisons that ask of the text of an element something that does not depend on the element,
/// by their place among the comparisons: what is decided once for each text and not for each element.
pub(crate) fn text_gate(
    tree: &TreeRows<'_>,
    comparisons: &[PlanComparison],
    gate: &mut ComparisonNumbers,
) {
    let gate = &mut gate.0;
    gate.clear();
    if tree.form != TreeForm::Elements {
        return;
    }
    for (number, comparison) in comparisons.iter().enumerate() {
        if matches!(
            (&comparison.left, &comparison.right),
            (
                Operand::Column(ColumnIndex(2)),
                Operand::Parameter(_) | Operand::Literal(_)
            ) | (
                Operand::Parameter(_) | Operand::Literal(_),
                Operand::Column(ColumnIndex(2))
            )
        ) {
            gate.push(number);
        }
    }
}

/// The comparisons the kind sets and the text gate have not decided, by their place among `comparisons`, into `rest`.
pub(crate) fn undecided(
    comparisons: &[PlanComparison],
    gate: &ComparisonNumbers,
    kinds_by: KindsBy,
    rest: &mut ComparisonNumbers,
) {
    let rest = &mut rest.0;
    rest.clear();
    rest.extend((0..comparisons.len()).filter(|number| {
        let decided = gate.0.contains(number)
            || (kinds_by == KindsBy::Sets
                && comparisons
                    .get(*number)
                    .is_some_and(|each| names_a_kind(each) == KindNaming::Names));
        !decided
    }));
}

/// Whether the text of the element at `index` is admitted by the comparisons of `gate`: decided once for each text
/// of the file, whichever elements have it.
#[expect(
    clippy::too_many_arguments,
    reason = "one decision reads the gate's comparisons, the file's verdicts, the name count, the row buffer, the kinds and the evaluation context, each a separate borrow of the caller's room"
)]
pub(crate) fn text_admitted(
    tree: &TreeRows<'_>,
    index: ElementIndex,
    comparisons: &[PlanComparison],
    gate: &ComparisonNumbers,
    text_ok: &mut TextVerdicts,
    names: NameCount,
    buffer: &mut [Value],
    kinds: &[CellKind],
    context: &mut OperatorContext<'_>,
) -> Result<Verdict, OperatingError> {
    let Some(&text) = tree.elements.text.get(index.0) else {
        return Ok(Verdict::Fails);
    };
    let slot = if text == NO_SYMBOL {
        names.0
    } else {
        usize::try_from(text.0).unwrap_or(usize::MAX)
    };
    match text_ok.0.get(slot).copied() {
        Some(1) => Ok(Verdict::Holds),
        Some(2) | None => Ok(Verdict::Fails),
        Some(_) => {
            if let Some(cell) = buffer.get_mut(2) {
                *cell = Value::Symbol(text);
            }
            let admitted = holds_some(comparisons, gate, buffer, kinds, names, context)?;
            if let Some(verdict) = text_ok.0.get_mut(slot) {
                // A verdict is kept as 1 when the text holds and 2 when it fails; 0 is a text not yet decided.
                *verdict = if admitted == Verdict::Holds { 1 } else { 2 };
            }
            Ok(admitted)
        }
    }
}

/// Whether the text `text` holds, deciding it first when it has not been: `text` stands in the text column of `buffer`.
#[expect(
    clippy::too_many_arguments,
    reason = "one decision reads the gate's comparisons, the file's verdicts, the name count, the row buffer, the kinds and the evaluation context, each a separate borrow of the caller's room"
)]
fn decided(
    text: Symbol,
    comparisons: &[PlanComparison],
    gate: &ComparisonNumbers,
    text_ok: &mut TextVerdicts,
    names: NameCount,
    buffer: &mut [Value],
    kinds: &[CellKind],
    context: &mut OperatorContext<'_>,
) -> Result<Verdict, OperatingError> {
    let text_ok = &mut text_ok.0;
    // The text's place among the verdicts: its symbol, or the one past the names for the elements with no text.
    let slot = if text == NO_SYMBOL {
        names.0
    } else {
        usize::try_from(text.0).unwrap_or(usize::MAX)
    };
    if text_ok.get(slot) == Some(&0) {
        if let Some(cell) = buffer.get_mut(2) {
            *cell = Value::Symbol(text);
        }
        let verdict = holds_some(comparisons, gate, buffer, kinds, names, context)?;
        if let Some(kept) = text_ok.get_mut(slot) {
            *kept = if verdict == Verdict::Holds { 1 } else { 2 };
        }
    }
    Ok(if text_ok.get(slot) == Some(&1) {
        Verdict::Holds
    } else {
        Verdict::Fails
    })
}

/// The places of the columns the comparisons read, each once, in order.
pub(crate) fn needed_of(comparisons: &[PlanComparison], needed: &mut ColumnPlaces) {
    let needed = &mut needed.0;
    needed.clear();
    for comparison in comparisons {
        for operand in [&comparison.left, &comparison.right] {
            each_column(operand, &mut |index| {
                needed.push(usize::try_from(index.0).unwrap_or(usize::MAX));
            });
        }
    }
    needed.sort_unstable();
    needed.dedup();
}

/// The places of the columns the comparisons numbered `which` read, each once, in order.
pub(crate) fn needed_of_rest(
    comparisons: &[PlanComparison],
    which: &ComparisonNumbers,
    needed: &mut ColumnPlaces,
) {
    let needed = &mut needed.0;
    needed.clear();
    for number in &which.0 {
        let Some(comparison) = comparisons.get(*number) else {
            continue;
        };
        for operand in [&comparison.left, &comparison.right] {
            each_column(operand, &mut |index| {
                needed.push(usize::try_from(index.0).unwrap_or(usize::MAX));
            });
        }
    }
    needed.sort_unstable();
    needed.dedup();
}
