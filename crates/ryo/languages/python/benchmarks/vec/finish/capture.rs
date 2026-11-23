//! THROWAWAY (redo-honest): the `capture` operation's meaning as the shipping engine has it (operators `capture::matches`):
//! EVERY way the text splits into the pattern's slots, one output row each; a slot holds at least one character; `{name}`
//! holds no separator character, `{*name}` may; the text between slots is literal. (The harness took one regex match, the first, so a rule
//! that needs each split saw one.)

/// Calls `each` with the byte ranges of the slots, once per way `text` matches `pattern`. `separator` (redo-binding pass 6,
/// merged by redo-int: the step's argument, it was `/`): the characters a `{name}` slot may not hold; none given: any.
pub(super) fn matches(pattern: &str, separator: Option<&str>, text: &str, each: &mut dyn FnMut(&[std::ops::Range<usize>])) {
    let mut slots = Vec::new();
    let sep = separator.unwrap_or("");
    walk(pattern, sep, text, text, &mut slots, each);
}

fn walk(pattern: &str, sep: &str, whole: &str, tail: &str, slots: &mut Vec<std::ops::Range<usize>>, each: &mut dyn FnMut(&[std::ops::Range<usize>])) {
    let blocked = |slot: &str| slot.chars().any(|c| sep.contains(c));
    let Some(open) = pattern.find('{') else {
        if pattern == tail {
            each(slots);
        }
        return;
    };
    let (literal, after_open) = pattern.split_at(open);
    let crosses = after_open.starts_with("{*");
    let Some(from) = tail.strip_prefix(literal) else { return };
    let rest = after_open.split_once('}').map(|(_, after)| after).unwrap_or_default();
    let start = whole.len() - from.len();
    if !rest.contains('{') {
        let Some(end) = from.len().checked_sub(rest.len()) else { return };
        if end >= 1
            && let (Some(slot), Some(after)) = (from.get(..end), from.get(end..))
            && after == rest
            && (crosses || !blocked(slot))
        {
            slots.push(start..start + end);
            each(slots);
            slots.pop();
        }
        return;
    }
    for end in 1..=from.len() {
        let (Some(slot), Some(after)) = (from.get(..end), from.get(end..)) else { continue };
        if !crosses && blocked(slot) {
            continue;
        }
        slots.push(start..start + end);
        walk(rest, sep, whole, after, slots, each);
        slots.pop();
    }
}

/// The slot names of `pattern`, in order.
pub(super) fn names(pattern: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = pattern;
    while let Some(o) = rest.find('{') {
        let after = &rest[o + 1..];
        let Some(c) = after.find('}') else { break };
        out.push(after[..c].trim_start_matches('*').to_owned());
        rest = &after[c + 1..];
    }
    out
}
