//! Test support split from adapter_test: Shape, leaf, bare and more.

use crate::domain::extras::types::Extra;
use crate::domain::syntax::service::{kind_name, kind_of, kinds};
use crate::domain::syntax::types::Kind;
use crate::outbound::syntax::interface::SyntaxPort as _;
use ryo::languages::{self, Elements, Position};

/// One piece of a tree held to the standard's: its rule's name, the bytes it spans and the pieces under it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) struct Shape {
    name: String,
    start: u32,
    end: u32,
    under: Vec<Shape>,
}

pub(in super::super) fn leaf(name: &str, start: u32, end: u32) -> Shape {
    Shape {
        name: name.to_owned(),
        start,
        end,
        under: Vec::new(),
    }
}

/// A piece with the white space left out of it and its bytes those from its first to its last piece that is not white space: the
/// standard's separators own the white space around them (and its `end-object` the white space after a document's last value),
/// where our trees leave it to the piece around, so the two are compared by what they hold. Where the white space stands is held
/// by the rail that reads the tokens back byte for byte.
pub(in super::super) fn bare(mut piece: Shape) -> Shape {
    piece.under = piece
        .under
        .into_iter()
        .filter(|each| each.name != "ws")
        .map(bare)
        .collect();
    if let (Some(first), Some(last)) = (piece.under.first(), piece.under.last()) {
        (piece.start, piece.end) = (first.start, last.end);
    }
    piece
}

/// What the dump printed, read back: `<rule> <start>-<end>` a line, a rule's parts indented one more than it. Nothing when the text
/// is no dump.
pub(in super::super) fn dumped(dump: &str) -> Option<Shape> {
    let mut open: Vec<(usize, Shape)> = Vec::new();
    let mut root = None;
    let close = |stack: &mut Vec<(usize, Shape)>, built: &mut Option<Shape>| {
        let (_, done) = stack.pop()?;
        match stack.last_mut() {
            Some((_, parent)) => parent.under.push(done),
            None => *built = Some(done),
        }
        Some(())
    };
    for line in dump.lines() {
        let text = line.trim_start();
        let depth = line.len() - text.len();
        let (name, span) = text.split_once(' ')?;
        let (start, end) = span.split_once('-')?;
        while open.last().is_some_and(|(at, _)| *at >= depth) {
            close(&mut open, &mut root)?;
        }
        open.push((depth, leaf(name, start.parse().ok()?, end.parse().ok()?)));
    }
    while !open.is_empty() {
        close(&mut open, &mut root)?;
    }
    root
}

/// The standard's rule `raw` as the kinds of the language: a rule the language has a node for is that node; one it has a token for
/// is that token, in the bytes its parts leave (`name-separator` is the `:`, the white space around it parts of it the language has
/// its own tokens for); any other rule (`value`) is only the way to its parts.
pub(in super::super) fn onto(raw: &Shape, out: &mut Vec<Shape>) {
    match kinds().find(|each| kind_name(*each).0 == raw.name) {
        Some(Kind::Node(_)) => {
            let mut under = Vec::new();
            raw.under.iter().for_each(|each| onto(each, &mut under));
            out.push(Shape {
                under,
                ..leaf(&raw.name, raw.start, raw.end)
            });
        }
        Some(Kind::Token(_) | Kind::Extra(_)) => {
            let mut cursor = raw.start;
            for each in &raw.under {
                if each.start > cursor {
                    out.push(leaf(&raw.name, cursor, each.start));
                }
                onto(each, out);
                cursor = each.end;
            }
            if cursor < raw.end {
                out.push(leaf(&raw.name, cursor, raw.end));
            }
        }
        None => raw.under.iter().for_each(|each| onto(each, out)),
    }
}

/// Our tree at `at` as the standard has it.
pub(in super::super) fn ours(elements: &Elements, at: Position, out: &mut Vec<Shape>) {
    let kind = kind_of(languages::kind_at(elements, at));
    let mut under = Vec::new();
    for each in languages::children(elements, at) {
        ours(elements, each, &mut under);
    }
    out.push(Shape {
        under,
        ..leaf(
            kind_name(kind).0,
            languages::start_at(elements, at).0,
            languages::end_at(elements, at).0,
        )
    });
}

/// What differs between our tree and the standard's, the first place found; nothing when they agree.
pub(in super::super) fn differences(ours: &Shape, theirs: &Shape, path: &str) -> Option<String> {
    let here = format!("{path}/{}", ours.name);
    if (&ours.name, ours.start, ours.end) != (&theirs.name, theirs.start, theirs.end) {
        return Some(format!(
            "{here}: we have `{} {}-{}`, the standard has `{} {}-{}`",
            ours.name, ours.start, ours.end, theirs.name, theirs.start, theirs.end
        ));
    }
    ours.under
        .iter()
        .zip(&theirs.under)
        .find_map(|(mine, yours)| differences(mine, yours, &here))
        .or_else(|| {
            (ours.under.len() != theirs.under.len()).then(|| {
                format!(
                    "{here}: we have {} pieces, the standard has {}",
                    ours.under.len(),
                    theirs.under.len()
                )
            })
        })
}
