//! The `fact_extraction` service: what a file says of its modules, read from the element table the parse has built, with no second parse,
//! and written as it is found into the notes the project reads: the names are borrowed from the file and nothing is made for them.

use crate::domain::nodes::types::Node;
use crate::domain::tokens::types::Token;
use ryo::languages::{self, Position, Tree};
use std::borrow::Cow;

use super::constants::{PATH_ATTRIBUTE, RAW_PREFIX, TEST_ATTRIBUTE};
use crate::domain::fact_notes::types::{
    DeclaredNote, Note, NoteFlag, NoteName, NoteOffset, NotePath, StepNote, WrittenNote,
};
use crate::domain::module_tree::constants::{CRATE, GLOB, SELF, SELF_TYPE, SUPER};
use crate::domain::syntax::service::{
    ancestors, child, end, is_token, kind, kind_bits, parent, start,
};
use crate::domain::syntax::types::{Kind, Leaf, TokenText};

/// The text of the element at `at`, as the file wrote it.
fn written_text<'a>(tree: &Tree<'a>, at: Position) -> TokenText<'a> {
    let (from, to) = (start(tree, at).0, end(tree, at).0);
    TokenText(
        tree.source
            .0
            .get(
                usize::try_from(from).unwrap_or(usize::MAX)
                    ..usize::try_from(to).unwrap_or(usize::MAX),
            )
            // the bytes of a node that are not text have no text, which the reader takes as an empty name: the row keeps its place either way
            .map(|bytes| std::str::from_utf8(bytes).unwrap_or_default())
            .unwrap_or_default(),
    )
}

/// The text of the element at `at` as the file wrote it, its whitespace left out: borrowed from the file when it holds none.
fn spelled<'a>(tree: &Tree<'a>, at: Position) -> NoteName<'a> {
    let text = written_text(tree, at).0;
    if text.contains(char::is_whitespace) {
        NoteName(Cow::Owned(text.split_whitespace().collect()))
    } else {
        NoteName(Cow::Borrowed(text))
    }
}

/// The name a `NAME` or `NAME_REF` at `at` holds: the text of its first token, a raw identifier's `r#` left out, as rustc
/// reads it (`mod r#type;` is the module `type`, in `type.rs`).
fn first_token<'a>(tree: &Tree<'a>, at: Position) -> Option<TokenText<'a>> {
    languages::children(tree.elements, at)
        .find(|each| is_token(tree, *each) == Leaf::Token)
        .map(|each| written_text(tree, each))
        .map(|text| TokenText(text.0.strip_prefix(RAW_PREFIX).unwrap_or(text.0)))
}

/// What the file at `tree` says of its modules.
#[must_use]
pub(crate) fn note_of<'a>(tree: &Tree<'a>) -> Note<'a> {
    let mut note = Note {
        clean: NoteFlag(languages::diagnostic_count(tree.elements).0 == 0),
        declared: Vec::new(),
        written: Vec::new(),
    };
    if !tree.elements.size.is_empty() {
        declarations(tree, Position(0), &mut Vec::new(), &mut note.declared);
        written_paths(tree, &mut note.written);
    }
    note
}

/// Every `mod` declaration among the children of `node`, in source order, each with the inline blocks around it.
fn declarations<'a>(
    tree: &Tree<'a>,
    node: Position,
    around: &mut Vec<StepNote<'a>>,
    declared: &mut Vec<DeclaredNote<'a>>,
) {
    for item in languages::children(tree.elements, node) {
        if kind(tree, item) != Kind::Node(Node::Module) {
            continue;
        }
        let Some(name) =
            child(tree, item, Kind::Node(Node::Name)).and_then(|each| first_token(tree, each))
        else {
            continue;
        };
        let path = path_attribute(tree, item);
        let list = child(tree, item, Kind::Node(Node::ItemList));
        declared.push(DeclaredNote {
            around: around.clone(),
            name: NoteName(Cow::Borrowed(name.0)),
            inline: NoteFlag(list.is_some()),
            path: path.clone(),
            test_only: test_attribute(tree, item),
        });
        if let Some(list) = list {
            let depth = around.len();
            around.push(StepNote {
                name: NoteName(Cow::Borrowed(name.0)),
                path,
            });
            declarations(tree, list, around, declared);
            around.truncate(depth);
        }
    }
}

/// Whether the string literal at `at` is an import specifier: the file a `#[path = "…"]` attribute loads a module from. `include!`
/// is no expression macro, so its argument is no `string_literal` row.
pub(crate) fn specifier(tree: &Tree<'_>, at: Position) -> languages::Flag {
    let meta = parent(tree, at);
    let loads = kind(tree, meta) == Kind::Node(Node::KeyValueMeta)
        && child(tree, meta, Kind::Node(Node::Path))
            .is_some_and(|path| written_text(tree, path).0 == PATH_ATTRIBUTE);
    if loads {
        languages::Flag::Raised
    } else {
        languages::Flag::Lowered
    }
}

/// Whether `item` carries `#[cfg(test)]`, its whitespace left out.
fn test_attribute(tree: &Tree<'_>, item: Position) -> NoteFlag {
    NoteFlag(
        languages::children(tree.elements, item)
            .filter(|each| kind(tree, *each) == Kind::Node(Node::Attr))
            .any(|attr| spelled(tree, attr).0 == TEST_ATTRIBUTE),
    )
}

/// The value of `item`'s `#[path = "…"]`, when it has one.
fn path_attribute(tree: &Tree<'_>, item: Position) -> Option<NotePath> {
    languages::children(tree.elements, item)
        .filter(|each| kind(tree, *each) == Kind::Node(Node::Attr))
        .find_map(|attr| {
            let meta = child(tree, attr, Kind::Node(Node::KeyValueMeta))?;
            if written_text(tree, child(tree, meta, Kind::Node(Node::Path))?).0 != PATH_ATTRIBUTE {
                return None;
            }
            let literal = child(tree, meta, Kind::Node(Node::Literal))?;
            let text = languages::children(tree.elements, literal)
                .find(|each| kind(tree, *each) == Kind::Token(Token::String))?;
            languages::decoded_at(tree.elements, text).map(|value| NotePath(value.0.to_owned()))
        })
}

/// Every `use` target and every outermost path of two segments or more in the file, in source order. A path
/// inside generic arguments or a qualified self hangs off a segment, not a qualifier, and counts.
fn written_paths<'a>(tree: &Tree<'a>, out: &mut Vec<WrittenNote<'a>>) {
    let (use_bits, path_bits) = (
        kind_bits(Kind::Node(Node::Use)),
        kind_bits(Kind::Node(Node::Path)),
    );
    let mut prefix: Vec<TokenText<'a>> = Vec::new();
    for (place, bits) in tree.elements.kind.iter().enumerate() {
        let node = Position(u32::try_from(place).unwrap_or(u32::MAX));
        if *bits == use_bits {
            if let Some(use_tree) = child(tree, node, Kind::Node(Node::UseTree)) {
                prefix.clear();
                flatten(tree, use_tree, &mut prefix, node, out);
            }
            continue;
        }
        if *bits != path_bits || kind(tree, parent(tree, node)) == Kind::Node(Node::Path) {
            continue;
        }
        let mut segments: Vec<NoteName<'a>> = Vec::new();
        segments_of(tree, node, &mut |segment| {
            segments.push(NoteName(Cow::Borrowed(segment.0)));
        });
        if segments.len() < 2
            || ancestors(tree, node).any(|each| kind(tree, each) == Kind::Node(Node::UseTree))
        {
            continue;
        }
        out.push(WrittenNote {
            segments,
            enclosing: enclosing_of(tree, node),
            start: NoteOffset(start(tree, node).0),
            end: NoteOffset(end(tree, node).0),
            text: spelled(tree, node),
            bound: None,
            block: None,
        });
    }
}

/// The names of the inline `mod name { … }` blocks around `node`, outermost first.
fn enclosing_of<'a>(tree: &Tree<'a>, node: Position) -> Vec<NoteName<'a>> {
    let blocks = |each: &Position| {
        kind(tree, *each) == Kind::Node(Node::Module)
            && child(tree, *each, Kind::Node(Node::ItemList)).is_some()
    };
    let name_of = |each: Position| {
        let name = child(tree, each, Kind::Node(Node::Name))?;
        first_token(tree, name)
    };
    let around: Vec<Position> = ancestors(tree, node).filter(blocks).collect();
    around
        .iter()
        .rev()
        .filter_map(|each| name_of(*each))
        .map(|name| NoteName(Cow::Borrowed(name.0)))
        .collect()
}

/// The block of statements around `node`, when it is nearer than any inline module around it: where a `use` written there binds.
fn block_of(tree: &Tree<'_>, node: Position) -> Option<languages::Span> {
    let scope = ancestors(tree, node).find(|each| {
        matches!(
            kind(tree, *each),
            Kind::Node(Node::StmtList | Node::ItemList)
        )
    })?;
    (kind(tree, scope) == Kind::Node(Node::StmtList)).then(|| languages::Span {
        start: start(tree, scope),
        end: end(tree, scope),
    })
}

/// Adds each leaf of `use_tree`, under the segments `prefix` its groups write.
fn flatten<'a>(
    tree: &Tree<'a>,
    use_tree: Position,
    prefix: &mut Vec<TokenText<'a>>,
    use_node: Position,
    out: &mut Vec<WrittenNote<'a>>,
) {
    let before = prefix.len();
    if let Some(path) = child(tree, use_tree, Kind::Node(Node::Path)) {
        let mut local: Vec<TokenText<'_>> = Vec::new();
        segments_of(tree, path, &mut |segment| local.push(segment));
        // A lone `self` inside a list names the list's prefix itself, which is written already.
        let names_prefix = matches!(local.as_slice(), [only] if only.0 == SELF) && before != 0;
        if !names_prefix {
            prefix.extend(local);
        }
    }
    if let Some(list) = child(tree, use_tree, Kind::Node(Node::UseTreeList)) {
        for each in languages::children(tree.elements, list)
            .filter(|each| kind(tree, *each) == Kind::Node(Node::UseTree))
        {
            flatten(tree, each, prefix, use_node, out);
        }
        prefix.truncate(before);
        return;
    }
    let glob = languages::children(tree.elements, use_tree).any(|each| {
        (is_token(tree, each) == Leaf::Token) && kind(tree, each) == Kind::Token(Token::Star)
    });
    if !prefix.is_empty() || glob {
        let mut segments: Vec<NoteName<'a>> = prefix
            .iter()
            .map(|segment| NoteName(Cow::Borrowed(segment.0)))
            .collect();
        if glob {
            segments.push(NoteName(Cow::Borrowed(GLOB)));
        }
        // The name the leaf binds: its `as` name (none for `as _`), or else its last segment; a glob binds no one name.
        let bound = match child(tree, use_tree, Kind::Node(Node::Rename)) {
            Some(rename) => child(tree, rename, Kind::Node(Node::Name))
                .and_then(|name| first_token(tree, name))
                .map(|name| NoteName(Cow::Borrowed(name.0))),
            None if glob => None,
            None => prefix.last().map(|last| NoteName(Cow::Borrowed(last.0))),
        };
        out.push(WrittenNote {
            segments,
            enclosing: enclosing_of(tree, use_node),
            start: NoteOffset(start(tree, use_tree).0),
            end: NoteOffset(end(tree, use_tree).0),
            text: spelled(tree, use_tree),
            bound,
            block: block_of(tree, use_node),
        });
    }
    prefix.truncate(before);
}

/// Calls `each` with the segments of the path at `path`, qualifiers first, keywords as written and generic arguments left out.
fn segments_of<'a>(tree: &Tree<'a>, path: Position, each: &mut dyn FnMut(TokenText<'a>)) {
    for part in languages::children(tree.elements, path) {
        let part_kind = kind(tree, part);
        if part_kind == Kind::Node(Node::Path) {
            segments_of(tree, part, each);
            continue;
        }
        if part_kind == Kind::Node(Node::PathSegment)
            && let Some(segment) = segment_of(tree, part)
        {
            each(segment);
        }
    }
}

/// The name a path segment writes: none for `<T as Trait>`, which writes a type.
fn segment_of<'a>(tree: &Tree<'a>, segment: Position) -> Option<TokenText<'a>> {
    if let Some(name) = child(tree, segment, Kind::Node(Node::NameRef)) {
        return first_token(tree, name);
    }
    languages::children(tree.elements, segment)
        .find(|each| {
            (is_token(tree, *each) == Leaf::Token)
                && matches!(
                    kind(tree, *each),
                    Kind::Token(
                        Token::SelfTypeKw | Token::SelfKw | Token::SuperKw | Token::CrateKw
                    )
                )
        })
        .map(|each| written_text(tree, each))
        .filter(|text| [CRATE, SELF, SUPER, SELF_TYPE].contains(&text.0))
}
