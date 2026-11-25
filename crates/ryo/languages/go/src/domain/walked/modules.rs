//! The `modules` service: free functions named by verb that find the modules a repository's manifests declare, say which require which and
//! which a workspace holds, and place each file in one, with no IO and no ports.

use crate::domain::file_view::types::Syntax as Tree;
use ryo::languages;

use crate::domain::modules::constants::{QUOTES, REQUIRE_VERB};
use crate::domain::modules::types::{ModuleMapping, RequiredLine, Verb};
use std::borrow::Cow;

use crate::domain::extras::types::Extra;
use crate::domain::literals::service::unquoted;
use crate::domain::literals::types::{Unquoted, Written as LiteralText};
use crate::domain::walked::syntax::{children, kind, parent, written};
use crate::domain::syntax::types::Kind;

/// The words of the `go.mod` element at `at`, in order: its children that are words.
fn words<'a>(
    tree: &'a Tree<'_>,
    at: languages::Position,
) -> impl Iterator<Item = languages::Position> + 'a {
    children(tree, at)
        .filter(move |each| kind(tree, *each) == Kind::Extra(Extra::ModWord))
}

/// The text of the `go.mod` word at `at`, as Go reads it: a quoted word without its quotes and with its escapes decoded
/// (`"example.com/\u007a"` is `example.com/z`), as `modfile` reads one with `strconv.Unquote`; a word that is no string, or a
/// string that holds no text, as it is written. Its spelling is `written`.
#[must_use]
pub(crate) fn word_text<'a>(tree: &Tree<'a>, at: languages::Position) -> Unquoted<'a> {
    let spelled = written(tree, at).0;
    if spelled.starts_with(QUOTES) {
        unquoted(LiteralText(spelled)).unwrap_or(Unquoted(Cow::Borrowed(spelled)))
    } else {
        Unquoted(Cow::Borrowed(spelled))
    }
}

/// Whether the `go.mod` element at `at` starts with the verb that requires a module.
fn verb_of(tree: &Tree<'_>, at: languages::Position) -> Verb {
    if words(tree, at)
        .next()
        .is_some_and(|verb| word_text(tree, verb).0 == REQUIRE_VERB)
    {
        Verb::Require
    } else {
        Verb::Other
    }
}

/// The requirement the `go.mod` line at `at` writes, when it writes one: a line of a `require` block, or a `require` line of its own.
#[must_use]
pub(crate) fn required_at(tree: &Tree<'_>, at: languages::Position) -> Option<RequiredLine> {
    if kind(tree, at) != Kind::Extra(Extra::ModLine) {
        return None;
    }
    let owner = parent(tree, at);
    let in_block = kind(tree, owner) == Kind::Extra(Extra::ModLineBlock);
    if in_block && verb_of(tree, owner) != Verb::Require {
        return None;
    }
    if !in_block && verb_of(tree, at) != Verb::Require {
        return None;
    }
    // A line of a block writes the module first; a line of its own writes the verb first.
    let skipped = usize::from(!in_block);
    let mut written = words(tree, at).skip(skipped);
    let path = written.next()?;
    let version = written.next();
    let earlier = children(tree, owner)
        .take_while(|each| *each != at)
        .filter(|each| {
            kind(tree, *each) == Kind::Extra(Extra::ModLine)
                && (in_block || verb_of(tree, *each) == Verb::Require)
        })
        .count();
    Some(RequiredLine {
        path,
        version,
        owner,
        ordinal: languages::Integer(i64::try_from(earlier).unwrap_or(i64::MAX)),
    })
}

/// The mapping the `go.mod` element at `at` is, when it is one: the file, which holds the `require` lines of its own, and each
/// `require` block, which holds its lines. The file is implied by its lines and holds itself; a block is opened by its `require (`
/// and is held by the file.
#[must_use]
pub(crate) fn mapping_of(tree: &Tree<'_>, at: languages::Position) -> Option<ModuleMapping> {
    let found = kind(tree, at);
    if found == Kind::Extra(Extra::ModFile) {
        Some(ModuleMapping {
            kind: languages::MappingKind::Implicit,
            owner: at,
        })
    } else if found == Kind::Extra(Extra::ModLineBlock) && verb_of(tree, at) == Verb::Require {
        Some(ModuleMapping {
            kind: languages::MappingKind::Standard,
            owner: parent(tree, at),
        })
    } else {
        None
    }
}
