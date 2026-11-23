//! Constant shapes depend on the decoded, typed value alone. Keep that value
//! directly while assigning class handles in the parser walk.
use ruff_python_ast::{AnyNodeRef, Number};
use ryo::languages::Position;
use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Scalar {
    Boolean(bool),
    None,
    Ellipsis,
    Integer(u64),
    LargeInteger(Box<str>),
    Float(u64),
    Complex(u64, u64),
}

#[derive(Default, Debug)]
pub(super) struct Classes {
    scalars: BTreeMap<Scalar, Position>,
    strings: BTreeMap<Box<[u8]>, Position>,
    bytes: BTreeMap<Box<[u8]>, Position>,
    joined: Vec<u8>,
}

fn class_of_bytes(map: &mut BTreeMap<Box<[u8]>, Position>, value: &[u8], next: Position) -> Position {
    if let Some(&class) = map.get(value) { return class; }
    map.insert(value.into(), next);
    next
}

impl Classes {
    /// The parser schema maps exactly these AST variants onto Constant. The
    /// representation preserves the scalar kind, float bits and concatenated
    /// payload compared by the complete structural key.
    pub(super) fn class(&mut self, node: AnyNodeRef<'_>, next: Position) -> Position {
        let scalar = match node {
            AnyNodeRef::ExprStringLiteral(literal) => {
                return class_of_bytes(&mut self.strings, literal.value.to_str().as_bytes(), next);
            }
            AnyNodeRef::InterpolatedStringLiteralElement(literal) => {
                return class_of_bytes(&mut self.strings, literal.value.as_bytes(), next);
            }
            AnyNodeRef::ExprBytesLiteral(literal) => {
                let bytes = if let [part] = literal.value.as_slice() {
                    part.value.as_ref()
                } else {
                    self.joined.clear();
                    self.joined.extend(literal.value.bytes());
                    &self.joined
                };
                return class_of_bytes(&mut self.bytes, bytes, next);
            }
            AnyNodeRef::ExprBooleanLiteral(literal) => Scalar::Boolean(literal.value),
            AnyNodeRef::ExprNoneLiteral(_) => Scalar::None,
            AnyNodeRef::ExprEllipsisLiteral(_) => Scalar::Ellipsis,
            AnyNodeRef::ExprNumberLiteral(literal) => match &literal.value {
                Number::Int(i) => i.as_u64().map_or_else(
                    || Scalar::LargeInteger(i.to_string().into_boxed_str()),
                    Scalar::Integer,
                ),
                Number::Float(v) => Scalar::Float(v.to_bits()),
                Number::Complex { real, imag } => Scalar::Complex(real.to_bits(), imag.to_bits()),
            },
            _ => unreachable!("the parser's Constant kind must carry a scalar value"),
        };
        *self.scalars.entry(scalar).or_insert(next)
    }

    pub(super) fn clear(&mut self) {
        self.scalars.clear();
        self.strings.clear();
        self.bytes.clear();
        self.joined.clear();
    }
}
