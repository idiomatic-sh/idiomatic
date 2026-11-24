//! THROWAWAY (redo-honest): the `capture` operation's meaning as the shipping engine has it (operators `capture::matches`):
//! EVERY way the text splits into the pattern's slots, one output row each; a slot holds at least one character; `{name}`
//! holds no separator character, `{*name}` may; the text between slots is literal. (The harness took one regex match, the first, so a rule
//! that needs each split saw one.)

/// Calls `each` with the byte ranges of the slots, once per way `text` matches `pattern`. `separator` (redo-binding pass 6,
/// merged by redo-int: the step's argument, it was `/`): the characters a `{name}` slot may not hold; none given: any.
pub(super) fn matches(
    pattern: &str,
    separator: Option<&str>,
    text: &str,
    each: &mut dyn FnMut(&[std::ops::Range<usize>]),
) {
    let mut slots = Vec::new();
    let sep = separator.unwrap_or("");
    walk(pattern, sep, text, text, &mut slots, each);
}

fn walk(
    pattern: &str,
    sep: &str,
    whole: &str,
    tail: &str,
    slots: &mut Vec<std::ops::Range<usize>>,
    each: &mut dyn FnMut(&[std::ops::Range<usize>]),
) {
    let blocked = |slot: &str| slot.chars().any(|c| sep.contains(c));
    let Some(open) = pattern.find('{') else {
        if pattern == tail {
            each(slots);
        }
        return;
    };
    let (literal, after_open) = pattern.split_at(open);
    let crosses = after_open.starts_with("{*");
    let Some(from) = tail.strip_prefix(literal) else {
        return;
    };
    let rest = after_open
        .split_once('}')
        .map(|(_, after)| after)
        .unwrap_or_default();
    let start = whole.len() - from.len();
    if !rest.contains('{') {
        let Some(end) = from.len().checked_sub(rest.len()) else {
            return;
        };
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
    // Each split must be followed by the next literal. Test its first
    // character so overlapping literals and every valid UTF-8 split survive.
    let limit = if crosses || sep.is_empty() {
        from.len()
    } else {
        from.find(|c: char| sep.contains(c)).unwrap_or(from.len())
    };
    let mut visit = |end: usize| {
        slots.push(start..start + end);
        walk(rest, sep, whole, &from[end..], slots, each);
        slots.pop();
    };
    if rest.starts_with('{') {
        for (end, _) in from.char_indices().skip(1).take_while(|(i, _)| *i <= limit) {
            visit(end);
        }
    } else if let Some(first) = rest.chars().next() {
        for (end, _) in from.match_indices(first).take_while(|(i, _)| *i <= limit) {
            if end > 0 {
                visit(end);
            }
        }
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

struct Ways {
    count: usize,
    cells: Vec<u64>,
}

fn captured(
    run: &mut super::Run,
    symbol: u32,
    pattern: &str,
    separator: Option<&str>,
    columns: &[Option<usize>],
) -> Ways {
    let mut result = Ways {
        count: 0,
        cells: Vec::new(),
    };
    let Ok(text) = String::from_utf8(run.text(symbol).to_vec()) else {
        return result;
    };
    matches(pattern, separator, &text, &mut |spans| {
        result.count += 1;
        for column in columns {
            let text = column
                .and_then(|i| spans.get(i))
                .map_or("", |r| &text[r.clone()]);
            result.cells.push(u64::from(run.sym(text.as_bytes())));
        }
    });
    result
}

pub(super) fn write(
    run: &mut super::Run,
    input: &super::Table,
    path: usize,
    pattern: &str,
    separator: Option<&str>,
    columns: &[Option<usize>],
    project: bool,
    out: &mut super::Table,
) -> super::R<()> {
    let mut by_path = super::FxHashMap::default();
    let mut row = Vec::new();
    let file = input.tys[path] == super::Ty::File;
    for (r, &word) in input.cols[path].iter().enumerate() {
        if word == super::NULL {
            continue;
        }
        let symbol = if file {
            *run.path_sym
                .get(word as usize)
                .ok_or("unbound capture file")?
        } else {
            let sym = u32::try_from(word).map_err(|_| "unbound capture text")?;
            if run.absent_word == Some(sym) {
                continue;
            }
            sym
        };
        // A closed relation can contain many edges from the same file. Only
        // the selected captures depend on its path. Preserve every input row.
        let single;
        let ways = if project && file {
            by_path
                .entry(symbol)
                .or_insert_with(|| captured(run, symbol, pattern, separator, columns))
        } else {
            single = captured(run, symbol, pattern, separator, columns);
            &single
        };
        for way in 0..ways.count {
            input.row(r, &mut row);
            let start = way * columns.len();
            row.extend_from_slice(&ways.cells[start..start + columns.len()]);
            out.push_row(&row);
        }
    }
    Ok(())
}
