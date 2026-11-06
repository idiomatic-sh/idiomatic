//! Lazily intern bottom-up parsed expression keys; source formatting and runtime value equality are separate.
use crate::domain::constant_keys::types::{KeyRecord, KeyRecordIndex, RetainedKeyIndex};
use crate::domain::model::types::{ShapeHash, ShapeState, ShapeWord};
use crate::domain::{
    extras::types::Extra,
    fields::service::field_of,
    labels::types::Field,
    nodes::types::Node,
    syntax::{
        constants::{
            BYTES_SCALAR, INTERPOLATION_CONVERSION, INTERPOLATION_DEBUG, SCALAR_IMAGINARY,
            SCALAR_KIND, SCALAR_VALUE,
        },
        service::{below, identifier_text, interpolation, is_token, kind, literal},
        types::{Kind, Leaf, Literal},
    },
    tokens::types::Token,
};
use ryo::languages::{self, NameText, Position, Symbols, Tree};
use std::cell::Ref;
use std::hash::{Hash as _, Hasher as _};

/// Concatenate only parser-decoded string token payloads; source spelling is never a fallback.
fn decoded_string_value(
    tree: &Tree<'_>,
    symbols: &mut Symbols,
    at: Position,
) -> Option<languages::Symbol> {
    let from = languages::begin_name(symbols);
    let mut found = false;
    for token in below(tree, at).filter(|each| {
        matches!(
            kind(tree, *each),
            Kind::Token(Token::String | Token::FstringMiddle | Token::TstringMiddle)
        )
    }) {
        languages::push_text(symbols, languages::decoded_at(tree.elements, token)?);
        found = true;
    }
    found.then(|| languages::end_name(symbols, tree.source, from))
}
/// Invalidate source-dependent classes while retaining all backing capacity.
pub(crate) fn clear(index: &mut RetainedKeyIndex<Position, ShapeWord, ShapeHash, ShapeState>) {
    index.classes.clear();
    index.words.clear();
    index.records.clear();
    index.buckets.clear();
    index.scratch.clear();
    index.state = ShapeState::Fresh;
}
/// The existing canonical representative of one occurrence.
#[must_use]
pub(crate) fn shape_of(
    index: &RetainedKeyIndex<Position, ShapeWord, ShapeHash, ShapeState>,
    at: Position,
) -> Option<Position> {
    let Ok(slot) = usize::try_from(at.0) else {
        return None;
    };
    index.classes.get(slot).copied().flatten()
}
/// Intern a full key. Hash collisions share a bucket, never an expression identity.
pub(super) fn intern(
    index: &mut RetainedKeyIndex<Position, ShapeWord, ShapeHash, ShapeState>,
    at: Position,
    hash: ShapeHash,
) -> Position {
    let mut candidate = index.buckets.get(&hash).copied();
    while let Some(place) = candidate {
        let Some(record) = index.records.get(place.0) else {
            break;
        };
        if index.words.get(record.words.start.0..record.words.end.0)
            == Some(index.scratch.as_slice())
        {
            return record.representative;
        }
        candidate = record.previous;
    }
    let start = index.words.len();
    index.words.extend_from_slice(&index.scratch);
    let record = KeyRecord {
        words: crate::domain::constant_keys::types::WordRange {
            start: crate::domain::constant_keys::types::KeyWordIndex(start),
            end: crate::domain::constant_keys::types::KeyWordIndex(index.words.len()),
        },
        representative: at,
        previous: index.buckets.get(&hash).copied(),
    };
    let place = KeyRecordIndex(index.records.len());
    index.records.push(record);
    let _previous_bucket_head = index.buckets.insert(hash, place);
    at
}
/// Append one normalized child class and its semantic field.
fn child(words: &mut Vec<ShapeWord>, classes: &[Option<Position>], field: Field, at: Position) {
    if let Some(Some(class)) = classes.get(usize::try_from(at.0).unwrap_or(usize::MAX)) {
        words.push(ShapeWord::Child(field, *class));
    }
}
/// Operators are meaningful; separators, redundant parentheses and lexical trivia are not.
fn significant(token: Token) -> Option<Token> {
    matches!(
        token,
        Token::Name
            | Token::Plus
            | Token::Minus
            | Token::Star
            | Token::Slash
            | Token::Vbar
            | Token::Amper
            | Token::Less
            | Token::Greater
            | Token::Percent
            | Token::Eqequal
            | Token::Notequal
            | Token::Lessequal
            | Token::Greaterequal
            | Token::Tilde
            | Token::Circumflex
            | Token::Leftshift
            | Token::Rightshift
            | Token::Doublestar
            | Token::Doubleslash
            | Token::At
            | Token::Ellipsis
    )
    .then_some(token)
}
/// Preserve the parser's literal category and payload, or its already-decoded text.
fn constant(
    tree: &Tree<'_>,
    at: Position,
    symbols: &mut Symbols,
    classes: &[Option<Position>],
    words: &mut Vec<ShapeWord>,
) -> Option<()> {
    if languages::property_at(tree.elements, at, SCALAR_KIND)
        .is_some_and(|kind| kind.0 == BYTES_SCALAR)
    {
        // Bytes share the parser's string-part node shape, but their decoded scalar payload is
        // already present and must not be sent through the text-only string decoder.
    } else if literal(tree, at) == Literal::String
        || kind(tree, at) == Kind::Node(Node::Constant)
            && below(tree, at).any(|child| {
                matches!(
                    kind(tree, child),
                    Kind::Token(Token::String | Token::FstringMiddle | Token::TstringMiddle)
                )
            })
    {
        if interpolation(tree, at) == crate::domain::syntax::types::Interpolation::Static {
            words.push(ShapeWord::Literal(decoded_string_value(tree, symbols, at)?));
        } else {
            segments(tree, at, symbols, classes, words);
            merge_literals(tree, symbols, words);
        }
    } else {
        // Scalars require the parser's typed payload, never physical carrier text.
        let _scalar_kind = languages::property_at(tree.elements, at, SCALAR_KIND)?;
    }
    Some(())
}
/// Flatten only concatenation wrappers; expressions nested in interpolations remain child classes.
fn segments(
    tree: &Tree<'_>,
    at: Position,
    symbols: &mut Symbols,
    classes: &[Option<Position>],
    words: &mut Vec<ShapeWord>,
) {
    for each in languages::children(tree.elements, at) {
        match kind(tree, each) {
            Kind::Extra(Extra::FStringPart | Extra::TStringPart) => {
                segments(tree, each, symbols, classes, words);
            }
            Kind::Extra(Extra::StringPart) => {
                if let Some(value) = decoded_string_value(tree, symbols, each) {
                    words.push(ShapeWord::Literal(value));
                }
            }
            Kind::Node(Node::Constant) => {
                let _constant_result = constant(tree, each, symbols, classes, words);
            }
            Kind::Node(Node::FormattedValue | Node::Interpolation) => {
                child(words, classes, Field::Values, each);
            }
            Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => {}
        }
    }
}
/// Merge adjacent decoded segments once per run, avoiding repeated prefix copying.
fn merge_literals(tree: &Tree<'_>, symbols: &mut Symbols, words: &mut Vec<ShapeWord>) {
    let mut read = 0;
    let mut write = 0;
    while read < words.len() {
        let Some(word) = words.get(read).copied() else {
            break;
        };
        let mut end = read + 1;
        let merged = if let ShapeWord::Literal(first) = word {
            while matches!(words.get(end), Some(ShapeWord::Literal(_))) {
                end += 1;
            }
            if end == read + 1 {
                ShapeWord::Literal(first)
            } else {
                let begin = languages::begin_name(symbols);
                for each in words.get(read..end).unwrap_or_default() {
                    if let ShapeWord::Literal(value) = each {
                        languages::push_name(symbols, tree.source, *value);
                    }
                }
                ShapeWord::Literal(languages::end_name(symbols, tree.source, begin))
            }
        } else {
            word
        };
        if let Some(slot) = words.get_mut(write) {
            *slot = merged;
        }
        write += 1;
        read = end;
    }
    words.truncate(write);
}
/// Build one non-recursive key from already-interned children.
fn key(
    tree: &Tree<'_>,
    at: Position,
    symbols: &mut Symbols,
    classes: &[Option<Position>],
    words: &mut Vec<ShapeWord>,
) -> Option<()> {
    words.clear();
    let node = kind(tree, at);
    words.push(ShapeWord::Kind(node));
    for property in [
        SCALAR_KIND,
        SCALAR_VALUE,
        SCALAR_IMAGINARY,
        INTERPOLATION_DEBUG,
        INTERPOLATION_CONVERSION,
    ] {
        if let Some(value) = languages::property_at(tree.elements, at, property) {
            words.push(ShapeWord::Property(
                property,
                languages::intern_derived(symbols, tree.source, value),
            ));
        }
    }
    match node {
        Kind::Node(Node::Name) | Kind::Extra(Extra::Identifier) => {
            words.push(ShapeWord::Identifier(languages::intern_derived(
                symbols,
                tree.source,
                NameText(identifier_text(tree, at).0),
            )));
            return Some(());
        }
        Kind::Node(Node::Constant) => {
            return constant(tree, at, symbols, classes, words);
        }
        Kind::Node(Node::JoinedStr | Node::TemplateStr) => {
            segments(tree, at, symbols, classes, words);
            merge_literals(tree, symbols, words);
            return Some(());
        }
        Kind::Extra(Extra::CallArguments) => {
            for wanted in [Field::Args, Field::Keywords] {
                for each in languages::children(tree.elements, at).filter(|each| {
                    is_token(tree, *each) == Leaf::Node && field_of(tree, *each) == wanted
                }) {
                    child(words, classes, wanted, each);
                }
            }
            return Some(());
        }
        Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => {}
    }
    for each in languages::children(tree.elements, at) {
        if is_token(tree, each) == Leaf::Node {
            child(words, classes, field_of(tree, each), each);
        } else if let Kind::Token(token) = kind(tree, each)
            && significant(token).is_some()
        {
            words.push(ShapeWord::Token(
                kind(tree, each),
                languages::text_at(tree.elements, each),
            ));
        } else {
            // Delimiters and trivia do not contribute structural identity.
        }
    }
    Some(())
}
/// Build at most once per parse. Incomplete frontend output has no apparently known class rows.
pub(crate) fn prepare<'a>(
    tree: &Tree<'_>,
    model: &'a crate::domain::model::types::Declarations,
    symbols: &mut Symbols,
) -> Option<Ref<'a, RetainedKeyIndex<Position, ShapeWord, ShapeHash, ShapeState>>> {
    let held = &model.expression_shapes;
    if held.borrow().state == ShapeState::Fresh {
        let mut index = held.borrow_mut();
        if languages::has_incomplete_syntax(tree.elements) {
            index.state = ShapeState::Partial;
            return None;
        }
        let Ok(count) = usize::try_from(languages::end_of(tree.elements).0) else {
            return None;
        };
        index.classes.resize(count, None);
        for raw in (0..count).rev() {
            let Ok(position) = u32::try_from(raw) else {
                return None;
            };
            let at = Position(position);
            if is_token(tree, at) == Leaf::Token
                || crate::domain::semantic_syntax::service::visibility(tree, model, at, symbols)
                    != crate::domain::semantic_syntax::types::Visibility::Visible
            {
                continue;
            }
            if kind(tree, at) == Kind::Extra(Extra::IpyEscapeCommand) {
                index.state = ShapeState::Partial;
                continue;
            }
            let RetainedKeyIndex {
                classes, scratch, ..
            } = &mut *index;
            if languages::children(tree.elements, at).any(|child| {
                is_token(tree, child) == Leaf::Node
                    && classes
                        .get(usize::try_from(child.0).unwrap_or(usize::MAX))
                        .is_none_or(Option::is_none)
            }) {
                index.state = ShapeState::Partial;
                continue;
            }
            if key(tree, at, symbols, classes, scratch).is_none() {
                index.state = ShapeState::Partial;
                continue;
            }
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            index.scratch.hash(&mut hasher);
            let class = intern(&mut index, at, ShapeHash(hasher.finish()));
            if let Some(slot) = index.classes.get_mut(raw) {
                *slot = Some(class);
            }
        }
        if index.state == ShapeState::Fresh {
            index.state = ShapeState::Complete;
        }
    }
    let index = held.borrow();
    (index.state == ShapeState::Complete).then_some(index)
}
