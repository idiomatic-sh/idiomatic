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
use crate::domain::nodes::types::{Category, Node};
use crate::domain::syntax::{service::{category_of, kind_bits}, types::Kind};
use ruff_python_ast::AnyNodeRef;
use ryo::languages::{FieldId, FileWords, KindBits, NameText, Position, RetainedKeyIndex, ShapeWord};
use std::collections::BTreeMap;

/// Ordinary string, byte string and number carriers encountered in the file's own tree.
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct Carriers {
    pub(crate) string: bool,
    pub(crate) bytes: bool,
    pub(crate) number: bool,
    pub(crate) percent: bool,
    pub(crate) brace: bool,
}

#[derive(Debug, Default)]
pub(crate) struct Classes {
    pub(crate) carriers: Carriers,
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

/// Most composite keys are the only key ending in a particular child class.
/// Keep those keys consecutively in one arena. A collision compares the full
/// sequence and promotes that bucket to an ordered dictionary when needed.
#[derive(Default, Debug)]
enum Bucket {
    #[default]
    Empty,
    Single { start: usize, end: usize, class: Position },
    Shared(Box<BTreeMap<Box<[u64]>, Position>>),
}

#[derive(Default, Debug)]
struct Dictionary {
    child: Vec<Bucket>,
    kind: Vec<Bucket>,
    words: Vec<u64>,
}

impl Dictionary {
    fn class(&mut self, kind: KindBits, child: Option<Position>, key: &[u64], next: Position) -> Position {
        let (buckets, index) = match child {
            Some(child) => (&mut self.child, usize::from(child)),
            None => (&mut self.kind, usize::from(kind.0)),
        };
        if index >= buckets.len() { buckets.resize_with(index + 1, Default::default); }
        let bucket = &mut buckets[index];
        match bucket {
            Bucket::Empty => {
                let start = self.words.len();
                self.words.extend_from_slice(key);
                *bucket = Bucket::Single { start, end: self.words.len(), class: next };
                next
            }
            Bucket::Single { start, end, class } => {
                let first = &self.words[*start..*end];
                if first == key { return *class; }
                let mut map = BTreeMap::new();
                map.insert(first.into(), *class);
                map.insert(key.into(), next);
                *bucket = Bucket::Shared(Box::new(map));
                next
            }
            Bucket::Shared(map) => {
                if let Some(&id) = map.get(key) { return id; }
                map.insert(key.into(), next);
                next
            }
        }
    }
}

#[derive(Default, Debug)]
pub(crate) struct WalkShapes {
    stack: Vec<Open>,
    children: Vec<ShapeWord>,
    // Name and Identifier are disjoint structural kinds. Their one payload
    // already has a canonical code assigned by the lexical walk.
    names: Vec<[Option<Position>; 2]>,
    other: Dictionary,
    key: Vec<u64>,
    words: FileWords,
    index: RetainedKeyIndex<Position, ShapeWord>,
    text: TextBuffer,
    pub(crate) carriers: Carriers,
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
        if self.stack.last().is_some_and(|p| p.active
            && !matches!(p.kind, Kind::Node(Node::Name) | Kind::Extra(Extra::Identifier))) {
            self.children.push(ShapeWord::Token(bits, word(&mut self.words, NameText(text))));
        }
    }

    pub(crate) fn leave(&mut self, at: Position, node: AnyNodeRef<'_>, code: NameCode) {
        match node {
            AnyNodeRef::ExprStringLiteral(literal) => {
                self.carriers.string = true;
                for part in &literal.value {
                    if self.carriers.percent && self.carriers.brace { break; }
                    let bytes = part.value.as_bytes();
                    self.carriers.percent |= !self.carriers.percent && bytes.contains(&b'%');
                    self.carriers.brace |= !self.carriers.brace && bytes.contains(&b'{');
                }
            }
            AnyNodeRef::ExprBytesLiteral(_) => self.carriers.bytes = true,
            AnyNodeRef::ExprNumberLiteral(_) => self.carriers.number = true,
            _ => {}
        }
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
            let mut child = None;
            for &word in &build.index.scratch {
                if let ShapeWord::Child(_, id) = word { child = Some(id); }
                self.key.push(encoded(word));
            }
            let next = Position(self.last.len() as u32);
            let class = self.other.class(kind_bits(open.kind), child, &self.key, next);
            if class == next { self.last.push(at); }
            class
        };
        self.children.truncate(open.start);
        self.index.classes[usize::from(at)] = Some(handle);
        self.last[usize::from(handle)] = self.last[usize::from(handle)].max(at);
        if self.stack.last().is_some_and(|p| p.active) {
            self.children.push(ShapeWord::Child(open.field, handle));
        }
    }

    pub(crate) fn finish(self) -> Classes {
        Classes { carriers: self.carriers, expressions: self.expressions, handles: self.index.classes, last: self.last }
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
