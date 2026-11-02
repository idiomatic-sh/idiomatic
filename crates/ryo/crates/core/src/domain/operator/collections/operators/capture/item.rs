//! The `capture` operator: match paths against a pattern with named slots and append what the slots matched.

use ryo_language::domain::relations::types::{CellKind, Value};
use ryo_language::domain::symbols::service::query_intern_derived;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::terms::types::NameText;

use super::super::interface::OperatorsItem;
use super::constants::CROSSING_SLOT;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::service::first;
use crate::domain::operating::service::minted;
use crate::domain::operating::types::AfterTheFiles;
use crate::domain::operating::types::{Emit, Feed, InputPlace, OperatorContext, Room};
use crate::domain::operator::collections::operators::types::{PathTail, SlotPattern, WholePath};
use crate::domain::operator::types::SlotCount;
use crate::domain::replay::service::derive;
use crate::domain::replay::types::{Collect, Pipe};
use crate::domain::table_model::service::{cell, count_of, kinds_of, positions, read_row, reset};
use crate::domain::table_model::types::{Rows, Table};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_scratch::types::{Captured, CapturedPlace, Stretches, WayCount};
use crate::domain::table_texts::service::datum;
use crate::domain::table_texts::types::{Datum, Texts};
use ryo_values::domain::plans::types::Operation;

/// `capture`, one item of the `operators` collection.
pub(crate) struct CaptureItem;

/// How many slots a pattern like `docs/{page}.md` has: how many `{` it holds.
fn slots_of(pattern: SlotPattern<'_>) -> SlotCount {
    SlotCount(memchr::memchr_iter(b'{', pattern.0.as_bytes()).count())
}

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

/// Passes on the rows that `row` makes: it with what the slots of the pattern matched in its path, once for each way they match. A
/// path is matched once, whichever rows name it.
fn capture_row(
    operation: &Operation,
    row: &[Value],
    context: &mut OperatorContext<'_>,
    joined: &mut Vec<Value>,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError> {
    let Operation::Capture { path, pattern, .. } = operation else {
        return Err(OperatingError::WrongOperation);
    };
    let width = slots_of(SlotPattern(&pattern.0)).0;
    let named = cell(row, *path)?;
    let hit = context
        .scratch
        .capture_scratch
        .captures
        .get(&named)
        .copied();
    let (from, ways) = if let Some(Captured {
        begin: CapturedPlace(begin),
        ways: WayCount(ways),
    }) = hit
    {
        (begin, ways)
    } else {
        let OperatorContext {
            symbols,
            source,
            paths,
            arenas,
            scratch,
            ..
        } = &mut *context;
        let after = if arenas.is_empty() {
            AfterTheFiles::Before
        } else {
            AfterTheFiles::After
        };
        let OperatorScratch {
            capture_scratch, ..
        } = &mut **scratch;
        let crate::domain::table_scratch::types::CaptureScratch {
            text,
            places,
            captures,
            captured,
            ..
        } = &mut **capture_scratch;

        let from = captured.len();
        let mut ways = 0;
        text.clear();
        let texts = Texts {
            symbols: symbols.as_ref(),
            source: *source,
            paths,
            arenas,
        };
        let Datum::Text(read) = datum(named, &texts) else {
            return Ok(());
        };
        text.push_str(read.0);
        places.0.clear();
        matches(
            SlotPattern(&pattern.0),
            WholePath(text),
            PathTail(text),
            places,
            &mut |slots| {
                for place in &slots.0 {
                    let made = query_intern_derived(
                        symbols,
                        ByteSlice(source.0),
                        NameText(text.get(place.clone()).unwrap_or_default()),
                    );
                    captured.push(minted(symbols.as_ref(), made, after));
                }
                ways += 1;
            },
        );
        let _: Option<Captured> = captures.insert(
            named,
            Captured {
                begin: CapturedPlace(from),
                ways: WayCount(ways),
            },
        );
        (from, ways)
    };
    for way in 0..ways {
        joined.clear();
        joined.extend_from_slice(row);
        joined.extend_from_slice(
            context
                .scratch
                .capture_scratch
                .captured
                .get(from + way * width..from + (way + 1) * width)
                .unwrap_or_default(),
        );
        emit.row(context, joined)?;
    }
    Ok(())
}

impl OperatorsItem for CaptureItem {
    fn compute(
        &self,
        operation: &Operation,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        let Operation::Capture { pattern, .. } = operation else {
            return Err(OperatingError::WrongOperation);
        };
        let rows = first(inputs)?;
        reset(out, kinds_of(rows));
        out.kinds.extend(std::iter::repeat_n(
            CellKind::Symbol,
            slots_of(SlotPattern(&pattern.0)).0,
        ));
        context.scratch.capture_scratch.captures.clear();
        context.scratch.capture_scratch.captured.clear();
        let mut buffer = std::mem::take(&mut context.scratch.row);
        let mut joined = std::mem::take(&mut context.scratch.other);
        let outcome = (|| {
            let mut emit = Collect(out);
            for position in positions(count_of(rows)) {
                let Some(row) = read_row(rows, position, &mut buffer) else {
                    continue;
                };
                capture_row(operation, row, context, &mut joined, &mut emit)?;
            }
            Ok(())
        })();
        context.scratch.row = buffer;
        context.scratch.other = joined;
        outcome
    }

    fn stream(
        &self,
        operation: &Operation,
        ins: &mut dyn Feed<'_>,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        let Room { memo, scratch, .. } = room;
        scratch.capture_scratch.captures.clear();
        scratch.capture_scratch.captured.clear();
        let mut joined = std::mem::take(&mut scratch.other);
        let outcome = ins.stream(
            InputPlace(0),
            context,
            &mut Pipe(|each: &mut OperatorContext<'_>, row: &[Value]| {
                let mut own = derive(each, memo, scratch);
                capture_row(operation, row, &mut own, &mut joined, emit)
            }),
        );
        scratch.other = joined;
        outcome
    }
}
