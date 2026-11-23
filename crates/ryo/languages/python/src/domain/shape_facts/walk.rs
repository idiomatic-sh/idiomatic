//! Expression classes from parser events. A class handle stays fixed as later
//! siblings arrive. Its externally visible representative is its greatest node
//! id, as in the reverse traversal used by the row reader.
use super::service::{payload, valued};
use super::types::{ShapeBuild, ShapeReading};
use crate::domain::extras::types::Extra;
use crate::domain::labels::{service::field_bits, types::Field};
use crate::domain::literal_facts::types::TextBuffer;
use crate::domain::model::names::types::NameCode;
use crate::domain::node::service::word;
use crate::domain::nodes::types::Category;
use crate::domain::syntax::{service::{category_of, kind_bits}, types::Kind};
use ruff_python_ast::AnyNodeRef;
use ryo::languages::{FieldId, FileWords, KindBits, NameText, Position, RetainedKeyIndex, ShapeWord};
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub(crate) struct Classes {
    expressions: Vec<Position>,
    handles: Vec<Option<Position>>,
    last: Vec<Position>,
}

impl Classes {
    pub(crate) fn rows(&self) -> impl Iterator<Item = (Position, Position)> + '_ {
        self.expressions.iter().filter_map(|&at| {
            let handle = self.handles[usize::from(at)]?;
            Some((at, self.last[usize::from(handle)]))
        })
    }
}

#[derive(Debug)]
struct Open {
    active: bool,
    valid: bool,
    kind: Kind,
    field: FieldId,
    start: usize,
}

#[derive(Default, Debug)]
pub(crate) struct WalkShapes {
    stack: Vec<Open>,
    children: Vec<ShapeWord>,
    // Name and Identifier are disjoint structural kinds. Their one payload
    // already has a canonical code assigned by the lexical walk.
    names: Vec<[Option<Position>; 2]>,
    other: BTreeMap<Box<[u64]>, Position>,
    key: Vec<u64>,
    words: FileWords,
    index: RetainedKeyIndex<Position, ShapeWord>,
    text: TextBuffer,
    expressions: Vec<Position>,
    last: Vec<Position>,
}

impl WalkShapes {
    pub(crate) fn enter(&mut self, at: Position, kind: Kind, field: Field) {
        let expression = category_of(kind) == Category::Expression;
        if expression { self.expressions.push(at); }
        self.index.classes.push(None);
        self.stack.push(Open {
            active: expression || self.stack.last().is_some_and(|p| p.active),
            valid: kind != Kind::Extra(Extra::IpyEscapeCommand),
            kind, field: field_bits(field), start: self.children.len(),
        });
    }

    pub(crate) fn token(&mut self, bits: KindBits, text: &str) {
        if self.stack.last().is_some_and(|p| p.active) {
            self.children.push(ShapeWord::Token(bits, word(&mut self.words, NameText(text))));
        }
    }

    pub(crate) fn leave(&mut self, at: Position, node: AnyNodeRef<'_>, code: NameCode) {
        let open = self.stack.pop().expect("matching syntax events");
        if !open.active { return; }
        if !open.valid {
            self.children.truncate(open.start);
            if let Some(parent) = self.stack.last_mut() { parent.valid = false; }
            return;
        }
        let name = match node {
            AnyNodeRef::ExprName(_) => Some(0),
            AnyNodeRef::Identifier(_) => Some(1),
            _ => None,
        };
        let handle = if let Some(which) = name {
            let c = usize::from(code);
            if c >= self.names.len() { self.names.resize(c + 1, [None; 2]); }
            *self.names[c][which].get_or_insert_with(|| {
                let id = Position(self.last.len() as u32);
                self.last.push(at);
                id
            })
        } else {
            self.index.scratch.clear();
            let mut build = ShapeBuild { index: &mut self.index, words: &mut self.words, text: &mut self.text };
            build.index.scratch.push(ShapeWord::Kind(kind_bits(open.kind)));
            payload(&mut build, node);
            if valued(&mut build, node) == ShapeReading::ByChildren {
                let children = &self.children[open.start..];
                if open.kind == Kind::Extra(Extra::CallArguments) {
                    for f in [Field::Args, Field::Keywords] {
                        build.index.scratch.extend(children.iter().filter(|w| matches!(w, ShapeWord::Child(c, _) if *c == field_bits(f))));
                    }
                } else {
                    build.index.scratch.extend_from_slice(children);
                }
            }
            self.key.clear();
            self.key.extend(build.index.scratch.iter().copied().map(encoded));
            if let Some(&id) = self.other.get(self.key.as_slice()) {
                id
            } else {
                let id = Position(self.last.len() as u32);
                self.other.insert(self.key.clone().into_boxed_slice(), id);
                self.last.push(at);
                id
            }
        };
        self.children.truncate(open.start);
        self.index.classes[usize::from(at)] = Some(handle);
        self.last[usize::from(handle)] = self.last[usize::from(handle)].max(at);
        if self.stack.last().is_some_and(|p| p.active) {
            self.children.push(ShapeWord::Child(open.field, handle));
        }
    }

    pub(crate) fn finish(self) -> Classes {
        Classes { expressions: self.expressions, handles: self.index.classes, last: self.last }
    }
}

/// Lossless ordering key: the discriminant, 16-bit field or kind and 32-bit value.
fn encoded(word: ShapeWord) -> u64 {
    let (tag, kind, value) = match word {
        ShapeWord::Kind(k) => (0u64, k.0, 0),
        ShapeWord::Child(f, c) => (1, f.0, c.0),
        ShapeWord::Token(k, s) => (2, k.0, s.0),
        ShapeWord::Identifier(s) => (3, 0, s.0),
        ShapeWord::Literal(s) => (4, 0, s.0),
        ShapeWord::Property(p, s) => (5, p.0, s.0),
    };
    (tag << 48) | (u64::from(kind) << 32) | u64::from(value)
}
