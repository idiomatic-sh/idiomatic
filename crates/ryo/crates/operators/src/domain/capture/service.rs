//! Enumerate path captures without allocating or splitting UTF-8 characters.
pub(crate) fn matches(
    pattern: &str,
    whole: &str,
    tail: &str,
    slots: &mut Vec<std::ops::Range<usize>>,
    each: &mut dyn crate::domain::capture::types::CaptureOutput,
) {
    let whole_text = whole;
    let Some(open) = pattern.find('{') else {
        if pattern == tail {
            each.captures(slots);
        }
        return;
    };
    let (literal, after_open) = pattern.split_at(open);
    // `{*name}` takes text with `/` in it; `{name}` stops at one.
    let crosses = after_open.starts_with("{*");
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
            let mark = slots.len();
            slots.push(start..start + end);
            each.captures(slots);
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
        matches(rest, whole, after, slots, each);
        slots.truncate(mark);
    }
}
