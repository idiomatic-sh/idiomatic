//! The `grammar` service: free functions named by verb over the library's tree as plain data, with no IO and no ports. It moves about
//! the tree the way the grammar's nodes are met, and says where `go/ast` ends a node the library ends elsewhere.

use ryo::languages::Count;

use super::types::{ByteIndex, Concrete, Form, Index, Naming, Raw, Role};
use crate::domain::tokens::types::Token;

/// The node at `at`.
#[must_use]
pub(crate) fn raw(tree: &Concrete<'_>, at: Index) -> Raw {
    tree.nodes
        .get(at.0)
        .copied()
        .unwrap_or_else(|| tree.nodes.first().copied().unwrap_or(EMPTY))
}

/// The node the tree answers for a place it does not hold: a tree always holds the places it hands out, so this is never met.
const EMPTY: Raw = Raw {
    form: Form::Unknown,
    role: None,
    naming: Naming::Anonymous,
    presence: super::types::Presence::Missing,
    start: ByteIndex(0),
    end: ByteIndex(0),
    parent: None,
    first: None,
    last: None,
    next: None,
    previous: None,
};

/// What the node at `at` is in Go.
#[must_use]
pub(crate) fn form(tree: &Concrete<'_>, at: Index) -> Form {
    raw(tree, at).form
}

/// Where the node at `at` starts.
#[must_use]
pub(crate) fn start(tree: &Concrete<'_>, at: Index) -> ByteIndex {
    raw(tree, at).start
}

/// Where the library says the node at `at` ends.
#[must_use]
pub(crate) fn stop(tree: &Concrete<'_>, at: Index) -> ByteIndex {
    raw(tree, at).end
}

/// The form of the node the node at `at` is under; none for the root.
#[must_use]
pub(crate) fn parent_form(tree: &Concrete<'_>, at: Index) -> Option<Form> {
    raw(tree, at).parent.map(|each| form(tree, each))
}

/// The nodes under the node at `at`, in order.
pub(crate) fn children<'a>(
    tree: &'a Concrete<'_>,
    at: Index,
) -> impl Iterator<Item = Index> + use<'a> {
    let mut next = raw(tree, at).first;
    std::iter::from_fn(move || {
        let current = next?;
        next = raw(tree, current).next;
        Some(current)
    })
}

/// The nodes under the node at `at`, last first.
pub(crate) fn children_back<'a>(
    tree: &'a Concrete<'_>,
    at: Index,
) -> impl Iterator<Item = Index> + use<'a> {
    let mut next = raw(tree, at).last;
    std::iter::from_fn(move || {
        let current = next?;
        next = raw(tree, current).previous;
        Some(current)
    })
}

/// How many nodes are under the node at `at`.
#[must_use]
pub(crate) fn child_count(tree: &Concrete<'_>, at: Index) -> Count {
    Count(children(tree, at).count())
}

/// How many named nodes are under the node at `at`, comments among them.
#[must_use]
pub(crate) fn named_count(tree: &Concrete<'_>, at: Index) -> Count {
    Count(
        children(tree, at)
            .filter(|each| raw(tree, *each).naming == Naming::Named)
            .count(),
    )
}

/// The named nodes under the node at `at`, in order, but for the comments among them.
pub(crate) fn named<'a>(
    tree: &'a Concrete<'_>,
    at: Index,
) -> impl Iterator<Item = Index> + use<'a> {
    children(tree, at).filter(move |each| {
        let node = raw(tree, *each);
        node.naming == Naming::Named && node.form != Form::Comment
    })
}

/// The first node under the node at `at` that plays the part `role`.
#[must_use]
pub(crate) fn field(tree: &Concrete<'_>, at: Index, role: Role) -> Option<Index> {
    children(tree, at).find(|each| raw(tree, *each).role == Some(role))
}

/// The first keyword, operator or piece of punctuation under the node at `at` that is `token`.
#[must_use]
pub(crate) fn spelled(tree: &Concrete<'_>, at: Index, token: Token) -> Option<Index> {
    children(tree, at).find(|each| {
        let node = raw(tree, *each);
        node.naming == Naming::Anonymous && node.form == Form::Spelled(token)
    })
}

/// The first node under the node at `at`, whatever it is.
#[must_use]
pub(crate) fn first(tree: &Concrete<'_>, at: Index) -> Option<Index> {
    raw(tree, at).first
}

/// The first byte after the node at `at` where `go/parser` finds a token: the start of the next node that is no comment, up the tree
/// from the node, or the end of the node when nothing follows.
#[must_use]
pub(crate) fn next_token(tree: &Concrete<'_>, at: Index) -> ByteIndex {
    let mut current = at;
    loop {
        match raw(tree, current).next {
            Some(next) if form(tree, next) == Form::Comment => current = next,
            Some(next) => return start(tree, next),
            None => match raw(tree, current).parent {
                Some(up) => current = up,
                None => return stop(tree, at),
            },
        }
    }
}

/// The place the label at `at` ends in `go/ast`, when it is a label with no statement after it: where `go/parser` puts the empty
/// statement it makes for it.
#[must_use]
pub(crate) fn empty_label(tree: &Concrete<'_>, at: Index) -> Option<ByteIndex> {
    (form(tree, at) == Form::Labeled && named(tree, at).count() == 1).then(|| next_token(tree, at))
}

/// Where a node of the library ends in the file: its own end, but not after the comments the library keeps at the end of a list of
/// statements, nor after the line end that list takes, which `go/ast` holds in no node; and a label with no statement ends where the
/// next token starts, where `go/parser` puts the empty statement it makes for it.
#[must_use]
pub(crate) fn ending(tree: &Concrete<'_>, at: Index) -> ByteIndex {
    if let Some(label) = empty_label(tree, at) {
        return label;
    }
    let listing = form(tree, at) == Form::StatementList;
    for child in children_back(tree, at) {
        let inner = form(tree, child);
        if inner == Form::Comment {
            continue;
        }
        return if listing || inner == Form::StatementList {
            ending(tree, child)
        } else {
            stop(tree, at)
        };
    }
    stop(tree, at)
}

/// The text the node at `at` spans, as it is written.
#[must_use]
pub(crate) fn written<'a>(tree: &Concrete<'a>, at: Index) -> super::types::GoText<'a> {
    super::types::GoText(
        tree.text
            .0
            .get(start(tree, at).0..stop(tree, at).0)
            .unwrap_or_default(),
    )
}
