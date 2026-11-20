//! The `expression_fields` service: the field each child of an expression, a string or a pattern fills, read from the typed node
//! (the fields of `Python.asdl`, by structure), in the order the children stand in the file.

use crate::domain::labels::types::Field;
use crate::domain::node::types::ChildField;
use ruff_python_ast::{AnyNodeRef, Expr, FStringPartRef};

/// Adds a child and the field it fills.
pub(crate) fn put<'t>(
    out: &mut Vec<ChildField<'t>>,
    field: Field,
    child: impl Into<AnyNodeRef<'t>>,
) {
    out.push(ChildField {
        child: child.into(),
        field,
    });
}

/// Adds every expression of a list under one field.
pub(crate) fn each<'t>(out: &mut Vec<ChildField<'t>>, field: Field, expressions: &'t [Expr]) {
    for expression in expressions {
        put(out, field, expression);
    }
}

/// Adds an expression that may be absent.
pub(crate) fn maybe<'t>(out: &mut Vec<ChildField<'t>>, field: Field, expression: Option<&'t Expr>) {
    if let Some(present) = expression {
        put(out, field, present);
    }
}

/// Adds the children of an expression, a string or a pattern to `out`, each with the field it fills.
pub(crate) fn expression_child_fields<'t>(node: AnyNodeRef<'t>, out: &mut Vec<ChildField<'t>>) {
    expression_fields(node, out);
    string_fields(node, out);
    pattern_fields(node, out);
}

/// The fields of an expression that is no string.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only expressions are read here; every other kind is read by another function of the service"
)]
#[expect(
    clippy::too_many_lines,
    reason = "one arm for each expression of the standard that holds a child: the length is the grammar's"
)]
fn expression_fields<'t>(node: AnyNodeRef<'t>, out: &mut Vec<ChildField<'t>>) {
    match node {
        AnyNodeRef::ExprBoolOp(expression) => each(out, Field::Values, &expression.values),
        AnyNodeRef::ExprNamed(expression) => {
            put(out, Field::Target, expression.target.as_ref());
            put(out, Field::Value, expression.value.as_ref());
        }
        AnyNodeRef::ExprBinOp(expression) => {
            put(out, Field::Left, expression.left.as_ref());
            put(out, Field::Right, expression.right.as_ref());
        }
        AnyNodeRef::ExprUnaryOp(expression) => {
            put(out, Field::Operand, expression.operand.as_ref())
        }
        AnyNodeRef::ExprIf(expression) => {
            put(out, Field::Body, expression.body.as_ref());
            put(out, Field::Test, expression.test.as_ref());
            put(out, Field::Orelse, expression.orelse.as_ref());
        }
        AnyNodeRef::ExprDict(expression) => {
            for item in &expression.items {
                maybe(out, Field::Keys, item.key.as_ref());
                put(out, Field::Values, &item.value);
            }
        }
        AnyNodeRef::ExprSet(expression) => each(out, Field::Elts, &expression.elts),
        AnyNodeRef::ExprList(expression) => each(out, Field::Elts, &expression.elts),
        AnyNodeRef::ExprTuple(expression) => each(out, Field::Elts, &expression.elts),
        AnyNodeRef::ExprListComp(expression) => {
            put(out, Field::Elt, expression.elt.as_ref());
            for generator in &expression.generators {
                put(out, Field::Generators, generator);
            }
        }
        AnyNodeRef::ExprSetComp(expression) => {
            put(out, Field::Elt, expression.elt.as_ref());
            for generator in &expression.generators {
                put(out, Field::Generators, generator);
            }
        }
        AnyNodeRef::ExprGenerator(expression) => {
            put(out, Field::Elt, expression.elt.as_ref());
            for generator in &expression.generators {
                put(out, Field::Generators, generator);
            }
        }
        AnyNodeRef::ExprDictComp(expression) => {
            // An unpacking comprehension, `{**each for each in all}`, has one expression: the standard holds it as the key.
            match expression.key.as_deref() {
                Some(key) => {
                    put(out, Field::Key, key);
                    put(out, Field::Value, expression.value.as_ref());
                }
                None => put(out, Field::Key, expression.value.as_ref()),
            }
            for generator in &expression.generators {
                put(out, Field::Generators, generator);
            }
        }
        AnyNodeRef::Comprehension(clause) => {
            put(out, Field::Target, &clause.target);
            put(out, Field::Iter, &clause.iter);
            each(out, Field::Ifs, &clause.ifs);
        }
        AnyNodeRef::ExprAwait(expression) => put(out, Field::Value, expression.value.as_ref()),
        AnyNodeRef::ExprYield(expression) => maybe(out, Field::Value, expression.value.as_deref()),
        AnyNodeRef::ExprYieldFrom(expression) => put(out, Field::Value, expression.value.as_ref()),
        AnyNodeRef::ExprCompare(expression) => {
            if let Some((first, rest)) = expression.operands.split_first() {
                put(out, Field::Left, first);
                each(out, Field::Comparators, rest);
            }
        }
        AnyNodeRef::ExprCall(expression) => put(out, Field::Func, expression.func.as_ref()),
        AnyNodeRef::ExprAttribute(expression) => {
            put(out, Field::Value, expression.value.as_ref());
            put(out, Field::Attr, &expression.attr);
        }
        AnyNodeRef::ExprSubscript(expression) => {
            put(out, Field::Value, expression.value.as_ref());
            put(out, Field::Slice, expression.slice.as_ref());
        }
        AnyNodeRef::ExprStarred(expression) => put(out, Field::Value, expression.value.as_ref()),
        AnyNodeRef::ExprSlice(expression) => {
            maybe(out, Field::Lower, expression.lower.as_deref());
            maybe(out, Field::Upper, expression.upper.as_deref());
            maybe(out, Field::Step, expression.step.as_deref());
        }
        AnyNodeRef::Keyword(keyword) => {
            if let Some(name) = &keyword.arg {
                put(out, Field::Arg, name);
            }
            put(out, Field::Value, &keyword.value);
        }
        _ => {}
    }
}

/// The fields of a string, an f-string, a t-string and their parts.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only strings and their parts are read here; every other kind is read by another function of the service"
)]
fn string_fields<'t>(node: AnyNodeRef<'t>, out: &mut Vec<ChildField<'t>>) {
    match node {
        AnyNodeRef::ExprStringLiteral(literal) => {
            for part in &literal.value {
                put(out, Field::Value, part);
            }
        }
        AnyNodeRef::ExprBytesLiteral(literal) => {
            for part in &literal.value {
                put(out, Field::Value, part);
            }
        }
        AnyNodeRef::ExprFString(string) => {
            for part in &string.value {
                match part {
                    FStringPartRef::Literal(literal) => put(out, Field::Values, literal),
                    FStringPartRef::FString(formatted) => put(out, Field::None, formatted),
                }
            }
        }
        AnyNodeRef::FString(string) => {
            for element in &string.elements {
                put(out, Field::Values, element);
            }
        }
        AnyNodeRef::TString(string) => {
            for element in &string.elements {
                put(out, Field::Values, element);
            }
        }
        AnyNodeRef::InterpolatedStringFormatSpec(spec) => {
            for element in &spec.elements {
                put(out, Field::Values, element);
            }
        }
        AnyNodeRef::InterpolatedElement(element) => {
            put(out, Field::Value, element.expression.as_ref());
            if let Some(spec) = &element.format_spec {
                put(out, Field::FormatSpec, spec.as_ref());
            }
        }
        _ => {}
    }
}

/// The fields of a pattern of a `match` and its parts.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only patterns are read here; every other kind is read by another function of the service"
)]
fn pattern_fields<'t>(node: AnyNodeRef<'t>, out: &mut Vec<ChildField<'t>>) {
    match node {
        AnyNodeRef::PatternMatchValue(pattern) => put(out, Field::Value, pattern.value.as_ref()),
        AnyNodeRef::PatternMatchClass(pattern) => {
            put(out, Field::Cls, pattern.cls.as_ref());
            put(out, Field::None, &pattern.arguments);
        }
        AnyNodeRef::PatternArguments(arguments) => {
            for pattern in &arguments.patterns {
                put(out, Field::Patterns, pattern);
            }
        }
        AnyNodeRef::PatternKeyword(keyword) => {
            put(out, Field::KwdAttrs, &keyword.attr);
            put(out, Field::KwdPatterns, &keyword.pattern);
        }
        AnyNodeRef::PatternMatchOr(pattern) => {
            for alternative in &pattern.patterns {
                put(out, Field::Patterns, alternative);
            }
        }
        AnyNodeRef::PatternMatchSequence(pattern) => {
            for element in &pattern.patterns {
                put(out, Field::Patterns, element);
            }
        }
        AnyNodeRef::PatternMatchMapping(pattern) => {
            each(out, Field::Keys, &pattern.keys);
            for value in &pattern.patterns {
                put(out, Field::Patterns, value);
            }
            if let Some(rest) = &pattern.rest {
                put(out, Field::Rest, rest);
            }
        }
        AnyNodeRef::PatternMatchAs(pattern) => {
            if let Some(inner) = &pattern.pattern {
                put(out, Field::Pattern, inner.as_ref());
            }
            if let Some(name) = &pattern.name {
                put(out, Field::Name, name);
            }
        }
        AnyNodeRef::PatternMatchStar(pattern) => {
            if let Some(name) = &pattern.name {
                put(out, Field::Name, name);
            }
        }
        _ => {}
    }
}
