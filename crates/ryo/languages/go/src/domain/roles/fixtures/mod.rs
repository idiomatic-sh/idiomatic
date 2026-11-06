//! The test support of the `roles` cases: what a row is given and the one subject every table is run through.

use std::sync::Arc;
use super::service::role_of;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{kind, parent};
use crate::domain::syntax::types::Kind;
use crate::inbound::contracts::parse::contract::ParseContract;
use crate::outbound::syntax::factory::tree_sitter_go;
use ryo::languages::{self, Position, Tree};



















/// What a row is given: the nth edge, in source order, from a node of a kind to an element of a kind, or every pinned file.
#[derive(Clone, Copy)]
pub(super) enum Given {
    /// An edge of the tree of some source.
    Edge {
        /// The source.
        source: &'static str,
        /// The kind of the parent.
        parent: Node,
        /// The kind of the child.
        child: Kind,
        /// Which of the edges of those kinds, from zero.
        nth: usize,
    },
    /// Every relation's input, every file of the corpus and every file of the suite that `go/parser` reads.
    Pinned,
}

pub(super) const fn edge(source: &'static str, parent: Node, child: Kind, nth: usize) -> Given {
    Given::Edge {
        source,
        parent,
        child,
        nth,
    }
}

/// What a row is given.
type Input = Given;
/// The name of the role the edge plays, or the pinned files whose roles differ from `go/ast`'s, joined with ` | `.
type Value = &'static str;
type Failure = ();

/// The table the parse builds of `source`, roles asked for.
pub(super) fn table(source: &[u8]) -> Result<(languages::Elements, languages::Symbols), Failure> {
    let language = ParseContract::new(
        Arc::new(tree_sitter_go()),
        Arc::new(crate::outbound::notes::factory::postcard()),
    );
    let (mut elements, mut symbols) = (
        languages::Elements::default(),
        languages::Symbols::default(),
    );
    languages::contract::build(
        &language,
        languages::ParseRequest {
            path: languages::SourcePath("p.go"),
            file: languages::FileId(0),
            bytes: languages::ByteSlice(source),
            notes: languages::NoteRequest::Unwanted,
            fields: languages::FieldRequest::Wanted,
        },
        &mut elements,
        &mut symbols,
    )
    .map_err(drop)?;
    Ok((elements, symbols))
}



/// The role the edge plays in the table the parse builds of its source, or the pinned files whose roles differ.
pub(super) fn subject(given: Input) -> Result<Value, Failure> {
    let Given::Edge {
        source,
        parent: holder,
        child,
        nth,
    } = given
    else {
        return Err(());
    };
    let (elements, symbols) = table(source.as_bytes())?;
    let tree = Tree {
        elements: &elements,
        source: languages::ByteSlice(source.as_bytes()),
        file: languages::FileId(0),
    };
    let found = languages::positions(&elements)
        .filter(|at| *at != Position(0))
        .filter(|at| {
            kind(&tree, parent(&tree, *at)) == Kind::Node(holder) && kind(&tree, *at) == child
        })
        .nth(nth)
        .ok_or(())?;
    assert_eq!(role_of(&tree, found), languages::field_at(&elements, found));
    let name = languages::text_of(
        &symbols,
        tree.source,
        languages::field_name_at(&elements, found),
    )
    .ok_or(())?;
    Ok(Box::leak(name.0.to_owned().into_boxed_str()))
}
