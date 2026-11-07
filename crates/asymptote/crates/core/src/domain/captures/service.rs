//! The `captures` service: free functions named by verb over the `{name}`s a tool's inputs and command lines write,
//! with no IO and no ports. A `{name}` matches one or more characters within one path segment, as a Tastemaker
//! selector's does, and a name written twice takes one value; braces around anything that is not a name are text.

use super::constants::{CLOSE, EXPANSION, OPEN, SEPARATOR};
use super::types::{
    CaptureName, CaptureValue, Captures, CommandKey, Literal, Piece, SegmentPattern, Substituted,
    Uncaptured, Written,
};
use crate::domain::manifest::types::{Argument, Argv, Fix, Pathspec, Tool};
use crate::domain::violations::types::ToolId;

/// The capture `inner`, the text between a `{` and its `}`, names, when it is a name: a letter or `_`, then letters,
/// digits, `_` and `-`. A regular expression's `{7}` or a shell's `{a,b}` names none, so it stays text.
fn name_of(inner: Written<'_>) -> Option<CaptureName> {
    let mut characters = inner.0.chars();
    let starts = characters
        .next()
        .filter(|first| first.is_ascii_alphabetic() || *first == '_');
    starts
        .filter(|_| {
            characters.all(|rest| rest.is_ascii_alphanumeric() || rest == '_' || rest == '-')
        })
        .map(|_| CaptureName(inner.0.into()))
}

/// `text` in its pieces: each `{name}` a capture, and the text between them as it is written.
#[must_use]
pub fn pieces(text: Written<'_>) -> SegmentPattern {
    let mut found = Vec::new();
    let mut literal = String::new();
    let mut rest = text.0;
    while let Some(at) = rest.find(OPEN) {
        let (before, from) = rest.split_at(at);
        literal.push_str(before);
        let inner = from.get(OPEN.len_utf8()..).unwrap_or_default();
        // A `{` a `$` opens is a shell's expansion (`${HOME}`), never a capture: it stays text.
        let expanded = literal.ends_with(EXPANSION);
        let named = inner
            .split_once(CLOSE)
            .filter(|_| !expanded)
            .and_then(|(written, after)| name_of(Written(written)).map(|name| (name, after)));
        if let Some((name, after)) = named {
            if !literal.is_empty() {
                found.push(Piece::Text(Literal(std::mem::take(&mut literal).into())));
            }
            found.push(Piece::Capture(name));
            rest = after;
        } else {
            literal.push(OPEN);
            rest = inner;
        }
    }
    literal.push_str(rest);
    if !literal.is_empty() {
        found.push(Piece::Text(Literal(literal.into())));
    }
    SegmentPattern(found)
}

/// The captures `text` writes, each once, in the order it first writes them.
#[must_use]
pub fn names(text: Written<'_>) -> Vec<CaptureName> {
    let mut found: Vec<CaptureName> = Vec::new();
    for piece in pieces(text).0 {
        if let Piece::Capture(name) = piece
            && !found.contains(&name)
        {
            found.push(name);
        }
    }
    found
}

/// The captures `inputs` write, each once, in the order they first write them: the order an expanded tool's id
/// writes their values in.
#[must_use]
pub fn captured(inputs: &[Pathspec]) -> Vec<CaptureName> {
    let mut found: Vec<CaptureName> = Vec::new();
    for input in inputs {
        for name in names(Written(&input.0)) {
            if !found.contains(&name) {
                found.push(name);
            }
        }
    }
    found
}

/// The captures `text` writes that `bound` gives no value.
#[must_use]
pub fn unbound(text: Written<'_>, bound: &Captures) -> Vec<CaptureName> {
    names(text)
        .into_iter()
        .filter(|name| !bound.0.contains_key(name))
        .collect()
}

/// `text` with each capture `captures` gives a value replaced by it; one it gives none stays as it is written.
#[must_use]
pub fn substitute(text: Written<'_>, captures: &Captures) -> Substituted {
    let mut written = String::with_capacity(text.0.len());
    for piece in pieces(text).0 {
        match piece {
            Piece::Text(literal) => written.push_str(&literal.0),
            Piece::Capture(name) => {
                if let Some(value) = captures.0.get(&name) {
                    written.push_str(&value.0);
                } else {
                    written.push(OPEN);
                    written.push_str(&name.0);
                    written.push(CLOSE);
                }
            }
        }
    }
    Substituted(written)
}

/// What the pieces `tail` that follow a matched piece make of `remaining`, the rest of the entry name: `captures`
/// when both are done, and none when only one is.
fn rest_of(tail: &[Piece], remaining: Written<'_>, captures: &Captures) -> Option<Captures> {
    if tail.is_empty() {
        remaining.0.is_empty().then(|| captures.clone())
    } else {
        bind(&SegmentPattern(tail.to_vec()), remaining, captures)
    }
}

/// The captures that make the segment `pattern` match the entry name `entry`, beside `bound`: a capture `bound`
/// gives a value matches that value, and each other takes one or more characters, the longest that lets the rest
/// match. None when the entry does not match, or is no segment.
#[must_use]
pub fn bind(pattern: &SegmentPattern, entry: Written<'_>, bound: &Captures) -> Option<Captures> {
    if entry.0.is_empty() || entry.0.contains(SEPARATOR) {
        return None;
    }
    let (first, tail) = pattern.0.split_first()?;
    match first {
        Piece::Text(literal) => rest_of(tail, Written(entry.0.strip_prefix(&*literal.0)?), bound),
        Piece::Capture(name) => {
            if let Some(value) = bound.0.get(name) {
                return rest_of(tail, Written(entry.0.strip_prefix(&*value.0)?), bound);
            }
            entry
                .0
                .char_indices()
                .map(|(at, character)| at + character.len_utf8())
                .rev()
                .find_map(|end| {
                    let (taken, remaining) = entry.0.split_at(end);
                    let mut captures = bound.clone();
                    drop(captures.0.insert(name.clone(), CaptureValue(taken.into())));
                    rest_of(tail, Written(remaining), &captures)
                })
        }
    }
}

/// The first capture `tool`'s `run`, then its `fix`, writes that none of its inputs does.
#[must_use]
pub fn uncaptured(tool: &Tool) -> Option<Uncaptured> {
    let inputs = captured(&tool.inputs);
    let commands = [
        (CommandKey::Run, Some(&tool.run)),
        (
            CommandKey::Fix,
            match &tool.fix {
                Fix::Command(argv) => Some(argv),
                Fix::Absent => None,
            },
        ),
    ];
    commands.into_iter().find_map(|(key, argv)| {
        argv?
            .0
            .iter()
            .flat_map(|argument| names(Written(&argument.0)))
            .find(|name| !inputs.contains(name))
            .map(|name| Uncaptured { key, name })
    })
}

/// A command line with `captures` written in.
fn written_in(argv: &Argv, captures: &Captures) -> Argv {
    Argv(
        argv.0
            .iter()
            .map(|argument| Argument(substitute(Written(&argument.0), captures).0.into()))
            .collect(),
    )
}

/// The tool `tool` makes for one match of its inputs: its id followed by each value in `order`, each after a `/`,
/// and its command lines and inputs with the values written in.
#[must_use]
pub fn instance(tool: &Tool, order: &[CaptureName], captures: &Captures) -> Tool {
    let mut id = String::from(&*tool.id.0);
    for value in order.iter().filter_map(|name| captures.0.get(name)) {
        id.push(SEPARATOR);
        id.push_str(&value.0);
    }
    Tool {
        id: ToolId(id.into()),
        run: written_in(&tool.run, captures),
        fix: match &tool.fix {
            Fix::Command(argv) => Fix::Command(written_in(argv, captures)),
            Fix::Absent => Fix::Absent,
        },
        output: tool.output,
        group: tool.group.clone(),
        timeout: tool.timeout,
        inputs: tool
            .inputs
            .iter()
            .map(|input| Pathspec(substitute(Written(&input.0), captures).0.into()))
            .collect(),
    }
}
