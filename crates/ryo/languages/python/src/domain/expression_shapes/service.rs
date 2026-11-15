//! Lazily intern bottom-up parsed expression keys; source formatting and runtime value equality are separate.
use crate::domain::{
    extras::types::Extra,
    fields::service::field_of,
    labels::{service::field_bits, types::Field},
    nodes::types::Node,
    syntax::{
        constants::{
            BYTES_SCALAR, INTERPOLATION_CONVERSION, INTERPOLATION_DEBUG, SCALAR_IMAGINARY,
            SCALAR_KIND, SCALAR_VALUE,
        },
        service::{below, identifier_text, interpolation, is_token, kind, kind_bits, literal},
        types::{Kind, Leaf, Literal},
    },
    tokens::types::Token,
};
use ryo::languages::{
    self, KeptShapeWords, NameText, Position, ShapeClasses, ShapeExtent, ShapeKey, ShapeStep,
    ShapeWord, Symbols, Tree, child_class, push_shape_word, rewrite_shape_words,
};

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
/// Append one normalized child class and its semantic field.
fn child(key: &mut ShapeKey<'_>, field: Field, at: Position) {
    if let Some(class) = child_class(key, at) {
        push_shape_word(key, ShapeWord::Child(field_bits(field), class));
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
    key: &mut ShapeKey<'_>,
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
            push_shape_word(
                key,
                ShapeWord::Literal(decoded_string_value(tree, symbols, at)?),
            );
        } else {
            segments(tree, at, symbols, key);
            rewrite_shape_words(key, |words| merge_literals(tree, symbols, words));
        }
    } else {
        // Scalars require the parser's typed payload, never physical carrier text.
        let _scalar_kind = languages::property_at(tree.elements, at, SCALAR_KIND)?;
    }
    Some(())
}
/// Flatten only concatenation wrappers; expressions nested in interpolations remain child classes.
fn segments(tree: &Tree<'_>, at: Position, symbols: &mut Symbols, key: &mut ShapeKey<'_>) {
    for each in languages::children(tree.elements, at) {
        match kind(tree, each) {
            Kind::Extra(Extra::FStringPart | Extra::TStringPart) => {
                segments(tree, each, symbols, key);
            }
            Kind::Extra(Extra::StringPart) => {
                if let Some(value) = decoded_string_value(tree, symbols, each) {
                    push_shape_word(key, ShapeWord::Literal(value));
                }
            }
            Kind::Node(Node::Constant) => {
                let _constant_result = constant(tree, each, symbols, key);
            }
            Kind::Node(Node::FormattedValue | Node::Interpolation) => {
                child(key, Field::Values, each);
            }
            Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => {}
        }
    }
}
/// Merge adjacent decoded segments once per run, avoiding repeated prefix copying.
fn merge_literals(
    tree: &Tree<'_>,
    symbols: &mut Symbols,
    words: &mut [ShapeWord],
) -> KeptShapeWords {
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
    KeptShapeWords(write)
}
/// Build one non-recursive key from already-interned children.
fn key(tree: &Tree<'_>, at: Position, symbols: &mut Symbols, key: &mut ShapeKey<'_>) -> Option<()> {
    let node = kind(tree, at);
    push_shape_word(key, ShapeWord::Kind(kind_bits(node)));
    for property in [
        SCALAR_KIND,
        SCALAR_VALUE,
        SCALAR_IMAGINARY,
        INTERPOLATION_DEBUG,
        INTERPOLATION_CONVERSION,
    ] {
        if let Some(value) = languages::property_at(tree.elements, at, property) {
            push_shape_word(
                key,
                ShapeWord::Property(
                    property,
                    languages::intern_derived(symbols, tree.source, value),
                ),
            );
        }
    }
    match node {
        Kind::Node(Node::Name) | Kind::Extra(Extra::Identifier) => {
            push_shape_word(
                key,
                ShapeWord::Identifier(languages::intern_derived(
                    symbols,
                    tree.source,
                    NameText(identifier_text(tree, at).0),
                )),
            );
            return Some(());
        }
        Kind::Node(Node::Constant) => {
            return constant(tree, at, symbols, key);
        }
        Kind::Node(Node::JoinedStr | Node::TemplateStr) => {
            segments(tree, at, symbols, key);
            rewrite_shape_words(key, |words| merge_literals(tree, symbols, words));
            return Some(());
        }
        Kind::Extra(Extra::CallArguments) => {
            for wanted in [Field::Args, Field::Keywords] {
                for each in languages::children(tree.elements, at).filter(|each| {
                    is_token(tree, *each) == Leaf::Node && field_of(tree, *each) == wanted
                }) {
                    child(key, wanted, each);
                }
            }
            return Some(());
        }
        Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => {}
    }
    for each in languages::children(tree.elements, at) {
        if is_token(tree, each) == Leaf::Node {
            child(key, field_of(tree, each), each);
        } else if let Kind::Token(token) = kind(tree, each)
            && significant(token).is_some()
        {
            push_shape_word(
                key,
                ShapeWord::Token(
                    kind_bits(kind(tree, each)),
                    languages::text_at(tree.elements, each),
                ),
            );
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
) -> Option<ShapeClasses<'a>> {
    languages::prepare_shapes(
        &model.expression_shapes,
        tree.elements,
        ShapeExtent::Table,
        |at, built| {
            if crate::domain::semantic_syntax::service::visibility(tree, model, at, symbols)
                != crate::domain::semantic_syntax::types::Visibility::Visible
            {
                return ShapeStep::Skipped;
            }
            if kind(tree, at) == Kind::Extra(Extra::IpyEscapeCommand) {
                return ShapeStep::Unbuilt;
            }
            if languages::children(tree.elements, at).any(|child| {
                is_token(tree, child) == Leaf::Node && child_class(built, child).is_none()
            }) {
                return ShapeStep::Unbuilt;
            }
            match key(tree, at, symbols, built) {
                Some(()) => ShapeStep::Built,
                None => ShapeStep::Unbuilt,
            }
        },
    )
}
