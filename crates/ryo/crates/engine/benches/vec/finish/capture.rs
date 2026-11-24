//! THROWAWAY (redo-honest): the `capture` operation's meaning as the shipping engine has it (operators `capture::matches`):
//! EVERY way the text splits into the pattern's slots, one output row each; a slot holds at least one character; `{name}`
//! holds no separator character, `{*name}` may; the text between slots is literal. (The harness took one regex match, the first, so a rule
//! that needs each split saw one.)

/// Calls `each` with the byte ranges of the slots, once per way `text` matches `pattern`. `separator` (redo-binding pass 6,
/// merged by redo-int: the step's argument, it was `/`): the characters a `{name}` slot may not hold; none given: any.
fn matches(
    pattern: &str,
    separator: Option<&str>,
    text: &str,
    slots: &mut Vec<std::ops::Range<usize>>,
    each: &mut dyn FnMut(&[std::ops::Range<usize>]),
) {
    let sep = separator.unwrap_or("");
    walk(pattern, sep, text, text, slots, each);
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
pub(super) fn names(pattern: &str) -> smallvec::SmallVec<[&str; 8]> {
    let mut out = smallvec::SmallVec::new();
    let mut rest = pattern;
    while let Some(o) = rest.find('{') {
        let after = &rest[o + 1..];
        let Some(c) = after.find('}') else { break };
        out.push(after[..c].trim_start_matches('*'));
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
    text: &str,
    pattern: &str,
    separator: Option<&str>,
    columns: &[Option<usize>],
    slots: &mut Vec<std::ops::Range<usize>>,
) -> Ways {
    let mut result = Ways {
        count: 0,
        cells: Vec::new(),
    };
    matches(pattern, separator, text, slots, &mut |spans| {
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

/// A closed capture may omit a split that cannot survive any direct consumer.
/// Every participating select reads this result as its only input. An emit,
/// a different operator or an unknown predicate contributes no proof.
pub(super) fn required_equalities(
    exec: &mut super::Exec<'_>, run: &mut super::Run, node: usize,
) -> Vec<(usize, usize)> {
    use super::{E, Op};
    if exec.plan.emits.iter().any(|e| e.1 == node) {
        return Vec::new();
    }
    let mut common: Option<Vec<(usize, usize)>> = None;
    for index in 0..exec.consumers[node].len() {
        let user = exec.consumers[node][index];
        let step = &exec.plan.nodes[user];
        if step.op != "select" || step.inputs.as_slice() != [node] {
            return Vec::new();
        }
        let Ok(plan) = exec.sel(run, user) else { return Vec::new() };
        let terms: Vec<_> = plan.conj.iter().filter_map(|e| {
            if let E::Cmp(Op::Eq, a, b) = e
                && let (E::Col(0, x, tx), E::Col(0, y, ty)) = (&**a, &**b)
                && tx == ty {
                return Some(((*x).min(*y), (*x).max(*y)));
            }
            None
        }).collect();
        let kept = common.get_or_insert_with(|| terms.clone());
        kept.retain(|term| terms.contains(term));
        if kept.is_empty() { break; }
    }
    common.unwrap_or_default()
}

pub(super) fn write(
    run: &mut super::Run,
    input: &super::Table,
    path: usize,
    pattern: &str,
    separator: Option<&str>,
    columns: &[Option<usize>],
    project: bool,
    equal: &[(usize, usize)],
    out: &mut super::Table,
) -> super::R<()> {
    let mut by_path = super::FxHashMap::default();
    let mut row = Vec::new();
    // Walking a split always pops its ranges before returning. Reuse the
    // buffer across subjects while keeping only finished values in by_path.
    let mut slots = Vec::new();
    let file = input.tys[path] == super::Ty::File;
    let paths = run.paths.clone();
    // Rows with the same subject have the same possible splits. Apply each
    // split to its original row so multiplicity and row-specific tests survive.
    let mut next_row = 0;
    for subjects in input.cols[path].chunk_by(|a, b| a == b) {
        let first_row = next_row;
        next_row += subjects.len();
        let word = subjects[0];
        if word == super::NULL {
            continue;
        }
        let single;
        let ways = if file {
            let path = paths.get(word as usize).ok_or("unbound capture file")?;
            if project {
                by_path.entry(word).or_insert_with(|| captured(run, path, pattern, separator, columns, &mut slots))
            } else {
                single = captured(run, path, pattern, separator, columns, &mut slots);
                &single
            }
        } else {
            let symbol = u32::try_from(word).map_err(|_| "unbound capture text")?;
            if run.absent_word == Some(symbol) {
                continue;
            }
            let Ok(text) = String::from_utf8(run.text(symbol).to_vec()) else { continue };
            single = captured(run, &text, pattern, separator, columns, &mut slots);
            &single
        };
        if ways.count == 0 {
            continue;
        }
        for r in first_row..next_row {
            for way in 0..ways.count {
                let start = way * columns.len();
                let word_at = |i: usize| {
                    if i < input.cols.len() { input.cols[i][r] }
                    else { ways.cells[start + i - input.cols.len()] }
                };
                if equal.iter().any(|&(a, b)| {
                    let x = word_at(a);
                    x == super::NULL || x != word_at(b)
                }) { continue; }
                input.row(r, &mut row);
                row.extend_from_slice(&ways.cells[start..start + columns.len()]);
                out.push_row(&row);
            }
        }
    }
    Ok(())
}
