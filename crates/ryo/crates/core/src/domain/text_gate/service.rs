//! The `text_gate` service: which elements of a tree their text admits, with no IO and no ports.

use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::relations::constants::NO_SYMBOL;
use ryo_language::domain::relations::types::{CellKind, Value};
use ryo_language::domain::terms::types::Symbol;

use crate::domain::comparing::service::holds_some;
use crate::domain::kind_sets::service::names_a_kind;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{KindNaming, KindNarrowing};
use crate::domain::operating::types::{NameCount, OperatorContext};
use crate::domain::table_cells::types::{ColumnPlaces, TreeForm, TreeRows};
use crate::domain::table_model::types::Verdict;
use crate::domain::table_scratch::types::{
    Candidates, ComparisonNumbers, ElementIndex, ElementPlaces, KindsBy, TextGrouping, TextSlot,
    TextStarts, TextVerdicts, TextedFile, TreeSize,
};
use ryo_values::domain::plan_parts::service::each_column;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Comparison, Operand};

/// The comparisons that ask of the text of an element something that does not depend on the element,
/// by their place among the comparisons: what is decided once for each text and not for each element.
pub(crate) fn text_gate(
    tree: &TreeRows<'_>,
    comparisons: &[Comparison],
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
    comparisons: &[Comparison],
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
    comparisons: &[Comparison],
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
    comparisons: &[Comparison],
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

/// The elements of a file by their text, counted and then placed into `order`, each text's run starting where `starts` says.
fn group_by_text(
    elements: &Elements,
    names: NameCount,
    starts: &mut TextStarts,
    order: &mut ElementPlaces,
) {
    // The elements by their text, counted and then placed: the texts of the elements were made when the table was built, and
    // the names a rule makes after are not among them.
    let built = names.0;
    let slots = built + 1;
    let starts = &mut starts.0;
    starts.clear();
    starts.resize(slots + 1, 0);
    let slot_of = |text: Symbol| -> TextSlot {
        TextSlot(if text == NO_SYMBOL {
            built
        } else {
            usize::try_from(text.0).map_or(built, |slot| slot.min(built))
        })
    };
    for text in elements.text.iter() {
        if let Some(count) = starts.get_mut(slot_of(*text).0 + 1) {
            *count += 1;
        }
    }
    let mut running = 0;
    for start in starts.iter_mut() {
        running += *start;
        *start = running;
    }
    let order = &mut order.0;
    order.clear();
    order.resize(elements.text.len(), 0);
    let mut filled = starts.clone();
    for (place, text) in elements.text.iter().enumerate() {
        if let Some(next) = filled.get_mut(slot_of(*text).0) {
            if let Some(at) = order.get_mut(usize::try_from(*next).unwrap_or(usize::MAX)) {
                *at = u32::try_from(place).unwrap_or(u32::MAX);
            }
            *next += 1;
        }
    }
}

/// The places of the elements of a tree whose text the comparisons of the gate admit, found among the elements of the texts that
/// do: decided once for each text of the file, and the elements of a text found by the file's order of its elements by text, made
/// once for the file and read by every select that asks. Nothing narrower is claimed when the gate admits an element with no text or
/// admits a fourth of the elements: the whole tree is then looked at, with the texts already decided in `text_ok`.
#[expect(
    clippy::too_many_arguments,
    reason = "the narrowing reads the gate's comparisons, the file's verdicts, the name count, the row buffer, the kinds and the evaluation context, each a separate borrow of the caller's room, and writes the candidates"
)]
pub(crate) fn text_candidates(
    tree: &TreeRows<'_>,
    comparisons: &[Comparison],
    gate: &ComparisonNumbers,
    text_ok: &mut TextVerdicts,
    names: NameCount,
    buffer: &mut [Value],
    kinds: &[CellKind],
    context: &mut OperatorContext<'_>,
    candidates: &mut Candidates,
) -> Result<KindNarrowing, OperatingError> {
    let candidates = &mut candidates.0;
    let elements = tree.elements;
    let total = elements.text.len();
    let key = TextedFile {
        file: tree.file,
        elements: TreeSize(total),
    };
    if context.scratch.select_scratch.text_file != TextGrouping::Made(key) {
        let select = &mut *context.scratch.select_scratch;
        group_by_text(
            elements,
            names,
            &mut select.text_starts,
            &mut select.text_order,
        );
        context.scratch.select_scratch.text_file = TextGrouping::Made(key);
    }
    let starts = std::mem::take(&mut context.scratch.select_scratch.text_starts.0);
    let order = std::mem::take(&mut context.scratch.select_scratch.text_order.0);
    let built = starts.len().saturating_sub(2);
    let span = |pair: &[_]| match pair {
        [from, to] => (
            usize::try_from(*from).unwrap_or(0),
            usize::try_from(*to).unwrap_or(0),
        ),
        _ => (0, 0),
    };
    let outcome = (|| {
        let mut admitted = 0_usize;
        let mut chosen: Vec<(usize, usize)> = Vec::new();
        for (slot, pair) in starts.windows(2).enumerate().take(built) {
            let (from, to) = span(pair);
            if from == to {
                continue;
            }
            let text = Symbol(u32::try_from(slot).unwrap_or(u32::MAX));
            let verdict = decided(
                text,
                comparisons,
                gate,
                text_ok,
                names,
                buffer,
                kinds,
                context,
            )?;
            if verdict == Verdict::Holds {
                admitted += to - from;
                chosen.push((from, to));
            }
        }
        // The elements that have no text (nodes, and tokens nobody reads) are decided as one text.
        let (from, to) = starts.get(built..built + 2).map_or((0, 0), span);
        if from != to
            && decided(
                NO_SYMBOL,
                comparisons,
                gate,
                text_ok,
                names,
                buffer,
                kinds,
                context,
            )? == Verdict::Holds
        {
            return Ok(KindNarrowing::Whole);
        }
        if admitted * 4 > total {
            return Ok(KindNarrowing::Whole);
        }
        candidates.clear();
        for (start, end) in &chosen {
            candidates.extend_from_slice(order.get(*start..*end).unwrap_or_default());
        }
        if chosen.len() > 1 {
            candidates.sort_unstable();
        }
        Ok(KindNarrowing::Narrowed)
    })();
    context.scratch.select_scratch.text_starts.0 = starts;
    context.scratch.select_scratch.text_order.0 = order;
    outcome
}

/// The places of the columns the comparisons read, each once, in order.
pub(crate) fn needed_of(comparisons: &[Comparison], needed: &mut ColumnPlaces) {
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
    comparisons: &[Comparison],
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
