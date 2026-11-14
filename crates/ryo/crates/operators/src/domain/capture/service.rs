//! Enumerate path captures without allocating or splitting UTF-8 characters.
use crate::domain::capture::constants::CROSSING_OPEN;
use crate::domain::capture::types::{
    CaptureOutput, CaptureSlots, CaptureSpans, SlotPattern, SlotSubject,
};

pub(crate) fn matches(
    pattern: SlotPattern<'_>,
    whole: SlotSubject<'_>,
    tail: SlotSubject<'_>,
    slots: CaptureSlots<'_>,
    each: &mut dyn CaptureOutput,
) {
    let (pattern, whole, tail, slots) = (pattern.0, whole.0, tail.0, slots.0);
    let Some(open) = pattern.find('{') else {
        if pattern == tail {
            each.captures(CaptureSpans(slots));
        }
        return;
    };
    let (literal, after_open) = pattern.split_at(open);
    // `{*name}` takes text with `/` in it; `{name}` stops at one.
    let crosses = after_open.starts_with(CROSSING_OPEN);
    let Some(from) = tail.strip_prefix(literal) else {
        return;
    };
    let rest = after_open
        .split_once('}')
        .map(|(_, after)| after)
        .unwrap_or_default();
    let start = whole.len() - from.len();
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
            let mark = slots.len();
            slots.push(start..start + end);
            each.captures(CaptureSpans(slots));
            slots.truncate(mark);
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
        let mark = slots.len();
        slots.push(start..start + end);
        matches(
            SlotPattern(rest),
            SlotSubject(whole),
            SlotSubject(after),
            CaptureSlots(&mut *slots),
            each,
        );
        slots.truncate(mark);
    }
}

/// Matches the text of a scheduled row's path against the pattern and emits the row once for each complete
/// match, with what the slots matched appended; a path that is not text has no row.
pub(crate) fn evaluate(
    row: &[ryo_relations::domain::relations::types::Value],
    path: ryo_values::domain::plan_parts::types::ColumnIndex,
    pattern: SlotPattern<'_>,
    driver: &mut dyn crate::domain::contract::types::RowDriver,
    scratch: &mut crate::domain::contract::types::RowScratch,
) -> Result<(), crate::domain::contract::types::PortableOperatorFault> {
    let value = ryo_values::domain::rows::service::column_in(row, path)
        .copied()
        .ok_or(
            crate::domain::contract::types::PortableOperatorFault::NoSuchColumn { index: path },
        )?;
    let crate::domain::contract::types::RowScratch {
        row: joined,
        captured,
        text,
        slots,
        ..
    } = scratch;
    text.0.clear();
    if driver.text(value, text) == crate::domain::contract::types::TextFound::NotText {
        return Ok(());
    }
    let text: &str = text.0.as_str();
    captured.clear();
    slots.0.clear();
    let width = pattern.0.bytes().filter(|byte| *byte == b'{').count();
    let mut minting = crate::domain::capture::types::SlotMinting {
        driver: &mut *driver,
        text: SlotSubject(text),
        captured: &mut *captured,
        ways: ryo_relations::domain::relations::types::Count(0),
    };
    matches(
        pattern,
        SlotSubject(text),
        SlotSubject(text),
        CaptureSlots(&mut slots.0),
        &mut minting,
    );
    let ways = minting.ways;
    for way in 0..ways.0 {
        joined.clear();
        joined.extend_from_slice(row);
        joined.extend_from_slice(
            captured
                .get(way * width..(way + 1) * width)
                .unwrap_or_default(),
        );
        driver.emit(joined)?;
    }
    Ok(())
}

/// The one lowering of a capture: its path column and slot pattern, for the `text_map` kernel.
pub(crate) fn lower(
    path: ryo_values::domain::plan_parts::types::ColumnIndex,
    pattern: &ryo_values::domain::plan_parts::types::CapturePattern,
) -> super::types::CaptureParts {
    super::types::CaptureParts {
        path,
        pattern: pattern.clone(),
    }
}
