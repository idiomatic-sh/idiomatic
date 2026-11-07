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
