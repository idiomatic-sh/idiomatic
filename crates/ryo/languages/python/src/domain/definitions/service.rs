//! The `definitions` service: free functions named by verb over the domain's types, with no IO and no ports. What an element
//! defines, as the `defines` and `parameter` relations say it: its shared kind, Python's word for it, its name and its scope.

use ryo::languages::{self, ParameterKind, Position, Symbol, Tree};

use super::constants::{
    ASYNC_FUNCTION, ASYNC_METHOD, CLASS, EXCEPTION, FROM_IMPORT, FUNCTION, IMPORT, KEYWORD_ONLY,
    METHOD, PARAM_SPEC, PATTERN, POSITIONAL_ONLY, POSITIONAL_OR_KEYWORD, TYPE_ALIAS, TYPE_VAR,
    TYPE_VAR_TUPLE, VAR_KEYWORD, VAR_POSITIONAL, VARIABLE,
};
use super::types::{Defined, Definition, ParameterPlace, Refinement};
use crate::domain::extras::types::Extra;
use crate::domain::imports::service::{binding_at, import_binding, imported};
use crate::domain::model::types::Declarations;
use crate::domain::names::service::unknown;
use crate::domain::names::types::{Imported, Unresolved};
use crate::domain::nodes::types::{Category, Node};
use crate::domain::scopes::service::{binding_scope, context, enclosing, place_among};
use crate::domain::scopes::types::{CategoryIndex, Context};
use crate::domain::syntax::service::{child, kind, name_of, parent, written};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;

/// What a definition is: its shared kind and Python's word for it.
const fn defined(kind: languages::DeclarationKind, word: Refinement) -> Defined {
    Defined { kind, word }
}

/// A word of the language as a name of the file.
pub(crate) fn word(tree: &Tree<'_>, symbols: &mut languages::Symbols, text: Refinement) -> Symbol {
    languages::intern_derived(symbols, tree.source, languages::NameText(text.0))
}

/// What a function definition is: a function or a method, as Python says it, `async` or not.
#[must_use]
pub(crate) fn function_word(tree: &Tree<'_>, function: Position) -> Defined {
    let method = kind(tree, enclosing(tree, function)) == Kind::Node(Node::ClassDef);
    let coroutine = languages::children(tree.elements, function)
        .find(|each| kind(tree, *each) == Kind::Token(Token::Name))
        .is_some_and(|keyword| written(tree, keyword).0 == crate::domain::syntax::constants::ASYNC);
    match (method, coroutine) {
        (true, true) => defined(languages::DeclarationKind::Method, Refinement(ASYNC_METHOD)),
        (true, false) => defined(languages::DeclarationKind::Method, Refinement(METHOD)),
        (false, true) => defined(
            languages::DeclarationKind::Function,
            Refinement(ASYNC_FUNCTION),
        ),
        (false, false) => defined(languages::DeclarationKind::Function, Refinement(FUNCTION)),
    }
}

/// Where the parameter at `at` is in its callable's list: how it takes its argument, its place, and the callable.
#[must_use]
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "a parameter list holds parameters, the markers `/`, `*` and `**`, and punctuation; the punctuation says nothing of a parameter's place"
)]
pub(crate) fn placed(tree: &Tree<'_>, at: Position) -> Option<ParameterPlace> {
    let holder = parent(tree, at);
    let element = if kind(tree, holder) == Kind::Extra(Extra::ParameterWithDefault) {
        holder
    } else {
        at
    };
    let parameters = parent(tree, element);
    if kind(tree, parameters) != Kind::Node(Node::Arguments) {
        return None;
    }
    let slash = child(tree, parameters, Kind::Token(Token::Slash)).is_some();
    let (mut after_slash, mut after_star, mut double) = (false, false, false);
    let mut ordinal = 0_i64;
    for each in languages::children(tree.elements, parameters) {
        match kind(tree, each) {
            Kind::Token(Token::Slash) => after_slash = true,
            Kind::Token(Token::Star) => after_star = true,
            Kind::Node(Node::Arg) | Kind::Extra(Extra::ParameterWithDefault) => {
                // `*args` and `**kwargs` hold their star: a parameter alone, with no default, is one of them.
                let variadic = kind(tree, each) == Kind::Node(Node::Arg);
                if variadic {
                    double = child(tree, each, Kind::Token(Token::Doublestar)).is_some();
                }
                if each == element {
                    let (kind, word) = if variadic {
                        if double {
                            (ParameterKind::VariadicKeyword, VAR_KEYWORD)
                        } else {
                            (ParameterKind::VariadicPositional, VAR_POSITIONAL)
                        }
                    } else if slash && !after_slash {
                        (ParameterKind::Positional, POSITIONAL_ONLY)
                    } else if after_star {
                        (ParameterKind::KeywordOnly, KEYWORD_ONLY)
                    } else {
                        (ParameterKind::PositionalOrKeyword, POSITIONAL_OR_KEYWORD)
                    };
                    return Some(ParameterPlace {
                        kind,
                        word: Refinement(word),
                        ordinal: languages::Integer(ordinal),
                        callable: parent(tree, parameters),
                    });
                }
                after_star |= variadic;
                ordinal += 1;
            }
            _ => {}
        }
    }
    None
}

/// A definition named by its identifier child, bound in the scope `binding_scope` says.
fn named_by_identifier(
    tree: &Tree<'_>,
    model: &Declarations,
    at: Position,
    defined: Defined,
) -> Option<Definition> {
    let identifier = child(tree, at, Kind::Extra(Extra::Identifier))?;
    let name = name_of(tree, identifier)?;
    Some(Definition {
        defined,
        name,
        owner: binding_scope(tree, model, at, name),
        named: identifier,
    })
}

/// What the element at `at` defines, when it defines a name: a function, a class, an imported name, an assigned name, a parameter,
/// an exception's name, a pattern's capture or a type parameter.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "the kinds that define a name are the readers' `kinds:`; any other kind defines none"
)]
pub(crate) fn definition(
    tree: &Tree<'_>,
    model: &Declarations,
    at: Position,
) -> Option<Definition> {
    use languages::DeclarationKind as Declared;
    match kind(tree, at) {
        Kind::Node(Node::FunctionDef | Node::AsyncFunctionDef) => {
            named_by_identifier(tree, model, at, function_word(tree, at))
        }
        Kind::Node(Node::ClassDef) => {
            named_by_identifier(tree, model, at, defined(Declared::Class, Refinement(CLASS)))
        }
        Kind::Node(Node::ExceptHandler) => named_by_identifier(
            tree,
            model,
            at,
            defined(Declared::Variable, Refinement(EXCEPTION)),
        ),
        Kind::Node(Node::MatchAs | Node::MatchStar | Node::MatchMapping) => named_by_identifier(
            tree,
            model,
            at,
            defined(Declared::Variable, Refinement(PATTERN)),
        ),
        Kind::Node(Node::Alias) => {
            let word = if kind(tree, parent(tree, at)) == Kind::Node(Node::ImportFrom) {
                FROM_IMPORT
            } else {
                IMPORT
            };
            let name = if imported(tree, at) == Imported::Everything {
                unknown(Unresolved::WildcardImport)
            } else {
                import_binding(tree, at)?
            };
            Some(Definition {
                defined: defined(Declared::Import, Refinement(word)),
                name,
                owner: binding_scope(tree, model, at, name),
                named: binding_at(tree, at),
            })
        }
        Kind::Node(Node::Name) => {
            if context(tree, at) != Context::Store {
                return None;
            }
            let name = name_of(tree, at)?;
            let above = parent(tree, at);
            let alias = kind(tree, above) == Kind::Node(Node::TypeAlias)
                && place_among(tree, above, at, Category::Expression) == Some(CategoryIndex(0));
            let defined = if alias {
                defined(Declared::TypeAlias, Refinement(TYPE_ALIAS))
            } else {
                defined(Declared::Variable, Refinement(VARIABLE))
            };
            Some(Definition {
                defined,
                name,
                owner: binding_scope(tree, model, at, name),
                named: at,
            })
        }
        Kind::Node(Node::Arg) => {
            let placed = placed(tree, at)?;
            let identifier = child(tree, at, Kind::Extra(Extra::Identifier))?;
            Some(Definition {
                defined: Defined {
                    kind: Declared::Parameter,
                    word: placed.word,
                },
                name: name_of(tree, identifier)?,
                owner: placed.callable,
                named: identifier,
            })
        }
        Kind::Node(Node::TypeVar | Node::TypeVarTuple | Node::ParamSpec) => {
            let word = match kind(tree, at) {
                Kind::Node(Node::TypeVarTuple) => TYPE_VAR_TUPLE,
                Kind::Node(Node::ParamSpec) => PARAM_SPEC,
                _ => TYPE_VAR,
            };
            let identifier = child(tree, at, Kind::Extra(Extra::Identifier))?;
            Some(Definition {
                defined: defined(Declared::TypeParameter, Refinement(word)),
                name: name_of(tree, identifier)?,
                owner: parent(tree, parent(tree, at)),
                named: identifier,
            })
        }
        _ => None,
    }
}
