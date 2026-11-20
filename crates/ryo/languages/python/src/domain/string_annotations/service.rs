//! The `string_annotations` service: free functions over the parser's own nodes. A string literal is a type expression where
//! the language evaluates it as one: in an annotation, as a type operand of a generic class or of `Annotated`, as a type
//! argument of `cast`, `NewType` or `TypeVar`, or as the value of a `TypeAlias`; never as an operand of `Literal`, as metadata of
//! `Annotated`, in a slice, or as an argument of any other call. Which form a name is comes from the model's static resolution;
//! a name no import of the file brings from a module with such forms is none, and is not resolved at all.

use super::constants::{GENERIC_BUILTINS, GENERIC_MODULES, TYPING_MODULES};
use super::types::{FormImports, FormNaming, FormSource, FormUse};
use crate::domain::lexical_facts::service::qualify;
use crate::domain::lexical_facts::types::NameProvenance;
use crate::domain::model::lexical::service::{id_of, node_at, parent_of};
use crate::domain::model::lexical::types::TypedTree;
use crate::domain::model::view::service::built;
use crate::domain::model::view::types::ModelPart;
use crate::domain::node::types::{AnnotationSeed, StringSite};
use crate::domain::nodes::types::Category;
use crate::domain::semantic_syntax::constants::{BOUND, DEFAULT, NEW_TYPE_KEYWORD, TYPE_KEYWORD};
use crate::domain::semantic_syntax::service::{operand_place, typing_form};
use crate::domain::semantic_syntax::types::{TypeContext, TypingForm};
use ruff_python_ast::{AnyNodeRef, ArgOrKeyword, Arguments, Expr, ExprSubscript};
use ryo::languages::{NameText, Position};

/// The module an import statement's alias brings its name from, by its first part; none for a relative import, whose names
/// are the project's own.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "an alias stands in an import statement of one of two kinds; under any other node it brings no module"
)]
fn imported_module<'a>(tree: TypedTree<'_, 'a>, alias: Position) -> Option<NameText<'a>> {
    let AnyNodeRef::Alias(typed) = node_at(tree, alias)? else {
        return None;
    };
    let module = match node_at(tree, parent_of(tree, alias))? {
        AnyNodeRef::StmtImportFrom(from) if from.level == 0 => from.module.as_ref()?.as_str(),
        AnyNodeRef::StmtImport(_) => typed.name.as_str(),
        _ => return None,
    };
    module.split('.').next().map(NameText)
}

/// Writes into `imports` the aliases of `aliases` that bring a name of a module with typing forms.
pub(crate) fn form_imports(
    tree: TypedTree<'_, '_>,
    aliases: &[Position],
    imports: &mut FormImports,
) {
    imports.typing.clear();
    imports.generic.clear();
    for alias in aliases {
        let Some(module) = imported_module(tree, *alias) else {
            continue;
        };
        if TYPING_MODULES.contains(&module.0) {
            imports.typing.push(*alias);
        } else if GENERIC_MODULES.contains(&module.0) {
            imports.generic.push(*alias);
        } else {
            // A module with no typing form: its names are none.
        }
    }
}

/// The name an alias binds: its `as` name, or the first part of what it imports.
fn bound_name<'a>(tree: TypedTree<'_, 'a>, alias: Position) -> Option<NameText<'a>> {
    let AnyNodeRef::Alias(typed) = node_at(tree, alias)? else {
        return None;
    };
    match &typed.asname {
        Some(renamed) => Some(NameText(renamed.as_str())),
        None => typed.name.as_str().split('.').next().map(NameText),
    }
}

/// The name a name or a chain of attributes starts from; none for any other expression.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only a name and an attribute write a dotted name; every other expression starts from none"
)]
fn base_name<'a>(tree: TypedTree<'_, 'a>, expression: Position) -> Option<NameText<'a>> {
    let mut at = expression;
    loop {
        match node_at(tree, at)? {
            AnyNodeRef::ExprName(name) => return Some(NameText(name.id.as_str())),
            AnyNodeRef::ExprAttribute(attribute) => at = id_of(&*attribute.value),
            _ => return None,
        }
    }
}

/// Whether the name `base` can name a typing form `asked` for: an import of the file brings it from a module with such forms,
/// or it is a builtin generic class. A name that cannot is no form whatever it resolves to, so it is never resolved.
fn may_name(
    tree: TypedTree<'_, '_>,
    imports: &FormImports,
    base: NameText<'_>,
    asked: FormUse,
) -> FormNaming {
    let brought = |aliases: &[Position]| {
        aliases
            .iter()
            .any(|alias| bound_name(tree, *alias) == Some(base))
    };
    let possible = match asked {
        FormUse::Called | FormUse::Annotating => brought(&imports.typing),
        FormUse::Subscripted => {
            brought(&imports.typing)
                || brought(&imports.generic)
                || GENERIC_BUILTINS.contains(&base.0)
        }
    };
    if possible {
        FormNaming::Possible
    } else {
        FormNaming::Impossible
    }
}

/// The typing form the expression at `expression` names, for what `asked`.
fn form_of(
    source: &mut FormSource<'_, '_, '_>,
    expression: Position,
    asked: FormUse,
) -> TypingForm {
    let tree = source.tree;
    let Some(base) = base_name(tree, expression) else {
        return TypingForm::Other;
    };
    if may_name(tree, source.imports, base, asked) == FormNaming::Impossible {
        return TypingForm::Other;
    }
    built(source.model, source.built, ModelPart::Resolution, tree);
    match qualify(source.model, expression, source.path) {
        NameProvenance::Import | NameProvenance::Builtin(_) => {
            typing_form(NameText(&source.path.0))
        }
        NameProvenance::Local | NameProvenance::Unresolved | NameProvenance::Ambiguous => {
            TypingForm::Other
        }
    }
}

/// Whether `host`, under the subscription of `Annotated`, is its first operand: the type; the rest is metadata.
fn annotated_operand(
    tree: TypedTree<'_, '_>,
    subscription: &ExprSubscript,
    at: Position,
    host: Position,
) -> TypeContext {
    let Expr::Tuple(operands) = &*subscription.slice else {
        return TypeContext::Type;
    };
    let Some(first) = operands.elts.first() else {
        return TypeContext::Value;
    };
    let slice = id_of(&*subscription.slice);
    let mut owner = host;
    while parent_of(tree, owner) != slice && owner != at {
        owner = parent_of(tree, owner);
    }
    if owner == id_of(first) {
        TypeContext::Type
    } else {
        TypeContext::Value
    }
}

/// Whether `argument`, among the arguments of a call of `form`, is one the form reads as a type.
fn type_argument(
    tree: TypedTree<'_, '_>,
    arguments: &Arguments,
    argument: Position,
    form: TypingForm,
) -> TypeContext {
    if let Some(AnyNodeRef::Keyword(keyword)) = node_at(tree, argument) {
        let name = keyword
            .arg
            .as_ref()
            .map(ruff_python_ast::Identifier::as_str);
        return if matches!(
            (form, name),
            (TypingForm::Cast, Some(TYPE_KEYWORD))
                | (TypingForm::NewType, Some(NEW_TYPE_KEYWORD))
                | (TypingForm::TypeVar, Some(BOUND | DEFAULT))
        ) {
            TypeContext::Type
        } else {
            TypeContext::Value
        };
    }
    let operands = arguments.iter_source_order().map(|each| match each {
        ArgOrKeyword::Arg(expression) => id_of(expression),
        ArgOrKeyword::Keyword(keyword) => id_of(keyword),
    });
    operand_place(operands, argument, form, |at| {
        if matches!(node_at(tree, at), Some(AnyNodeRef::Keyword(_))) {
            Category::Part
        } else {
            Category::Expression
        }
    })
}

/// The callee the arguments at `arguments` are handed to: a call's function; for the arguments of a class definition, its
/// first decorator, as the first expression the definition writes.
fn callee(tree: TypedTree<'_, '_>, arguments: Position) -> Option<Position> {
    match node_at(tree, parent_of(tree, arguments))? {
        AnyNodeRef::ExprCall(call) => Some(id_of(&*call.func)),
        AnyNodeRef::StmtClassDef(class) => class
            .decorator_list
            .first()
            .map(|decorator| id_of(&decorator.expression)),
        _ => None,
    }
}

/// Whether the string at `site` stands where the language reads a type expression.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only a slice, a subscription, the arguments of a call and an annotated assignment change what a string below them is"
)]
pub(crate) fn type_position(source: &mut FormSource<'_, '_, '_>, site: StringSite) -> TypeContext {
    let tree = source.tree;
    let mut allowed = site.seed == AnnotationSeed::Annotation;
    let mut established = false;
    let mut below = site.at;
    loop {
        // The ancestors through the string's statement, and no further than the expression of the string that holds it.
        let above = parent_of(tree, below);
        let root = tree
            .segments
            .binary_search_by_key(&below, |segment| segment.first)
            .is_ok();
        if above == below || root || node_at(tree, below).is_some_and(AnyNodeRef::is_statement) {
            break;
        }
        match node_at(tree, above) {
            Some(AnyNodeRef::ExprSlice(_)) => return TypeContext::Value,
            Some(AnyNodeRef::ExprSubscript(subscription)) => {
                let form = form_of(source, id_of(&*subscription.value), FormUse::Subscripted);
                if form == TypingForm::Literal {
                    return TypeContext::Value;
                }
                if form == TypingForm::Annotated && !established {
                    if annotated_operand(tree, subscription, above, site.at) == TypeContext::Value {
                        return TypeContext::Value;
                    }
                    allowed = true;
                    established = true;
                }
                if form == TypingForm::Generic
                    && !established
                    && id_of(&*subscription.slice) == below
                {
                    allowed = true;
                    established = true;
                }
            }
            Some(AnyNodeRef::Arguments(arguments)) if !established => {
                let Some(function) = callee(tree, above) else {
                    return TypeContext::Value;
                };
                let form = form_of(source, function, FormUse::Called);
                if type_argument(tree, arguments, below, form) == TypeContext::Value {
                    return TypeContext::Value;
                }
                allowed = true;
                established = true;
            }
            Some(AnyNodeRef::StmtAnnAssign(assignment)) if !allowed => {
                let annotation = id_of(&*assignment.annotation);
                if annotation != below
                    && form_of(source, annotation, FormUse::Annotating) == TypingForm::TypeAlias
                {
                    allowed = true;
                    established = true;
                }
            }
            _ => {}
        }
        below = above;
    }
    if allowed {
        TypeContext::Type
    } else {
        TypeContext::Value
    }
}

/// Writes into `hosts` the places of the strings among `sites` that the language reads as type expressions.
pub(crate) fn type_strings(
    source: &mut FormSource<'_, '_, '_>,
    sites: &[StringSite],
    hosts: &mut Vec<Position>,
) {
    hosts.clear();
    for site in sites {
        if type_position(source, *site) == TypeContext::Type {
            hosts.push(site.at);
        }
    }
}
