//! The `declarations` service: free functions named by verb that read what a node declares, with
//! no IO and no ports. The match over the kinds is the table of what declares a name.

use crate::domain::nodes::types::RustNode;
use crate::domain::refinements::types::Declared;
use crate::domain::tokens::types::Token;

use super::constants::{MANY, NAMED};
use super::types::{Declaration, Implementation, Supertrait};
use crate::domain::bindings::service::{bound_name, in_let_pattern, looks_like_constant};
use crate::domain::bindings::types::{Constness, LetPattern};
use crate::domain::references::service::{is_single, name, segments};
use crate::domain::references::types::Singleness;
use crate::domain::refinements::service::refinement_name;
use crate::domain::syntax::service::{
    ancestors, child, end, is_token, kind, name_text, parent, start,
};
use crate::domain::syntax::service::{first_node, interned, is_type};
use crate::domain::syntax::types::{Kind, Leaf, Typeness};
use languages::Tree;
use ryo::languages;

/// What a declaration is under the shared closed set of kinds every language answers in.
#[must_use]
pub(crate) fn declaration_kind(declared: Declared) -> languages::DeclarationKind {
    use languages::DeclarationKind as Kind;
    match declared {
        Declared::Function => Kind::Function,
        Declared::Struct | Declared::Union => Kind::Struct,
        Declared::Enum => Kind::Enum,
        Declared::Trait => Kind::Trait,
        Declared::Variable | Declared::Static => Kind::Variable,
        Declared::Const => Kind::Constant,
        Declared::Module => Kind::Namespace,
        Declared::TypeAlias => Kind::TypeAlias,
        Declared::MacroRules | Declared::Macro => Kind::Macro,
        Declared::Method | Declared::AssociatedFunction => Kind::Method,
        Declared::Variant => Kind::EnumMember,
        Declared::Field | Declared::TupleField => Kind::Property,
        Declared::TypeParameter | Declared::LifetimeParameter | Declared::ConstParameter => {
            Kind::TypeParameter
        }
        Declared::Use | Declared::ExternCrate => Kind::Import,
        Declared::Implementation => Kind::Implementation,
        Declared::Label => Kind::Label,
    }
}

/// What a node that declares its own `NAME` declares.
pub(crate) fn named_kind(node: Kind) -> Option<Declared> {
    NAMED
        .iter()
        .find(|(each, _)| *each == node)
        .map(|(_, declared)| *declared)
}

/// A declaration whose name is the node `named`, written where it is.
fn written(tree: &Tree<'_>, declared: Declared, named: languages::Position) -> Option<Declaration> {
    Some(Declaration {
        declared,
        name: name_text(tree, named)?,
        named,
        from: start(tree, named),
        to: end(tree, named),
    })
}

/// The name a leaf of a `use` tree binds, and what it declares: its `as` name, else its last
/// segment, else, for `self`, the module the enclosing tree names. A glob binds no name of its
/// own, a group is no leaf, and `as _` binds none.
pub(crate) fn imported(tree: &Tree<'_>, node: languages::Position) -> Option<Declaration> {
    if child(tree, node, Kind::Node(RustNode::UseTreeList)).is_some()
        || child(tree, node, Kind::Token(Token::Star)).is_some()
    {
        return None;
    }
    if let Some(rename) = child(tree, node, Kind::Node(RustNode::Rename)) {
        return written(
            tree,
            Declared::Use,
            child(tree, rename, Kind::Node(RustNode::Name))?,
        );
    }
    let segment = child(
        tree,
        child(tree, node, Kind::Node(RustNode::Path))?,
        Kind::Node(RustNode::PathSegment),
    )?;
    let mut name = child(tree, segment, Kind::Node(RustNode::NameRef))?;
    if languages::children(tree.elements, name).any(|token| {
        (is_token(tree, token) == Leaf::Token) && kind(tree, token) == Kind::Token(Token::SelfKw)
    }) {
        let outer =
            ancestors(tree, node).find(|each| kind(tree, *each) == Kind::Node(RustNode::UseTree))?;
        name = child(
            tree,
            child(
                tree,
                child(tree, outer, Kind::Node(RustNode::Path))?,
                Kind::Node(RustNode::PathSegment),
            )?,
            Kind::Node(RustNode::NameRef),
        )?;
    }
    written(tree, Declared::Use, name)
}

/// What the node at `at` declares, when it declares a name.
pub(crate) fn declaration(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
) -> Option<Declaration> {
    let element = kind(tree, at);
    if element == Kind::Node(RustNode::LetStmt) {
        return written(tree, Declared::Variable, bound_name(tree, at)?);
    }
    // A name bound by a destructuring `let`, unless it is written as a constant is: it may be one.
    if element == Kind::Node(RustNode::IdentPat) {
        if in_let_pattern(tree, at) == LetPattern::Outside {
            return None;
        }
        let name = child(tree, at, Kind::Node(RustNode::Name))?;
        if looks_like_constant(tree, name) == Constness::Constant {
            return None;
        }
        return written(tree, Declared::Variable, name);
    }
    if element == Kind::Node(RustNode::Fn) {
        let declared = if kind(tree, parent(tree, at)) == Kind::Node(RustNode::AssocItemList) {
            let receives = child(tree, at, Kind::Node(RustNode::ParamList))
                .is_some_and(|list| child(tree, list, Kind::Node(RustNode::SelfParam)).is_some());
            if receives {
                Declared::Method
            } else {
                Declared::AssociatedFunction
            }
        } else {
            Declared::Function
        };
        return written(tree, declared, child(tree, at, Kind::Node(RustNode::Name))?);
    }
    if element == Kind::Node(RustNode::TupleField) {
        let index = languages::children(tree.elements, parent(tree, at))
            .filter(|each| is_token(tree, *each) == Leaf::Node)
            .position(|each| each == at)?;
        return Some(Declaration {
            declared: Declared::TupleField,
            name: number(
                tree,
                symbols,
                languages::ChildOrdinal(u32::try_from(index).unwrap_or(u32::MAX)),
            ),
            named: at,
            from: start(tree, at),
            to: end(tree, at),
        });
    }
    if element == Kind::Node(RustNode::Label) {
        return written(
            tree,
            Declared::Label,
            child(tree, at, Kind::Node(RustNode::Lifetime))?,
        );
    }
    if element == Kind::Node(RustNode::LifetimeParam) {
        return written(
            tree,
            Declared::LifetimeParameter,
            child(tree, at, Kind::Node(RustNode::Lifetime))?,
        );
    }
    if element == Kind::Node(RustNode::UseTree) {
        return imported(tree, at);
    }
    if element == Kind::Node(RustNode::Impl) {
        return crate::domain::implementations::service::implemented(tree, symbols, at).map(
            |header| Declaration {
                declared: Declared::Implementation,
                name: header.name,
                named: at,
                from: header.from,
                to: header.to,
            },
        );
    }
    if element == Kind::Node(RustNode::ExternCrate) {
        let name = match child(tree, at, Kind::Node(RustNode::Rename)) {
            Some(rename) => child(tree, rename, Kind::Node(RustNode::Name))?,
            None => child(tree, at, Kind::Node(RustNode::NameRef))?,
        };
        return written(tree, Declared::ExternCrate, name);
    }
    written(
        tree,
        named_kind(element)?,
        child(tree, at, Kind::Node(RustNode::Name))?,
    )
}

/// The name of a whole number, written out in decimal, with nothing allocated.
fn number(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    ordinal: languages::ChildOrdinal,
) -> languages::Symbol {
    let mut value = ordinal.0;
    let mut digits = [0_u8; 20];
    let mut used = 0;
    // Written from the last slot back, one digit a slot, until the value is spent.
    for (slot, count) in digits.iter_mut().rev().zip(1..) {
        *slot = b'0' + u8::try_from(value % 10).unwrap_or(0);
        value /= 10;
        used = count;
        if value == 0 {
            break;
        }
    }
    let written = digits.get(digits.len() - used..).unwrap_or_default();
    languages::intern_derived(
        symbols,
        tree.source,
        languages::NameText(std::str::from_utf8(written).unwrap_or_default()),
    )
}

/// The element a declaration is known by: a binding by where its name is, any other declaration by where it starts.
#[must_use]
pub(crate) fn identity(declaration: &Declaration, at: languages::Position) -> languages::Position {
    if declaration.declared == Declared::Variable {
        declaration.named
    } else {
        at
    }
}

/// The name of what a declaration is, in Rust's own words: its refinement.
pub(crate) fn refined(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    declaration: &Declaration,
) -> languages::Symbol {
    interned(tree, symbols, refinement_name(declaration.declared))
}

/// Where a declaration's name is written.
#[must_use]
pub(crate) fn declaration_span(declaration: &Declaration) -> languages::Span {
    languages::Span {
        start: declaration.from,
        end: declaration.to,
    }
}

/// The path a type is, when it is a path type.
fn path_of(tree: &Tree<'_>, ty: languages::Position) -> Option<languages::Position> {
    if kind(tree, ty) == Kind::Node(RustNode::PathType) {
        child(tree, ty, Kind::Node(RustNode::Path))
    } else {
        None
    }
}

/// The name of the path the type at `ty` is, when it is a path that writes a name.
fn named(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    ty: languages::Position,
) -> Option<languages::Symbol> {
    name(tree, symbols, path_of(tree, ty)?)
}

/// The traits the trait at `at` requires, one for each bound that writes a path.
pub(crate) fn supertraits<'a>(
    tree: &'a Tree<'a>,
    symbols: &'a mut languages::Symbols,
    at: languages::Position,
) -> impl Iterator<Item = Supertrait> + 'a {
    child(tree, at, Kind::Node(RustNode::TypeBoundList))
        .into_iter()
        .flat_map(move |bounds| {
            languages::children(tree.elements, bounds)
                .filter(move |each| kind(tree, *each) == Kind::Node(RustNode::TypeBound))
        })
        .filter_map(move |bound| {
            let contract = first_node(tree, bound)?;
            Some(Supertrait {
                contract,
                target: named(tree, symbols, contract)?,
            })
        })
}

/// What the file declares once by the name `name`, a `struct` or an `enum`: the first time it is asked, the file is read for them
/// once and the count of each name is noted. A name declared twice is declared by neither.
fn declared_once(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    name: languages::Symbol,
) -> Option<languages::Position> {
    if languages::has_noted_words(symbols) == languages::Noted::Nothing {
        languages::note_word(symbols, languages::Bits(0));
        for at in languages::positions(tree.elements) {
            if !matches!(kind(tree, at), Kind::Node(RustNode::Struct | RustNode::Enum)) {
                continue;
            }
            let Some(declared) =
                child(tree, at, Kind::Node(RustNode::Name)).and_then(|each| name_text(tree, each))
            else {
                continue;
            };
            let before = languages::note_of(symbols, declared);
            languages::set_note(
                symbols,
                declared,
                if before == languages::NameNote(0) {
                    languages::NameNote(at.0 + 1)
                } else {
                    MANY
                },
            );
        }
    }
    match languages::note_of(symbols, name) {
        languages::NameNote(0) | MANY => None,
        languages::NameNote(place) => Some(languages::Position(place - 1)),
    }
}

/// The trait the `impl` at `at` implements, written before its `for`: what inherits it is the declaration of the type it is
/// implemented for when that is a plain name the file declares once, otherwise (a type of another file or crate, a generic parameter, a
/// reference, a slice) the `impl` itself.
pub(crate) fn implementation(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
) -> Option<Implementation> {
    let written_for = child(tree, at, Kind::Token(Token::ForKw))?;
    let before = languages::children(tree.elements, at)
        .take_while(|each| *each != written_for)
        .find(|each| is_type(tree, *each) == Typeness::Type)?;
    let after = languages::children(tree.elements, at)
        .skip_while(|each| *each != written_for)
        .find(|each| is_type(tree, *each) == Typeness::Type)?;
    let target = named(tree, symbols, before)?;
    let mut only = languages::NO_SYMBOL;
    if let Some(path) =
        path_of(tree, after).filter(|path| is_single(tree, *path) == Singleness::Single)
    {
        segments(tree, path, &mut |segment| only = segment);
    }
    Some(Implementation {
        contract: before,
        target,
        declaration: declared_once(tree, symbols, only).unwrap_or(at),
    })
}
