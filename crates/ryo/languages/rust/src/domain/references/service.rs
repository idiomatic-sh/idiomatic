//! The `references` service: free functions named by verb that read the names a path writes, with
//! no IO and no ports. A path is written out segment by segment; its name joins the segments with
//! `::` and leaves the generic arguments out.

use super::constants::{CFG, CFG_ATTR, CLOSERS, OPENERS, SEPARATOR};
use super::types::{Called, Singleness, Usage};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{
    child, end, first_body, first_node, is_inside, is_token, kind, name_text, node_after, parent,
    start, written,
};
use crate::domain::syntax::types::{Containment, Kind, Leaf};
use crate::domain::tokens::types::Token;
use ryo::languages;
use ryo::languages::Tree;

/// Gives `each` the text of every segment of the path at `path`, in source order: the qualifier's
/// first. A segment that is no name (`<T as Trait>`) has none.
pub(crate) fn segments(
    tree: &Tree<'_>,
    path: languages::Position,
    each: &mut impl FnMut(languages::Symbol),
) {
    if let Some(qualifier) = child(tree, path, Kind::Node(Node::Path)) {
        segments(tree, qualifier, each);
    }
    if let Some(name) = child(tree, path, Kind::Node(Node::PathSegment))
        .and_then(|segment| child(tree, segment, Kind::Node(Node::NameRef)))
        .and_then(|name| name_text(tree, name))
    {
        each(name);
    }
}

/// Whether the path at `path` writes exactly one name.
#[must_use]
pub(crate) fn is_single(tree: &Tree<'_>, path: languages::Position) -> Singleness {
    let mut counted = 0_i32;
    segments(tree, path, &mut |_| counted += 1_i32);
    if counted == 1 {
        Singleness::Single
    } else {
        Singleness::Several
    }
}

/// The name of the path at `path`, its segments joined with `::`; nothing for a path that writes none.
pub(crate) fn name(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    path: languages::Position,
) -> Option<languages::Symbol> {
    let (mut counted, mut only) = (0_i32, languages::NO_SYMBOL);
    segments(tree, path, &mut |segment| {
        counted += 1_i32;
        only = segment;
    });
    match counted {
        0 => None,
        1 => Some(only),
        _ => {
            let from = languages::begin_name(symbols);
            let mut first = true;
            segments(tree, path, &mut |segment| {
                if !first {
                    languages::push_text(symbols, languages::NameText(SEPARATOR));
                }
                first = false;
                languages::push_name(symbols, tree.source, segment);
            });
            Some(languages::end_name(symbols, tree.source, from))
        }
    }
}

/// Whether the path at `at` is a use of a name: `a::b::c` parses as nested qualifier paths, and only the outermost is a reference, and
/// the paths of a `use` are imports.
#[must_use]
pub(crate) fn is_written_use(tree: &Tree<'_>, at: languages::Position) -> Usage {
    if kind(tree, parent(tree, at)) != Kind::Node(Node::Path)
        && is_inside(tree, at, Kind::Node(Node::UseTree)) == Containment::Outside
    {
        Usage::Written
    } else {
        Usage::Skipped
    }
}

/// What the call at `at` calls: the path written out as its callee, or the name a method is called by.
pub(crate) fn called(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
) -> Option<Called> {
    if kind(tree, at) == Kind::Node(Node::CallExpr) {
        let callee =
            first_body(tree, at).filter(|each| kind(tree, *each) == Kind::Node(Node::PathExpr))?;
        let target = child(tree, callee, Kind::Node(Node::Path))
            .and_then(|path| name(tree, symbols, path))?;
        Some(Called { callee, target })
    } else {
        let callee = child(tree, at, Kind::Node(Node::NameRef))?;
        Some(Called {
            callee,
            target: name_text(tree, callee)?,
        })
    }
}

/// The name an attribute is written with: the path it names, or `cfg` and `cfg_attr` for the two the parser reads apart.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "the meta of an attribute is a `cfg`, a `cfg_attr` or a path, and a path is every other kind, so naming the path is right for every kind not named; `references/service_test.rs` holds a row per attribute form and the oracle rail compares every attribute of the Rust suite"
)]
pub(crate) fn attribute_target(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    attribute: languages::Position,
) -> Option<languages::Symbol> {
    let meta = first_node(tree, attribute)?;
    match kind(tree, meta) {
        Kind::Node(Node::CfgMeta) => Some(languages::intern_derived(
            symbols,
            tree.source,
            languages::NameText(CFG),
        )),
        Kind::Node(Node::CfgAttrMeta) => Some(languages::intern_derived(
            symbols,
            tree.source,
            languages::NameText(CFG_ATTR),
        )),
        _ => {
            let path = child(tree, meta, Kind::Node(Node::Path))?;
            name(tree, symbols, path)
        }
    }
}

/// The arguments the attribute at `attribute` passes, as written: what its delimiters hold (`Clone, Debug` of `#[derive(Clone, Debug)]`,
/// `test` of `#[cfg(test)]`), the value of `#[name = value]`, or nothing for an attribute that is only a path.
pub(crate) fn attribute_arguments(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    attribute: languages::Position,
) -> languages::Symbol {
    let (from, to) = first_node(tree, attribute)
        .and_then(|meta| {
            if kind(tree, meta) == Kind::Node(Node::KeyValueMeta) {
                let value = node_after(tree, meta, Kind::Token(Token::Eq))?;
                return Some((start(tree, value), end(tree, value)));
            }
            let delimited = child(tree, meta, Kind::Node(Node::TokenTree)).unwrap_or(meta);
            let mut tokens = languages::children(tree.elements, delimited)
                .filter(|each| is_token(tree, *each) == Leaf::Token);
            let open = tokens.find(|each| OPENERS.contains(&kind(tree, *each)))?;
            let close = tokens
                .filter(|each| CLOSERS.contains(&kind(tree, *each)))
                .last()?;
            Some((end(tree, open), start(tree, close)))
        })
        .unwrap_or_default();
    written(tree, symbols, from, to)
}

/// The place of the attribute at `attribute` among the attributes of `declaration`.
#[must_use]
pub(crate) fn attribute_ordinal(
    tree: &Tree<'_>,
    declaration: languages::Position,
    attribute: languages::Position,
) -> languages::ChildOrdinal {
    let before = languages::children(tree.elements, declaration)
        .take_while(|each| *each != attribute)
        .filter(|each| kind(tree, *each) == Kind::Node(Node::Attr))
        .count();
    languages::ChildOrdinal(u32::try_from(before).unwrap_or(u32::MAX))
}

/// The arguments the call at `call` hands its callee, in the order of the callee's parameters: the receiver of a method call, which
/// its `self` parameter takes, then the arguments in the parentheses.
pub(crate) fn arguments_of<'a>(
    tree: &'a Tree<'a>,
    call: languages::Position,
) -> impl Iterator<Item = languages::Position> + 'a {
    let receiver =
        first_body(tree, call).filter(|_| kind(tree, call) == Kind::Node(Node::MethodCallExpr));
    receiver.into_iter().chain(
        child(tree, call, Kind::Node(Node::ArgList))
            .into_iter()
            .flat_map(move |list| {
                languages::children(tree.elements, list).filter(move |each| {
                    is_token(tree, *each) == Leaf::Node
                        && kind(tree, *each) != Kind::Node(Node::Attr)
                })
            }),
    )
}

/// The name the argument at `argument` hands on: the one name it writes, behind a reference, a dereference or parentheses; the
/// unknown value when it is anything else (a call, a field, a literal, a path of several names).
#[must_use]
pub(crate) fn passed(tree: &Tree<'_>, argument: languages::Position) -> languages::Symbol {
    let mut current = argument;
    loop {
        match kind(tree, current) {
            Kind::Node(Node::PathExpr) => {
                let (mut counted, mut only) = (0_i32, languages::NO_SYMBOL);
                if let Some(path) = child(tree, current, Kind::Node(Node::Path)) {
                    segments(tree, path, &mut |segment| {
                        counted += 1_i32;
                        only = segment;
                    });
                }
                return if counted == 1 {
                    only
                } else {
                    languages::NO_SYMBOL
                };
            }
            Kind::Node(Node::RefExpr | Node::ParenExpr) => {}
            Kind::Node(Node::PrefixExpr)
                if child(tree, current, Kind::Token(Token::Star)).is_some() => {}
            Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => return languages::NO_SYMBOL,
        }
        match first_body(tree, current) {
            Some(inner) => current = inner,
            None => return languages::NO_SYMBOL,
        }
    }
}
