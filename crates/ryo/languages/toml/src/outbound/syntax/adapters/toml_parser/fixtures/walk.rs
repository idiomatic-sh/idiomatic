//! The library's own tree of a file, walked in the language's kinds: what the shape rail of the conformance holds our tree to.
//! `toml_parser` gives no tree, only a stream of events (`events`), every byte of the file in one of them, so its tree is read from
//! them in passes that share nothing with the adapter's one. First the constructs of TOML the events make, each with the bytes it
//! spans: a table from its header to the next one, a key-value from its key to the end of its value (or, with none, to the last piece
//! before what ends it), a key over its parts and the dots between them, an array or an inline table from its opening to its
//! closing. Then every event is a token, put in the deepest construct whose bytes hold it, an empty one, which has no bytes, in the
//! construct the parser gave it in. A token the adapter's pass puts in the wrong construct, or a construct it opens or closes in the
//! wrong place, is a difference.

use ryo::languages::Offset;
use ryo::languages::contract::{Leafness, Walked};

use super::events::{Recorded, Role, events};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;

/// A construct of TOML the events make: its kind, its bytes, and the construct it is in (none for the document) at what depth.
struct Construct {
    /// What construct it is.
    kind: Kind,
    /// Where its first event starts.
    start: Offset,
    /// Where the last event it holds ends.
    end: Offset,
    /// The construct it is in, by its place among those made; none for the document.
    parent: Option<usize>,
    /// How many constructs were open, itself included, when it was opened.
    depth: usize,
}

/// The constructs the events make, in the order they are opened, and those still open.
#[derive(Default)]
struct Constructs {
    /// Every construct made so far, in the order they were opened.
    made: Vec<Construct>,
    /// The constructs still open, by their place among those made, the innermost last.
    open: Vec<usize>,
}

impl Constructs {
    /// The kind of the innermost construct open: the document when none is.
    fn top(&self) -> Kind {
        self.open
            .last()
            .and_then(|at| self.made.get(*at))
            .map_or(Kind::Node(Node::Toml), |top| top.kind)
    }

    /// Opens a construct of `kind` at `at`, inside the innermost one open.
    fn open(&mut self, kind: Kind, at: Offset) {
        let parent = self.open.last().copied();
        self.open.push(self.made.len());
        self.made.push(Construct {
            kind,
            start: at,
            end: at,
            parent,
            depth: self.open.len(),
        });
    }

    /// Closes the innermost construct open.
    fn close(&mut self) {
        let _: Option<usize> = self.open.pop();
    }

    /// Closes the key, and the key-value it is in, of an entry that ends before its value.
    fn end_entry(&mut self) {
        for kind in [Kind::Node(Node::Key), Kind::Node(Node::Keyval)] {
            if self.top() == kind {
                self.close();
            }
        }
    }

    /// Takes the piece `event` into every construct open: a key holds only its parts and the dots between them.
    fn hold(&mut self, event: &Recorded) {
        let key_piece = matches!(event.role, Role::KeyPart | Role::KeyDot);
        for at in &self.open {
            if let Some(each) = self.made.get_mut(*at)
                && (each.kind != Kind::Node(Node::Key) || key_piece)
            {
                each.end = each.end.max(event.end);
            }
        }
    }
}

/// The constructs of `events`, in the order they are opened, and for each event the construct it comes in (none for the document).
fn constructs(events: &[Recorded]) -> (Vec<Construct>, Vec<Option<usize>>) {
    let mut made = Constructs::default();
    let mut homes = Vec::with_capacity(events.len());
    let mut header = false;
    for event in events {
        match event.role {
            Role::HeaderOpen(kind) => {
                made.open.clear();
                made.open(kind, event.start);
                header = true;
            }
            Role::HeaderClose | Role::Equals => {
                if made.top() == Kind::Node(Node::Key) {
                    made.close();
                }
                header &= event.role != Role::HeaderClose;
            }
            Role::KeyPart if made.top() != Kind::Node(Node::Key) => {
                if !header {
                    made.open(Kind::Node(Node::Keyval), event.start);
                }
                made.open(Kind::Node(Node::Key), event.start);
            }
            Role::Ends | Role::Close(_) => made.end_entry(),
            Role::Open(kind) => {
                made.hold(event);
                made.open(kind, event.start);
            }
            Role::KeyPart | Role::KeyDot | Role::Value | Role::Plain => {}
        }
        if !matches!(event.role, Role::Open(_)) {
            made.hold(event);
        }
        homes.push(made.open.last().copied());
        if let Role::Close(kind) = event.role
            && made.top() == kind
        {
            made.close();
        }
        // A value ends the key-value it is the value of.
        if matches!(event.role, Role::Close(_) | Role::Value)
            && made.top() == Kind::Node(Node::Keyval)
        {
            made.close();
        }
    }
    (made.made, homes)
}

/// A token of kind `kind` over `start..end`.
fn token(kind: Kind, start: Offset, end: Offset) -> Walked {
    Walked {
        kind: kind_bits(kind),
        leaf: Leafness::Token,
        start,
        end,
        children: Vec::new(),
    }
}

/// The tokens of `events`, each with the event it is from (none for bytes no event spans), in the order of the file: an empty token,
/// which the parser gives where it notices something missing, waits for the next token and goes after it when that one starts before
/// it, and is where the tokens before it end; the bytes no event spans between them are text the parser could not read.
fn tokens(events: &[Recorded], length: Offset) -> Vec<(Walked, Option<usize>)> {
    let mut pieces = Vec::new();
    let mut end = Offset(0);
    let put = |at: usize, into: &mut Vec<(Walked, Option<usize>)>, reached: &mut Offset| {
        if let Some(event) = events.get(at) {
            let start = event.start.max(*reached);
            if start > *reached {
                into.push((token(Kind::Extra(Extra::Error), *reached, start), None));
            }
            *reached = event.end.max(start);
            into.push((token(event.kind, start, *reached), Some(at)));
        }
    };
    let mut waiting: Option<usize> = None;
    for (at, event) in events.iter().enumerate() {
        if event.start == event.end {
            if let Some(before) = waiting.replace(at) {
                put(before, &mut pieces, &mut end);
            }
            continue;
        }
        if let Some(before) = waiting.take_if(|before| {
            events
                .get(*before)
                .is_some_and(|empty| empty.start <= event.start)
        }) {
            put(before, &mut pieces, &mut end);
        }
        put(at, &mut pieces, &mut end);
    }
    if let Some(before) = waiting {
        put(before, &mut pieces, &mut end);
    }
    if end < length {
        pieces.push((token(Kind::Extra(Extra::Error), end, length), None));
    }
    pieces
}

/// Puts `piece` among `pieces` in the order of the file: before the first that starts after it, or that starts where it does and
/// spans bytes.
fn insert(pieces: &mut Vec<Walked>, piece: Walked) {
    let at = pieces
        .iter()
        .position(|each| {
            each.start > piece.start || (each.start == piece.start && each.end > each.start)
        })
        .unwrap_or(pieces.len());
    pieces.insert(at, piece);
}

/// The library's tree of `text`.
#[must_use]
pub(crate) fn walked(text: &str) -> Walked {
    let events = events(text);
    let length = Offset(u32::try_from(text.len()).unwrap_or(u32::MAX));
    let (made, homes) = constructs(&events);
    // The pieces under each construct, the document's first.
    let mut under: Vec<Vec<Walked>> = std::iter::repeat_with(Vec::new)
        .take(made.len() + 1)
        .collect();
    for (piece, from) in tokens(&events, length) {
        let home = from.and_then(|at| homes.get(at).copied().flatten());
        // An empty token has no bytes to be placed by: it is in the construct the parser gave it in. Any other is in the deepest
        // construct whose bytes hold it.
        let target = match home {
            Some(home) if piece.start == piece.end => Some(home),
            Some(_) | None => made
                .iter()
                .enumerate()
                .filter(|(_, each)| each.start <= piece.start && piece.end <= each.end)
                .max_by_key(|(_, each)| each.depth)
                .map(|(at, _)| at),
        };
        if let Some(pieces) = under.get_mut(target.map_or(0, |at| at + 1)) {
            insert(pieces, piece);
        }
    }
    for (at, construct) in made.iter().enumerate().rev() {
        let children = under
            .get_mut(at + 1)
            .map(std::mem::take)
            .unwrap_or_default();
        let node = Walked {
            kind: kind_bits(construct.kind),
            leaf: Leafness::Node,
            start: construct.start,
            end: construct.end,
            children,
        };
        if let Some(pieces) = under.get_mut(construct.parent.map_or(0, |parent| parent + 1)) {
            insert(pieces, node);
        }
    }
    Walked {
        kind: kind_bits(Kind::Node(Node::Toml)),
        leaf: Leafness::Node,
        start: Offset(0),
        end: length,
        children: under.into_iter().next().unwrap_or_default(),
    }
}

#[cfg(test)]
#[path = "walk_test.rs"]
mod tests;
