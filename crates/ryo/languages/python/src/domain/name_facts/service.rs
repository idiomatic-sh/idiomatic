//! The `name_facts` service: free functions over the names the parser's nodes write. The dotted name of an expression, what an import binds and names, what a decorator and a class base name, the name an argument hands on, and
//! how a parameter takes its argument.

use super::constants::EVERYTHING;
use super::types::{Decoration, ImportedName};
use crate::domain::definitions::constants::{
    KEYWORD_ONLY, POSITIONAL_ONLY, POSITIONAL_OR_KEYWORD, VAR_KEYWORD, VAR_POSITIONAL,
};
use crate::domain::definitions::types::{ParameterPlace, Refinement};
use crate::domain::literal_facts::types::TextBuffer;
use crate::domain::names::constants::DOT;
use crate::domain::names::types::{Dottedness, Unresolved};
use crate::domain::node::service::{id_of, node_at, text_between};
use crate::domain::node::types::SyntaxFile;
use ruff_python_ast::{AnyNodeRef, Expr, Parameter};
use ruff_text_size::Ranged as _;
use ryo::languages::{Integer, NameText, Offset, ParameterKind, Position, node_table};

/// Writes the dotted name a name or an attribute chain writes after what `dotted_name` holds; says whether the expression is one.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only a name and an attribute of a dotted name write a dotted name; any other expression writes none"
)]
pub(crate) fn dotted(expression: &Expr, dotted_name: &mut TextBuffer) -> Dottedness {
    match expression {
        Expr::Name(name) => {
            dotted_name.0.push_str(name.id.as_str());
            Dottedness::Dotted
        }
        Expr::Attribute(attribute) => {
            if dotted(&attribute.value, dotted_name) == Dottedness::Other {
                return Dottedness::Other;
            }
            dotted_name.0.push_str(DOT);
            dotted_name.0.push_str(attribute.attr.as_str());
            Dottedness::Dotted
        }
        _ => Dottedness::Other,
    }
}

/// What the alias at `id` binds and imports, written into `names`: `import a.b` binds `a` and imports `a.b`; `from .m import n as
/// k` binds `k` and imports `.m.n`; `from m import *` binds names no file states and imports `m`. None for any other node.
pub(crate) fn imported<'b>(
    file: &SyntaxFile<'_>,
    id: Position,
    names: &'b mut TextBuffer,
) -> Option<ImportedName<'b>> {
    let AnyNodeRef::Alias(alias) = node_at(file, id)? else {
        return None;
    };
    let from = match node_at(file, node_table::parent_of(file.table, id)) {
        Some(AnyNodeRef::StmtImportFrom(statement)) => Some(statement),
        Some(_) | None => None,
    };
    let wildcard = alias.name.as_str() == EVERYTHING;
    names.0.clear();
    if wildcard {
        // `*` binds whatever the module exports: no name is written.
    } else if let Some(asname) = &alias.asname {
        names.0.push_str(asname.as_str());
    } else if from.is_some() {
        names.0.push_str(alias.name.as_str());
    } else {
        names
            .0
            .push_str(alias.name.as_str().split('.').next().unwrap_or_default());
    }
    let split = names.0.len();
    if let Some(statement) = from {
        for _ in 0..statement.level {
            names.0.push_str(DOT);
        }
        if let Some(module) = &statement.module {
            names.0.push_str(module.as_str());
            if !wildcard {
                names.0.push_str(DOT);
            }
        }
    }
    if !wildcard {
        names.0.push_str(alias.name.as_str());
    }
    let (binding, target) = names.0.split_at_checked(split)?;
    Some(ImportedName {
        binding: (!wildcard).then_some(NameText(binding)),
        target: NameText(target),
    })
}

/// What the decorator at `id` names and is called with, written into `names`: the dotted name it writes or calls, and the text
/// between the parentheses of its call. None for any other node.
pub(crate) fn decoration<'b>(
    file: &SyntaxFile<'_>,
    id: Position,
    names: &'b mut TextBuffer,
) -> Option<Decoration<'b>> {
    let AnyNodeRef::Decorator(decorator) = node_at(file, id)? else {
        return None;
    };
    names.0.clear();
    let called = if let Expr::Call(call) = &decorator.expression {
        Some(call)
    } else {
        None
    };
    let target = called.map_or(&decorator.expression, |call| call.func.as_ref());
    let known = dotted(target, names) == Dottedness::Dotted;
    if !known {
        names.0.clear();
    }
    let split = names.0.len();
    if let Some(call) = called {
        // The argument list spans its parentheses: the arguments are what is between them.
        let open = Offset(call.arguments.start().to_u32().saturating_add(1));
        let close = Offset(call.arguments.end().to_u32().saturating_sub(1));
        if close >= open {
            names.0.push_str(text_between(file, open, close).0);
        }
    }
    let (name, arguments) = names.0.split_at_checked(split)?;
    Some(Decoration {
        target: known.then_some(NameText(name)),
        arguments: NameText(arguments),
    })
}

/// Why a class base names no class a file states, or none when `dotted_name` now holds the dotted name it writes: a starred base
/// and a base subscripted by a number name none; a subscripted base names what it subscripts.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "a base that is not starred or subscripted is read as a dotted name, whatever its kind"
)]
pub(crate) fn base(expression: &Expr, dotted_name: &mut TextBuffer) -> Option<Unresolved> {
    dotted_name.0.clear();
    let named = match expression {
        Expr::Starred(_) => return Some(Unresolved::StarBase),
        Expr::Subscript(subscript) if matches!(&*subscript.slice, Expr::NumberLiteral(_)) => {
            Dottedness::Other
        }
        Expr::Subscript(subscript) => dotted(&subscript.value, dotted_name),
        _ => dotted(expression, dotted_name),
    };
    match named {
        Dottedness::Dotted => None,
        Dottedness::Other => {
            dotted_name.0.clear();
            Some(Unresolved::DynamicBase)
        }
    }
}

/// The name an argument hands on: the one name it writes, behind any stars; none when it is anything else.
#[must_use]
pub(crate) fn passed(argument: &Expr) -> Option<NameText<'_>> {
    let mut current = argument;
    while let Expr::Starred(starred) = current {
        current = &starred.value;
    }
    if let Expr::Name(name) = current {
        Some(NameText(name.id.as_str()))
    } else {
        None
    }
}

/// The place of a parameter that takes its argument one way.
const fn place(
    kind: ParameterKind,
    word: Refinement,
    ordinal: Integer,
    callable: Position,
) -> ParameterPlace {
    ParameterPlace {
        kind,
        word,
        ordinal,
        callable,
    }
}

/// How the parameter at `id` takes its argument, its place among the parameters of its callable, and the callable: none for a node
/// that is no parameter of a parameter list.
#[must_use]
pub(crate) fn parameter_place(file: &SyntaxFile<'_>, id: Position) -> Option<ParameterPlace> {
    let mut holder = node_table::parent_of(file.table, id);
    if matches!(
        node_at(file, holder),
        Some(AnyNodeRef::ParameterWithDefault(_))
    ) {
        holder = node_table::parent_of(file.table, holder);
    }
    let AnyNodeRef::Parameters(parameters) = node_at(file, holder)? else {
        return None;
    };
    let callable = node_table::parent_of(file.table, holder);
    let is = |parameter: &Parameter| id_of(parameter) == Some(id);
    let mut ordinal = Integer(0);
    for each in &parameters.posonlyargs {
        if is(&each.parameter) {
            return Some(place(
                ParameterKind::Positional,
                Refinement(POSITIONAL_ONLY),
                ordinal,
                callable,
            ));
        }
        ordinal.0 += 1;
    }
    for each in &parameters.args {
        if is(&each.parameter) {
            let kind = ParameterKind::PositionalOrKeyword;
            return Some(place(
                kind,
                Refinement(POSITIONAL_OR_KEYWORD),
                ordinal,
                callable,
            ));
        }
        ordinal.0 += 1;
    }
    if let Some(each) = &parameters.vararg {
        if is(each) {
            let kind = ParameterKind::VariadicPositional;
            return Some(place(kind, Refinement(VAR_POSITIONAL), ordinal, callable));
        }
        ordinal.0 += 1;
    }
    for each in &parameters.kwonlyargs {
        if is(&each.parameter) {
            return Some(place(
                ParameterKind::KeywordOnly,
                Refinement(KEYWORD_ONLY),
                ordinal,
                callable,
            ));
        }
        ordinal.0 += 1;
    }
    parameters
        .kwarg
        .as_deref()
        .filter(|each| is(each))
        .map(|_| {
            place(
                ParameterKind::VariadicKeyword,
                Refinement(VAR_KEYWORD),
                ordinal,
                callable,
            )
        })
}

/// The place of the decorator at `id` among the decorators of its definition, from zero.
#[must_use]
pub(crate) fn decorator_ordinal(file: &SyntaxFile<'_>, id: Position) -> Integer {
    let decorators = match node_at(file, node_table::parent_of(file.table, id)) {
        Some(AnyNodeRef::StmtFunctionDef(function)) => function.decorator_list.as_slice(),
        Some(AnyNodeRef::StmtClassDef(class)) => class.decorator_list.as_slice(),
        Some(_) | None => &[],
    };
    let ordinal = decorators
        .iter()
        .position(|each| id_of(each) == Some(id))
        .unwrap_or(0);
    Integer(i64::try_from(ordinal).unwrap_or(i64::MAX))
}
