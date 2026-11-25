//! The `shape_facts` service: free functions over the nodes of a walked file. Two nodes have one shape when they are written
//! with the same structure: the same kind, the same payload (a name's spelling, a literal's decoded value, an interpolation's
//! conversion), the same operators and keywords, and children of the same shapes in the same fields; parentheses, delimiters and
//! trivia are no structure. A shape is a key of the engine's shape words, interned exactly; a class is named by its last member
//! in the file, and a node that holds a node with no shape has none.

use super::types::{Pending, ShapeBuild, ShapeReading};
use crate::domain::expression_shapes::service::significant;
use crate::domain::extras::types::Extra;
use crate::domain::labels::service::field_bits;
use crate::domain::labels::types::Field;
use crate::domain::node::service::{field_code, id_of, place_of_id, table_id, word, written};
use crate::domain::node::types::SyntaxFile;
use crate::domain::syntax::constants::{
    BOOLEAN_SCALAR, BYTES_SCALAR, COMPLEX_SCALAR, ELLIPSIS_SCALAR, FLOAT_SCALAR, INTEGER_SCALAR,
    INTERPOLATION_CONVERSION, INTERPOLATION_DEBUG, NONE_SCALAR, SCALAR_IMAGINARY, SCALAR_KIND,
    SCALAR_VALUE,
};
use crate::domain::syntax::service::kind_of;
use crate::domain::syntax::types::Kind;
use ruff_python_ast::{
    AnyNodeRef, FStringPartRef, InterpolatedStringElement, InterpolatedStringElements, Number,
};
use ryo::languages::{
    FieldId, NameText, Position, PropertyId, ShapeWord, clear_keys, intern_key, node_table,
    push_key_word,
};
use std::fmt::Write as _;

/// The class a node was found to have.
fn class_of(build: &ShapeBuild<'_>, id: Position) -> Option<Position> {
    build.index.classes.get(usize::from(id)).copied().flatten()
}

/// Appends a property of the node whose value is one of the language's fixed words.
fn property(build: &mut ShapeBuild<'_>, named: PropertyId, value: NameText<'_>) {
    let text = word(build.words, value);
    let _room = push_key_word(build.index, ShapeWord::Property(named, text));
}

/// Appends a property whose value is written out: a number's value, an interpolation's conversion.
fn written_property(build: &mut ShapeBuild<'_>, named: PropertyId, value: impl std::fmt::Display) {
    build.text.0.clear();
    // Writing into a `String` does not fail.
    let _written: std::fmt::Result = write!(build.text.0, "{value}");
    let text = word(build.words, NameText(&build.text.0));
    let _room = push_key_word(build.index, ShapeWord::Property(named, text));
    build.text.0.clear();
}

/// Appends the payload the parser holds for a node: the kind and the value of a scalar, the debug text and the conversion of an
/// interpolation. A node of any other kind holds none.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only the scalars and the interpolation hold a payload; every other node holds none"
)]
fn payload(build: &mut ShapeBuild<'_>, node: AnyNodeRef<'_>) {
    match node {
        AnyNodeRef::ExprNumberLiteral(number) => match &number.value {
            Number::Int(integer) => {
                property(build, SCALAR_KIND, NameText(INTEGER_SCALAR));
                written_property(build, SCALAR_VALUE, integer);
            }
            Number::Float(float) => {
                property(build, SCALAR_KIND, NameText(FLOAT_SCALAR));
                written_property(build, SCALAR_VALUE, float.to_bits());
            }
            Number::Complex { real, imag } => {
                property(build, SCALAR_KIND, NameText(COMPLEX_SCALAR));
                written_property(build, SCALAR_VALUE, real.to_bits());
                written_property(build, SCALAR_IMAGINARY, imag.to_bits());
            }
        },
        AnyNodeRef::ExprBooleanLiteral(boolean) => {
            property(build, SCALAR_KIND, NameText(BOOLEAN_SCALAR));
            written_property(build, SCALAR_VALUE, boolean.value);
        }
        AnyNodeRef::ExprNoneLiteral(_) => property(build, SCALAR_KIND, NameText(NONE_SCALAR)),
        AnyNodeRef::ExprEllipsisLiteral(_) => {
            property(build, SCALAR_KIND, NameText(ELLIPSIS_SCALAR));
        }
        AnyNodeRef::ExprBytesLiteral(bytes) => {
            property(build, SCALAR_KIND, NameText(BYTES_SCALAR));
            build.text.0.clear();
            for byte in bytes.value.bytes() {
                // Writing into a `String` does not fail.
                let _written: std::fmt::Result = write!(build.text.0, "{byte:02x}");
            }
            let text = word(build.words, NameText(&build.text.0));
            let _room = push_key_word(build.index, ShapeWord::Property(SCALAR_VALUE, text));
            build.text.0.clear();
        }
        AnyNodeRef::InterpolatedElement(interpolation) => {
            if let Some(debug) = &interpolation.debug_text {
                // A line ending is one character of the text, however the file writes it.
                let mut spelled = debug.as_str().chars().peekable();
                build.text.0.clear();
                while let Some(character) = spelled.next() {
                    if character == '\r' {
                        let _line_feed = spelled.next_if_eq(&'\n');
                        build.text.0.push('\n');
                    } else {
                        build.text.0.push(character);
                    }
                }
                let text = word(build.words, NameText(&build.text.0));
                let _room =
                    push_key_word(build.index, ShapeWord::Property(INTERPOLATION_DEBUG, text));
                build.text.0.clear();
            }
            if let Some(conversion) = interpolation.conversion.to_char() {
                written_property(build, INTERPOLATION_CONVERSION, conversion);
            }
        }
        _ => {}
    }
}

/// Appends the literal text that waits in the buffer, if any, as one piece.
fn flush(build: &mut ShapeBuild<'_>, pending: &mut Pending) {
    if *pending == Pending::Text {
        let text = word(build.words, NameText(&build.text.0));
        let _room = push_key_word(build.index, ShapeWord::Literal(text));
        build.text.0.clear();
        *pending = Pending::Nothing;
    }
}

/// Appends the literal pieces and the interpolations of an f-string, a t-string or a format spec: adjacent literal pieces are one
/// piece, and an interpolation is a child of the string whatever part of it holds it.
fn segments(
    build: &mut ShapeBuild<'_>,
    elements: &InterpolatedStringElements,
    pending: &mut Pending,
) {
    for element in elements {
        match element {
            InterpolatedStringElement::Literal(literal) => {
                build.text.0.push_str(&literal.value);
                *pending = Pending::Text;
            }
            InterpolatedStringElement::Interpolation(_) => {
                flush(build, pending);
                if let Some(class) = id_of(element).and_then(|id| class_of(build, id)) {
                    let _room = push_key_word(
                        build.index,
                        ShapeWord::Child(field_bits(Field::Values), class),
                    );
                }
            }
        }
    }
}

/// Appends what a name or a string is by its value, not by its children: a name's spelling as the language reads it, a string's
/// decoded text; a scalar is its payload alone. Answers how `node` is read.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only the names and the strings are read by their value; every other node is read by its children"
)]
fn valued(build: &mut ShapeBuild<'_>, node: AnyNodeRef<'_>) -> ShapeReading {
    let mut pending = Pending::Nothing;
    build.text.0.clear();
    let name = match node {
        AnyNodeRef::ExprName(name) => Some(name.id.as_str()),
        AnyNodeRef::Identifier(identifier) => Some(identifier.id.as_str()),
        AnyNodeRef::ExprStringLiteral(string) => {
            for part in &string.value {
                build.text.0.push_str(&part.value);
            }
            pending = Pending::Text;
            None
        }
        AnyNodeRef::InterpolatedStringLiteralElement(literal) => {
            build.text.0.push_str(&literal.value);
            pending = Pending::Text;
            None
        }
        AnyNodeRef::ExprFString(string) => {
            for part in &string.value {
                match part {
                    FStringPartRef::Literal(literal) => {
                        build.text.0.push_str(&literal.value);
                        pending = Pending::Text;
                    }
                    FStringPartRef::FString(formatted) => {
                        segments(build, &formatted.elements, &mut pending);
                    }
                }
            }
            None
        }
        AnyNodeRef::ExprTString(string) => {
            for part in &string.value {
                segments(build, &part.elements, &mut pending);
            }
            None
        }
        AnyNodeRef::InterpolatedStringFormatSpec(spec) => {
            segments(build, &spec.elements, &mut pending);
            None
        }
        AnyNodeRef::ExprBytesLiteral(_)
        | AnyNodeRef::ExprNumberLiteral(_)
        | AnyNodeRef::ExprBooleanLiteral(_)
        | AnyNodeRef::ExprNoneLiteral(_)
        | AnyNodeRef::ExprEllipsisLiteral(_) => None,
        _ => return ShapeReading::ByChildren,
    };
    if let Some(spelling) = name {
        let text = word(build.words, NameText(spelling));
        let _room = push_key_word(build.index, ShapeWord::Identifier(text));
    }
    flush(build, &mut pending);
    ShapeReading::ByValue
}

/// Appends the children of a node in the order they are written: a node child by its field and its class, an operator or a
/// keyword by its kind and its text. The arguments of a call are read positional first, then by keyword.
fn children(file: &SyntaxFile<'_>, build: &mut ShapeBuild<'_>, id: Position, kind: Kind) {
    let listed = node_table::children_of(file.table, id);
    if kind == Kind::Extra(Extra::CallArguments) {
        for wanted in [Field::Args, Field::Keywords] {
            for child in listed {
                if let Some(place) = place_of_id(file.tree, *child)
                    && node_table::field_of(file.table, *child) == field_code(wanted)
                    && let Some(class) = class_of(build, place)
                {
                    let _room =
                        push_key_word(build.index, ShapeWord::Child(field_bits(wanted), class));
                }
            }
        }
        return;
    }
    for child in listed {
        let bits = node_table::kind_of(file.table, *child);
        if let Some(place) = place_of_id(file.tree, *child) {
            if let Some(class) = class_of(build, place) {
                let field = FieldId(node_table::field_of(file.table, *child).0);
                let _room = push_key_word(build.index, ShapeWord::Child(field, class));
            }
        } else if let Kind::Token(token) = kind_of(bits)
            && significant(token).is_some()
        {
            let text = word(build.words, node_table::decoded_of(file.table, *child).unwrap_or_else(|| written(file, *child)));
            let _room = push_key_word(build.index, ShapeWord::Token(bits, text));
        } else {
            // A delimiter or trivia: no structure.
        }
    }
}

/// Finds the shape of every node of `file`, by node id: one pass from the last node to the first, so a child's class is known
/// before the shape of what holds it is asked.
pub(crate) fn shapes(file: &SyntaxFile<'_>, build: &mut ShapeBuild<'_>) {
    clear_keys(build.index);
    build.index.classes.resize(file.tree.nodes.len(), None);
    for (id, node) in file.tree.nodes.iter().enumerate().rev() {
        let at = table_id(file.tree.segments, Position(u32::try_from(id).unwrap_or(u32::MAX)));
        let bits = node_table::kind_of(file.table, at);
        let kind = kind_of(bits);
        let whole = kind != Kind::Extra(Extra::IpyEscapeCommand)
            && node_table::children_of(file.table, at)
                .iter()
                .all(|child| place_of_id(file.tree, *child).is_none_or(|place| class_of(build, place).is_some()));
        if !whole {
            continue;
        }
        build.index.scratch.clear();
        let _room = push_key_word(build.index, ShapeWord::Kind(bits));
        payload(build, *node);
        if valued(build, *node) == ShapeReading::ByChildren {
            children(file, build, at, kind);
        }
        let class = intern_key(build.index, at);
        if let Some(slot) = build.index.classes.get_mut(id) {
            *slot = Some(class);
        }
    }
}
