//! The `capture` operator: match paths against a pattern with named slots and append what the slots matched.
use super::super::interface::OperatorsItem;
use super::constants::CROSSING_SLOT;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::service::minted;
use crate::domain::operating::types::AfterTheFiles;
use crate::domain::operator::collections::operators::types::{PathTail, SlotPattern, WholePath};
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use crate::domain::table_model::service::cell;
use crate::domain::table_model::types::ParameterBinding;
use crate::domain::table_scratch::types::Stretches;
use crate::domain::table_texts::service::datum;
use crate::domain::table_texts::types::{Datum, Texts};
use ryo_language::domain::symbols::service::query_intern_derived;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::plans::types::{Operation, OperationKind};
/// Every way `pattern` matches `tail`, the end of `whole`: its text must be there, and a slot matches some text,
/// without a `/` unless it is written `{*name}`. Where each slot matched, as a range of `whole`, is
/// pushed onto `slots` in order and given to `each` when the pattern is matched whole. Nothing is allocated: the
/// pattern and the text are read as they are.
fn matches(
    pattern: SlotPattern<'_>,
    whole: WholePath<'_>,
    tail: PathTail<'_>,
    slots: &mut Stretches,
    each: &mut dyn FnMut(&Stretches),
) {
    let (pattern, whole_path, tail) = (pattern.0, whole, tail.0);
    let whole_text = whole_path.0;
    let Some(open) = pattern.find('{') else {
        if pattern == tail {
            each(slots);
        }
        return;
    };
    let (literal, after_open) = pattern.split_at(open);
    // `{*name}` takes text with `/` in it; `{name}` stops at one.
    let crosses = after_open.starts_with(CROSSING_SLOT);
    let Some(from) = tail.strip_prefix(literal) else {
        return;
    };
    let rest = after_open
        .split_once('}')
        .map(|(_, after)| after)
        .unwrap_or_default();
    let start = whole_text.len() - from.len();
    // What follows the slot is plain text: the slot is all that comes before it at the end of the text, nowhere else.
    if !rest.contains('{') {
        let Some(end) = from.len().checked_sub(rest.len()) else {
            return;
        };
        if end >= 1
            && let (Some(slot), Some(after)) = (from.get(..end), from.get(end..))
            && after == rest
            && (crosses || !slot.contains('/'))
        {
            let mark = slots.0.len();
            slots.0.push(start..start + end);
            each(slots);
            slots.0.truncate(mark);
        }
        return;
    }
    for end in 1..=from.len() {
        // Only the ends that are the boundaries of characters split the text.
        let (Some(slot), Some(after)) = (from.get(..end), from.get(end..)) else {
            continue;
        };
        if !crosses && slot.contains('/') {
            continue;
        }
        let mark = slots.0.len();
        slots.0.push(start..start + end);
        matches(SlotPattern(rest), whole_path, PathTail(after), slots, each);
        slots.0.truncate(mark);
    }
}

pub(crate) struct CaptureItem;
impl OperatorsItem for CaptureItem {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[crate::domain::operation_schema::types::InputSchema<'_>],
        bindings: &[ParameterBinding],
    ) -> Result<OperatorContract, OperatingError> {
        crate::domain::operator::contract::service::prepare_for(
            OperationKind::Capture,
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
        let PreparedInputs::Row { row, .. } = inputs else {
            return Err(OperatingError::WrongOperation);
        };
        let Operation::Capture {
            path: column,
            pattern,
            ..
        } = operation
        else {
            return Err(OperatingError::WrongOperation);
        };
        let texts = Texts {
            symbols: lease.derived.as_ref(),
            source: lease.views.source,
            paths: lease.views.paths,
            arenas: lease.views.arenas,
        };
        let Datum::Text(value) = datum(cell(row, *column)?, &texts) else {
            return Ok(());
        };
        // Own the input while derived interning may grow the symbol table.
        let mut text = std::mem::take(lease.workspace.text);
        text.clear();
        text.push_str(value.0);
        let after = if lease.views.arenas.is_empty() {
            AfterTheFiles::Before
        } else {
            AfterTheFiles::After
        };
        let mut captured = std::mem::take(lease.workspace.captured);
        captured.clear();
        let width = memchr::memchr_iter(b'{', pattern.0.as_bytes()).count();
        let mut ways = 0;
        let mut slots = std::mem::take(lease.workspace.stretches);
        slots.0.clear();
        matches(
            SlotPattern(&pattern.0),
            WholePath(&text),
            PathTail(&text),
            &mut slots,
            &mut |places| {
                for place in &places.0 {
                    let made = query_intern_derived(
                        &mut lease.derived,
                        ByteSlice(lease.views.source.0),
                        NameText(text.get(place.clone()).unwrap_or_default()),
                    );
                    captured.push(minted(lease.derived.as_ref(), made, after));
                }
                ways += 1;
            },
        );
        let mut joined = std::mem::take(lease.workspace.row);
        let result = (|| {
            for way in 0..ways {
                joined.clear();
                joined.extend_from_slice(row);
                joined.extend_from_slice(
                    captured
                        .get(way * width..(way + 1) * width)
                        .unwrap_or_default(),
                );
                emit.row(lease, &joined)?;
            }
            Ok(())
        })();
        *lease.workspace.row = joined;
        *lease.workspace.captured = captured;
        *lease.workspace.text = text;
        *lease.workspace.stretches = slots;
        result
    }
}
