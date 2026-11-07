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

/// Interns what each slot of a complete match matched, and counts the matches.
struct SlotMinting<'a> {
    driver: &'a mut dyn crate::domain::evaluation::types::InvocationDriver,
    text: &'a str,
    captured: &'a mut Vec<ryo_relations::domain::relations::types::Value>,
    ways: usize,
}

impl CaptureOutput for SlotMinting<'_> {
    fn captures(&mut self, ranges: super::types::CaptureSpans<'_>) {
        for place in ranges.0 {
            let made = self
                .driver
                .mint(ryo_values::domain::row_texts::types::CellText(
                    self.text.get(place.clone()).unwrap_or_default(),
                ));
            self.captured.push(made);
        }
        self.ways += 1;
    }
}

/// Matches the text of a scheduled row's path against the pattern and emits the row once for each complete
/// match, with what the slots matched appended; a path that is not text has no row.
pub(crate) fn evaluate(
    row: &[ryo_relations::domain::relations::types::Value],
    path: ryo_values::domain::plan_parts::types::ColumnIndex,
    pattern: SlotPattern<'_>,
    driver: &mut dyn crate::domain::evaluation::types::InvocationDriver,
    scratch: &mut crate::domain::evaluation::types::EvaluationScratch,
) -> Result<(), crate::domain::contract::types::PortableOperatorFault> {
    let value = crate::domain::evaluation::service::cell(row, path)?;
    let crate::domain::evaluation::types::EvaluationScratch {
        row: joined,
        captured,
        text,
        slots,
        ..
    } = scratch;
    text.clear();
    if driver.text(value, text) == crate::domain::evaluation::types::TextFound::NotText {
        return Ok(());
    }
    let text: &str = text;
    captured.clear();
    slots.clear();
    let width = pattern.0.bytes().filter(|byte| *byte == b'{').count();
    let mut minting = SlotMinting {
        driver: &mut *driver,
        text,
        captured: &mut *captured,
        ways: 0,
    };
    matches(
        pattern,
        SlotSubject(text),
        SlotSubject(text),
        CaptureSlots(slots),
        &mut minting,
    );
    let ways = minting.ways;
    for way in 0..ways {
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
