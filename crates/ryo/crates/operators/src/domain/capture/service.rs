//! Enumerate path captures without allocating or splitting UTF-8 characters.
use crate::domain::capture::constants::CROSSING_OPEN;
use crate::domain::capture::types::{
    CaptureOutput, CaptureSlotCount, CaptureSlots, CaptureSpans, SlotPattern, SlotSubject,
};

/// How many slots `pattern` has, each a `{name}` or `{*name}`: the columns a capture appends to each row.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::capture::service::slot_count;
/// use ryo_operators::domain::capture::types::{CaptureSlotCount, SlotPattern};
/// assert_eq!(slot_count(SlotPattern("src/{crate}/{*rest}")), CaptureSlotCount(2));
/// ```
#[must_use]
pub fn slot_count(pattern: SlotPattern<'_>) -> CaptureSlotCount {
    CaptureSlotCount(pattern.0.bytes().filter(|byte| *byte == b'{').count())
}

/// `capture`: every way the slot pattern matches the whole path, each handed to `each` as the ranges of its slots in
/// slot order. `whole` is the path and `tail` the part of it not yet matched (the path itself for a caller); `slots` is
/// the caller's scratch for the slots matched so far. A pattern of no slot matches once when it is the path.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::capture::service::matches;
/// use ryo_operators::domain::capture::types::{CaptureOutput, CaptureSlots, CaptureSpans, SlotPattern, SlotSubject};
///
/// struct Ways(Vec<Vec<std::ops::Range<usize>>>);
/// impl CaptureOutput for Ways {
///     fn captures(&mut self, ranges: CaptureSpans<'_>) {
///         self.0.push(ranges.0.to_vec());
///     }
/// }
/// let (mut slots, mut ways) = (Vec::new(), Ways(Vec::new()));
/// let path = "src/a.rs";
/// matches(SlotPattern("src/{name}.rs"), SlotSubject(path), SlotSubject(path), CaptureSlots(&mut slots), &mut ways);
/// assert_eq!(ways.0, [vec![4..5]]);
/// ```
pub fn matches(
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
