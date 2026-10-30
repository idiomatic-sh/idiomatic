//! The `imports` service: free functions named by verb that read the names a `use` brings in, with
//! no IO and no ports. A `use` tree is a path, a group of trees under it, a glob or a rename.

use super::constants::{GLOB, ROOT, SELF_SEGMENT, UNDERSCORE};
use super::types::{Ending, Import, Imported, Rooting};
use crate::domain::nodes::types::Node;
use crate::domain::references::constants::SEPARATOR;
use crate::domain::references::service::segments;
use crate::domain::syntax::service::{child, kind, last_descendant, name_text, parent};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use ryo::languages;
use ryo::languages::Tree;

/// The path `prefix` followed by the path at `path` (or nothing), and a glob when `ending` is one.
fn joined(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    prefix: Option<Imported>,
    path: Option<languages::Position>,
    rooting: Rooting,
    ending: Ending,
) -> Option<Imported> {
    let mut last = prefix.map_or(languages::NO_SYMBOL, |each| each.last);
    let from = languages::begin_name(symbols);
    let mut first = true;
    match (prefix, rooting) {
        (Some(each), _) => {
            languages::push_name(symbols, tree.source, each.target);
            first = false;
        }
        (None, Rooting::Absolute) => languages::push_text(symbols, languages::NameText(ROOT)),
        (None, Rooting::Relative) => {}
    }
    if let Some(path) = path {
        segments(tree, path, &mut |segment| {
            if !first {
                languages::push_text(symbols, languages::NameText(SEPARATOR));
            }
            first = false;
            languages::push_name(symbols, tree.source, segment);
            last = segment;
        });
    }
    if ending == Ending::Glob {
        if !first {
            languages::push_text(symbols, languages::NameText(SEPARATOR));
        }
        first = false;
        languages::push_text(symbols, languages::NameText(GLOB));
        last = languages::intern_derived(symbols, tree.source, languages::NameText(GLOB));
    }
    if first {
        return None;
    }
    Some(Imported {
        target: languages::end_name(symbols, tree.source, from),
        last,
    })
}

/// Whether the `use` tree at `use_tree` starts its path with `::`: on its first segment, or on the tree itself when it writes no path.
fn rooting_of(tree: &Tree<'_>, use_tree: languages::Position) -> Rooting {
    let Some(mut path) = child(tree, use_tree, Kind::Node(Node::Path)) else {
        return if child(tree, use_tree, Kind::Token(Token::Colon2)).is_some() {
            Rooting::Absolute
        } else {
            Rooting::Relative
        };
    };
    while let Some(inner) = child(tree, path, Kind::Node(Node::Path)) {
        path = inner;
    }
    if child(tree, path, Kind::Node(Node::PathSegment))
        .is_some_and(|segment| child(tree, segment, Kind::Token(Token::Colon2)).is_some())
    {
        Rooting::Absolute
    } else {
        Rooting::Relative
    }
}

/// The leaves of the `use` tree at `use_tree`, in source order: the trees that name what they bring in, none of the groups above them.
pub(crate) fn leaves<'a>(
    tree: &'a Tree<'a>,
    use_tree: languages::Position,
) -> impl Iterator<Item = languages::Position> + 'a {
    (use_tree.0..=last_descendant(tree, use_tree).0)
        .map(languages::Position)
        .filter(|at| {
            kind(tree, *at) == Kind::Node(Node::UseTree)
                && child(tree, *at, Kind::Node(Node::UseTreeList)).is_none()
        })
}

/// What the groups above the `use` tree at `use_tree` write: the path down to the group it is in.
fn above(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    use_tree: languages::Position,
) -> Option<Imported> {
    let list = parent(tree, use_tree);
    if kind(tree, list) != Kind::Node(Node::UseTreeList) {
        return None;
    }
    let group = parent(tree, list);
    (kind(tree, group) == Kind::Node(Node::UseTree)).then(|| through(tree, symbols, group))?
}

/// The path the `use` tree at `use_tree` names, with the groups above it: a `self` and a tree that writes no path name the group.
fn through(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    use_tree: languages::Position,
) -> Option<Imported> {
    let prefix = above(tree, symbols, use_tree);
    let path = child(tree, use_tree, Kind::Node(Node::Path));
    let (mut count, mut only) = (0_i32, languages::NO_SYMBOL);
    if let Some(path) = path {
        segments(tree, path, &mut |segment| {
            count += 1_i32;
            only = segment;
        });
    }
    let only_self = count == 1_i32
        && prefix.is_some()
        && languages::text_of(symbols, tree.source, only).map(|text| text.0) == Some(SELF_SEGMENT);
    if only_self || count == 0 {
        prefix
    } else {
        joined(
            tree,
            symbols,
            prefix,
            path,
            rooting_of(tree, use_tree),
            Ending::Name,
        )
    }
}

/// What the leaf at `leaf` brings in: the name it binds and the path it imports; nothing for a leaf that names nothing.
pub(crate) fn import_of(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    leaf: languages::Position,
) -> Option<Import> {
    let target = through(tree, symbols, leaf);
    let target = if child(tree, leaf, Kind::Token(Token::Star)).is_some() {
        joined(
            tree,
            symbols,
            target,
            None,
            rooting_of(tree, leaf),
            Ending::Glob,
        )
    } else {
        target
    }?;
    let binding = match child(tree, leaf, Kind::Node(Node::Rename)) {
        Some(rename) => {
            match child(tree, rename, Kind::Node(Node::Name)).and_then(|name| name_text(tree, name))
            {
                Some(name) => name,
                None => {
                    languages::intern_derived(symbols, tree.source, languages::NameText(UNDERSCORE))
                }
            }
        }
        None => target.last,
    };
    Some(Import {
        binding,
        target: target.target,
    })
}
