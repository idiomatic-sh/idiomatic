//! The `definitions` service: free functions named by verb over the domain's types, with no IO and no ports. What an element declares,
//! as the `defines` and `annotates` relations say it: its shared kind, Go's word for it, its name and its scope.

use ryo::languages::{self, DeclarationKind, Position, Symbol, Tree};

use super::constants::{
    ALIAS, CONSTANT, DEFINED_TYPE, EMBEDDED_FIELD, FIELD, FUNCTION, IMPORT, INTERFACE,
    INTERFACE_METHOD, LABEL, METHOD, RANGE_VARIABLE, SHORT_VARIABLE, STRUCT, TYPE_PARAMETER,
    TYPE_SWITCH_VARIABLE, VARIABLE,
};
use super::types::{Annotation, Assignment, Declaration, Defined, Meaning, NameSpelling, DeclarationShape};
use crate::domain::assignments::service::{left, operator, right};
use crate::domain::fields::service::{names as field_names, type_of};
use crate::domain::names::service::{
    blankness, embedded_name, import_binding, name_of, string_token,
};
use crate::domain::names::types::Blankness;
use crate::domain::nodes::types::GoNode;
use crate::domain::scopes::service::enclosing;
use crate::domain::signatures::constants::{PARAMETER, RECEIVER, RESULT, VARIADIC};
use crate::domain::signatures::service::{callable, listing, signature};
use crate::domain::signatures::types::{Listing, SignatureRefinement};
use crate::domain::specs::service::{
    declares, import_name, name as spec_name, names as spec_names, underlying, value_type,
};
use crate::domain::specs::types::{Declares, Underlying};
use crate::domain::syntax::service::{ancestors, kind, nodes, parent, token};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;

/// What a declaration is: its shared kind and Go's word for it.
const fn defined(kind: DeclarationKind, word: SignatureRefinement) -> Defined {
    Defined { kind, word }
}

/// A word of the language as a name of the file.
pub(crate) fn word(tree: &Tree<'_>, symbols: &mut languages::Symbols, text: SignatureRefinement) -> Symbol {
    languages::intern_derived(symbols, tree.source, languages::NameText(text.0))
}

/// What the element at `at` declares, by its kind.
#[must_use]
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only the kinds that declare names have a shape; every other kind declares nothing"
)]
pub(crate) fn shape(tree: &Tree<'_>, at: Position) -> DeclarationShape {
    match kind(tree, at) {
        Kind::Node(GoNode::FuncDecl) => DeclarationShape::Function,
        Kind::Node(GoNode::TypeSpec) => DeclarationShape::Type,
        Kind::Node(GoNode::ValueSpec) => DeclarationShape::Values,
        Kind::Node(GoNode::ImportSpec) => DeclarationShape::Import,
        Kind::Node(GoNode::Field) => DeclarationShape::Field,
        Kind::Node(GoNode::AssignStmt | GoNode::RangeStmt) => DeclarationShape::Variables,
        Kind::Node(GoNode::LabeledStmt) => DeclarationShape::Label,
        _ => DeclarationShape::Nothing,
    }
}

/// The type declaration, or the file, that holds the element at `at`: what a member of a type is owned by.
fn holder(tree: &Tree<'_>, at: Position) -> Position {
    ancestors(tree, at)
        .find(|each| kind(tree, *each) == Kind::Node(GoNode::TypeSpec))
        .unwrap_or_else(|| enclosing(tree, at))
}

/// The function or method declared at `at`; none for one that writes no name or is named `_`.
#[must_use]
pub(crate) fn function_declaration(tree: &Tree<'_>, at: Position) -> Option<Declaration> {
    let parts = signature(tree, at);
    let name = parts
        .name
        .filter(|each| blankness(tree, *each) == Blankness::Named)?;
    let (kind, word) = if parts.receiver.is_some() {
        (DeclarationKind::Method, METHOD)
    } else {
        (DeclarationKind::Function, FUNCTION)
    };
    Some(Declaration {
        defined: defined(kind, word),
        written: NameSpelling::Identifier,
        owner: enclosing(tree, at),
        named: name,
    })
}

/// The label the labeled statement at `at` declares: its scope is the body of the function it is in (the Go specification, "Label
/// scopes"); none for `_`.
#[must_use]
pub(crate) fn label_declaration(tree: &Tree<'_>, at: Position) -> Option<Declaration> {
    let name = nodes(tree, at)
        .next()
        .filter(|each| kind(tree, *each) == Kind::Node(GoNode::Ident))
        .filter(|each| blankness(tree, *each) == Blankness::Named)?;
    Some(Declaration {
        defined: defined(DeclarationKind::Label, LABEL),
        written: NameSpelling::Identifier,
        owner: enclosing(tree, at),
        named: name,
    })
}

/// The type declared at `at`.
#[must_use]
pub(crate) fn type_declaration(tree: &Tree<'_>, at: Position) -> Option<Declaration> {
    let name = spec_name(tree, at).filter(|each| blankness(tree, *each) == Blankness::Named)?;
    let (kind, word) = match underlying(tree, at) {
        Underlying::Structure => (DeclarationKind::Struct, STRUCT),
        Underlying::Interface => (DeclarationKind::Interface, INTERFACE),
        Underlying::Alias => (DeclarationKind::TypeAlias, ALIAS),
        Underlying::Other => (DeclarationKind::Type, DEFINED_TYPE),
    };
    Some(Declaration {
        defined: defined(kind, word),
        written: NameSpelling::Identifier,
        owner: enclosing(tree, at),
        named: name,
    })
}

/// The constants or variables declared at `at`, one for each name but `_`.
pub(crate) fn value_declarations<'a>(
    tree: &'a Tree<'_>,
    at: Position,
) -> impl Iterator<Item = Declaration> + 'a {
    let (kind, word) = match declares(tree, at) {
        Some(Declares::Constants) => (DeclarationKind::Constant, CONSTANT),
        Some(Declares::Variables | Declares::Types | Declares::Imports) | None => {
            (DeclarationKind::Variable, VARIABLE)
        }
    };
    let owner = enclosing(tree, at);
    spec_names(tree, at)
        .filter(|each| blankness(tree, *each) == Blankness::Named)
        .map(move |named| Declaration {
            defined: defined(kind, word),
            written: NameSpelling::Identifier,
            owner,
            named,
        })
}

/// The name the import at `at` brings in; none for one that binds `_`.
#[must_use]
pub(crate) fn import_declaration(tree: &Tree<'_>, at: Position) -> Option<Declaration> {
    if import_name(tree, at).is_some_and(|each| blankness(tree, each) == Blankness::Blank) {
        return None;
    }
    Some(Declaration {
        defined: defined(DeclarationKind::Import, IMPORT),
        written: NameSpelling::Import,
        owner: enclosing(tree, at),
        named: import_name(tree, at)
            .or_else(|| crate::domain::specs::service::import_path(tree, at))
            .unwrap_or(at),
    })
}

/// What the fields of the list that holds the field at `field` declare; none for a list that declares nothing.
fn meaning(tree: &Tree<'_>, field: Position) -> Option<Meaning> {
    let list = parent(tree, field);
    let listed = listing(tree, list)?;
    let function = || callable(tree, list).map(|each| each.callable);
    let variadic =
        || type_of(tree, field).is_some_and(|each| kind(tree, each) == Kind::Node(GoNode::Ellipsis));
    let (kind, word, owner) = match listed {
        Listing::Members => (DeclarationKind::Property, FIELD, holder(tree, field)),
        Listing::Methods => (
            DeclarationKind::Method,
            INTERFACE_METHOD,
            holder(tree, field),
        ),
        Listing::Receiver => (DeclarationKind::Parameter, RECEIVER, function()?),
        Listing::Parameters => (
            DeclarationKind::Parameter,
            if variadic() { VARIADIC } else { PARAMETER },
            function()?,
        ),
        Listing::Results => (DeclarationKind::Parameter, RESULT, function()?),
        Listing::TypeParameters => {
            let owner = parent(tree, list);
            let owner = if kind(tree, owner) == Kind::Node(GoNode::FuncType) {
                parent(tree, owner)
            } else {
                owner
            };
            (DeclarationKind::TypeParameter, TYPE_PARAMETER, owner)
        }
    };
    Some(Meaning { kind, word, owner })
}

/// The names the field at `at` declares: each of its names but `_`, and the type an embedded field of a struct is named by.
pub(crate) fn field_declarations<'a>(
    tree: &'a Tree<'_>,
    at: Position,
) -> impl Iterator<Item = Declaration> + 'a {
    let meant = meaning(tree, at);
    let identified = meant.into_iter().flat_map(move |meant| {
        field_names(tree, at)
            .filter(|each| blankness(tree, *each) == Blankness::Named)
            .map(move |named| Declaration {
                defined: defined(meant.kind, meant.word),
                written: NameSpelling::Identifier,
                owner: meant.owner,
                named,
            })
    });
    let embedded = meant
        .filter(|meant| meant.kind == DeclarationKind::Property)
        .filter(|_| field_names(tree, at).next().is_none())
        .and_then(|meant| {
            type_of(tree, at).map(|named| Declaration {
                defined: defined(DeclarationKind::Property, EMBEDDED_FIELD),
                written: NameSpelling::Embedded,
                owner: meant.owner,
                named,
            })
        });
    identified.chain(embedded)
}

/// The names a short variable declaration or a `range` clause at `at` declares, each but `_`.
pub(crate) fn variable_declarations<'a>(
    tree: &'a Tree<'_>,
    at: Position,
) -> impl Iterator<Item = Declaration> + 'a {
    let declares = operator(tree, at).is_some_and(|each| each.token == Token::Define);
    let word = if kind(tree, at) == Kind::Node(GoNode::RangeStmt) {
        RANGE_VARIABLE
    } else if kind(tree, parent(tree, at)) == Kind::Node(GoNode::TypeSwitchStmt) {
        TYPE_SWITCH_VARIABLE
    } else {
        SHORT_VARIABLE
    };
    let owner = enclosing(tree, at);
    left(tree, at)
        .filter(move |each| {
            declares
                && kind(tree, *each) == Kind::Node(GoNode::Ident)
                && blankness(tree, *each) == Blankness::Named
        })
        .map(move |named| Declaration {
            defined: defined(DeclarationKind::Variable, word),
            written: NameSpelling::Identifier,
            owner,
            named,
        })
}

/// The name a declaration writes.
pub(crate) fn declared_name(tree: &Tree<'_>, declaration: &Declaration) -> Option<Symbol> {
    match declaration.written {
        NameSpelling::Identifier => name_of(tree, declaration.named),
        NameSpelling::Embedded => embedded_name(tree, declaration.named),
        NameSpelling::Import => Some(import_binding(tree, parent(tree, declaration.named))),
    }
}

/// The types that annotate the names the element at `at`, a field or a constant or variable declaration, declares: one for each name,
/// and for a field with none, one for the field.
pub(crate) fn annotations<'a>(
    tree: &'a Tree<'_>,
    at: Position,
) -> impl Iterator<Item = Annotation> + 'a {
    let listed = (kind(tree, at) == Kind::Node(GoNode::Field))
        .then(|| listing(tree, parent(tree, at)))
        .flatten();
    let field_type = matches!(
        listed,
        Some(Listing::Members | Listing::Receiver | Listing::Parameters | Listing::Results)
    )
    .then(|| type_of(tree, at))
    .flatten();
    let spec_type = (kind(tree, at) == Kind::Node(GoNode::ValueSpec))
        .then(|| value_type(tree, at))
        .flatten();
    let fields = field_type.into_iter().flat_map(move |annotation| {
        let mut named = field_names(tree, at).peekable();
        let unnamed = named.peek().is_none().then_some(Annotation {
            annotation,
            target: at,
        });
        named
            .map(move |target| Annotation { annotation, target })
            .chain(unnamed)
    });
    let specs = spec_type.into_iter().flat_map(move |annotation| {
        spec_names(tree, at).map(move |target| Annotation { annotation, target })
    });
    fields.chain(specs)
}

/// The names the assignment or declaration at `at` binds to a string whose value is known: `x = "a"`, `x := "a"`, `var x = "a"`,
/// `const x, y = "a", "b"`. A name bound to anything else, or by an operator that computes (`+=`), binds no string here.
pub(crate) fn assignments<'a>(
    tree: &'a Tree<'_>,
    at: Position,
) -> impl Iterator<Item = Assignment> + 'a {
    let plain =
        operator(tree, at).is_some_and(|each| matches!(each.token, Token::Assign | Token::Define));
    let from_statement = (plain && kind(tree, at) == Kind::Node(GoNode::AssignStmt))
        .then(|| left(tree, at).zip(right(tree, at)));
    let assign = token(tree, at, Token::Assign);
    let from_spec = (kind(tree, at) == Kind::Node(GoNode::ValueSpec)).then(|| {
        spec_names(tree, at)
            .zip(nodes(tree, at).skip_while(move |each| assign.is_none_or(|assign| *each < assign)))
    });
    from_statement
        .into_iter()
        .flatten()
        .chain(from_spec.into_iter().flatten())
        .filter(move |(binding, value)| {
            kind(tree, *binding) == Kind::Node(GoNode::Ident) && string_token(tree, *value).is_some()
        })
        .map(|(binding, value)| Assignment { binding, value })
}

/// Every name the element at `at` declares, whatever it is.
pub(crate) fn declarations<'a>(
    tree: &'a Tree<'_>,
    at: Position,
) -> impl Iterator<Item = Declaration> + 'a {
    let shaped = shape(tree, at);
    let one = match shaped {
        DeclarationShape::Function => function_declaration(tree, at),
        DeclarationShape::Type => type_declaration(tree, at),
        DeclarationShape::Import => import_declaration(tree, at),
        DeclarationShape::Label => label_declaration(tree, at),
        DeclarationShape::Values | DeclarationShape::Field | DeclarationShape::Variables | DeclarationShape::Nothing => None,
    };
    let values = (shaped == DeclarationShape::Values).then(|| value_declarations(tree, at));
    let fields = (shaped == DeclarationShape::Field).then(|| field_declarations(tree, at));
    let variables = (shaped == DeclarationShape::Variables).then(|| variable_declarations(tree, at));
    one.into_iter()
        .chain(values.into_iter().flatten())
        .chain(fields.into_iter().flatten())
        .chain(variables.into_iter().flatten())
}
