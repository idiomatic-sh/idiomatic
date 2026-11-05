//! The `documents` service: it works out, in one pass over a file's syntax tree, the mappings, entries and sequence elements its TOML
//! declares, and answers each reader from that pass. A key is an entry where it first opens its mapping, or where it is the last
//! part of a key-value; a key a later header or dotted key says again is the same entry, and writes none. Every logical table has
//! one identity, whether a header, a dotted key or a longer header opens it, and a path through an array of tables resolves within
//! its most recent member, as TOML says. An ordinal counts among the entries of the owner the entry resolves to.
//! The pass is made once for a file, as its `Parse::FileModel`, which the engine gives every reader of the file.

use super::types::Keyness;
use crate::domain::extras::types::Extra;
use crate::domain::keys::service::{put, slot};
use crate::domain::keys::types::Child;
use crate::domain::model::types::{ElementFact, EntryFact, MappingFact, Model, Readability};
use crate::domain::nodes::types::TomlNode;
use crate::domain::syntax::service::{kind, value_kind};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use ryo::languages::{self, Count, MappingKind, Position, Tree, ValueKind};

/// Whether an element of `kind` can be a part of a key.
fn keyness(kind: Kind) -> Keyness {
    if matches!(
        kind,
        Kind::Token(Token::UnquotedKey | Token::BasicString | Token::LiteralString)
    ) {
        Keyness::Part
    } else {
        Keyness::Other
    }
}

/// The parts of the key of the header or key-value `holder`, in order.
fn parts<'a>(tree: &'a Tree<'_>, holder: Position) -> impl Iterator<Item = Position> + 'a {
    let key = languages::children(tree.elements, holder)
        .find(|each| kind(tree, *each) == Kind::Node(TomlNode::Key));
    key.into_iter()
        .flat_map(|key| languages::children(tree.elements, key))
        .filter(|each| keyness(kind(tree, *each)) == Keyness::Part)
}

/// The value of the key-value `pair`: the element after its equals sign that is a value, or what the parser could not read there.
#[must_use]
pub(crate) fn value_of(tree: &Tree<'_>, pair: Position) -> Option<Position> {
    languages::children(tree.elements, pair)
        .skip_while(|each| kind(tree, *each) != Kind::Token(Token::KeyvalSep))
        .find(|each| {
            let found = kind(tree, *each);
            value_kind(found).is_some() || found == Kind::Extra(Extra::Error)
        })
}

/// The next ordinal among the entries of `owner`.
fn next(model: &mut Model, owner: Position) -> languages::Integer {
    let count = model.counts.entry(owner).or_insert(languages::Integer(0));
    let ordinal = *count;
    count.0 += 1;
    ordinal
}

/// Records an entry.
fn entry(
    model: &mut Model,
    occurrence: Position,
    kind: ValueKind,
    owner: Position,
    value: Option<Position>,
    readability: Readability,
) {
    let ordinal = next(model, owner);
    model.entries.push(EntryFact {
        occurrence,
        kind,
        owner,
        ordinal,
        value,
        readability,
    });
}

/// Records a mapping.
fn mapping(model: &mut Model, id: Position, kind: MappingKind, owner: Position, ordinal: Count) {
    model.mappings.push(MappingFact {
        id,
        kind,
        owner,
        ordinal: languages::Integer(i64::try_from(ordinal.0).unwrap_or(i64::MAX)),
    });
}

/// Records an element of a sequence.
fn element(
    model: &mut Model,
    occurrence: Position,
    kind: ValueKind,
    owner: Position,
    ordinal: Count,
) {
    model.elements.push(ElementFact {
        occurrence,
        kind,
        owner,
        ordinal: languages::Integer(i64::try_from(ordinal.0).unwrap_or(i64::MAX)),
    });
}

/// The mapping a part of a key that is not the last opens within `owner`, opening it as `opened` when nothing has; nothing when the
/// key names a value, which nothing extends (the library reports it). A path through an array of tables goes on in its last member.
fn open(
    tree: &Tree<'_>,
    model: &mut Model,
    owner: Position,
    part: Position,
    opened: MappingKind,
) -> Option<Position> {
    let slot = slot(tree, &model.children, model.hash, owner, part);
    match slot.child {
        Some(Child::Table(id)) => Some(id),
        Some(Child::Sequence { last: element, .. }) => Some(element),
        Some(Child::Value) => None,
        None => {
            entry(
                model,
                part,
                ValueKind::Mapping,
                owner,
                None,
                Readability::Readable,
            );
            mapping(model, part, opened, part, Count(0));
            put(&mut model.children, slot, part, Child::Table(part));
            Some(part)
        }
    }
}

/// Reads the key-value `pair` within the mapping `owner`.
fn pair(tree: &Tree<'_>, model: &mut Model, owner: Position, pair: Position) {
    let mut within = owner;
    let mut last = None;
    for part in parts(tree, pair) {
        if let Some(before) = last {
            let Some(opened) = open(tree, model, within, before, MappingKind::Dotted) else {
                return;
            };
            within = opened;
        }
        last = Some(part);
    }
    let Some(part) = last else {
        return;
    };
    let slot = slot(tree, &model.children, model.hash, within, part);
    if slot.child.is_some() {
        return;
    }
    put(&mut model.children, slot, part, Child::Value);
    let value = value_of(tree, pair);
    let Some(found) = value.and_then(|each| value_kind(kind(tree, each))) else {
        entry(
            model,
            part,
            ValueKind::String,
            within,
            value,
            Readability::Unreadable,
        );
        return;
    };
    entry(model, part, found, within, value, Readability::Readable);
    match (found, value) {
        (ValueKind::InlineMapping, Some(table)) => {
            mapping(model, table, MappingKind::Inline, part, Count(0));
            inline(tree, model, table);
        }
        (ValueKind::Sequence, Some(array)) => sequence(tree, model, array, part),
        _ => {}
    }
}

/// Reads the entries of the inline table `table`.
fn inline(tree: &Tree<'_>, model: &mut Model, table: Position) {
    for each in languages::children(tree.elements, table) {
        if kind(tree, each) == Kind::Node(TomlNode::Keyval) {
            pair(tree, model, table, each);
        }
    }
}

/// Reads the values of the array `array`, whose sequence the element `owner` holds.
fn sequence(tree: &Tree<'_>, model: &mut Model, array: Position, owner: Position) {
    let mut place = Count(0);
    for each in languages::children(tree.elements, array) {
        let Some(found) = value_kind(kind(tree, each)) else {
            continue;
        };
        element(model, each, found, owner, place);
        match found {
            ValueKind::InlineMapping => {
                mapping(model, each, MappingKind::Inline, each, place);
                inline(tree, model, each);
            }
            ValueKind::Sequence => sequence(tree, model, each, each),
            ValueKind::String
            | ValueKind::Integer
            | ValueKind::Float
            | ValueKind::Boolean
            | ValueKind::Datetime
            | ValueKind::Mapping
            | ValueKind::SequenceOfMappings
            | ValueKind::Null => {}
        }
        place.0 += 1;
    }
}

/// Reads the header of the table `table`, and answers the mapping its entries go in.
fn table(tree: &Tree<'_>, model: &mut Model, table: Position) -> Option<Position> {
    let mut within = Position(0);
    let mut last = None;
    for part in parts(tree, table) {
        if let Some(before) = last {
            within = open(tree, model, within, before, MappingKind::Implicit)?;
        }
        last = Some(part);
    }
    let part = last?;
    let slot = slot(tree, &model.children, model.hash, within, part);
    match slot.child {
        // A mapping a longer header implied is opened by its own header now, and keeps its anchor and its kind of first writing.
        Some(Child::Table(id)) => Some(id),
        Some(Child::Sequence { last: element, .. }) => Some(element),
        Some(Child::Value) => None,
        None => {
            entry(
                model,
                part,
                ValueKind::Mapping,
                within,
                None,
                Readability::Readable,
            );
            mapping(model, table, MappingKind::Standard, part, Count(0));
            put(&mut model.children, slot, part, Child::Table(table));
            Some(table)
        }
    }
}

/// Reads the header of the member `member` of an array of tables, and answers the mapping its entries go in.
fn member(tree: &Tree<'_>, model: &mut Model, member: Position) -> Option<Position> {
    let mut within = Position(0);
    let mut last = None;
    for part in parts(tree, member) {
        if let Some(before) = last {
            within = open(tree, model, within, before, MappingKind::Implicit)?;
        }
        last = Some(part);
    }
    let part = last?;
    let slot = slot(tree, &model.children, model.hash, within, part);
    let (opener, place) = match slot.child {
        Some(Child::Sequence { entry, members, .. }) => (entry, members),
        Some(Child::Table(_) | Child::Value) => return None,
        None => {
            entry(
                model,
                part,
                ValueKind::SequenceOfMappings,
                within,
                None,
                Readability::Readable,
            );
            (part, Count(0))
        }
    };
    put(
        &mut model.children,
        slot,
        part,
        Child::Sequence {
            entry: opener,
            members: Count(place.0 + 1),
            last: member,
        },
    );
    mapping(model, member, MappingKind::SequenceMember, member, place);
    element(model, member, ValueKind::Mapping, opener, place);
    Some(member)
}

/// Refills `model` with the model of the file of `tree`, worked out in one pass over its top-level elements: what the last file left is
/// cleared, and the room it took is kept.
pub(crate) fn build(tree: &Tree<'_>, model: &mut Model) {
    model.mappings.clear();
    model.entries.clear();
    model.elements.clear();
    model.children.clear();
    model.counts.clear();
    let root = Position(0);
    // The document is a table of keys: its root mapping, `standard`, anchored at the root and owned by it.
    mapping(model, root, MappingKind::Standard, root, Count(0));
    let mut within = Some(root);
    for each in languages::children(tree.elements, root) {
        let found = kind(tree, each);
        if found == Kind::Node(TomlNode::Keyval) {
            if let Some(owner) = within {
                pair(tree, model, owner, each);
            }
        } else if found == Kind::Node(TomlNode::StdTable) {
            within = table(tree, model, each);
        } else if found == Kind::Node(TomlNode::ArrayTable) {
            within = member(tree, model, each);
        } else {
            continue;
        }
        // A header's node holds the key-values below it.
        if let Some(owner) = within
            && kind(tree, each) != Kind::Node(TomlNode::Keyval)
        {
            for below in languages::children(tree.elements, each) {
                if kind(tree, below) == Kind::Node(TomlNode::Keyval) {
                    pair(tree, model, owner, below);
                }
            }
        }
    }
    model.mappings.sort_unstable_by_key(|each| each.id);
    model.entries.sort_unstable_by_key(|each| each.occurrence);
    model.elements.sort_unstable_by_key(|each| each.occurrence);
}
