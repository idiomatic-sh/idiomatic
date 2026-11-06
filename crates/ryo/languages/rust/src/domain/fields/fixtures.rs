//! The test support of the `fields` cases: what a row is given and the one subject every table is run through.

use std::sync::Arc;
use super::service::role_of;
use crate::domain::syntax::service::{kind, parent};
use crate::domain::syntax::types::Kind;
use crate::inbound::contracts::parse::contract::ParseContract;
use crate::outbound::syntax::fixtures::fake::SyntaxFake;
use crate::outbound::syntax::fixtures::trees::tree_of;
use ryo::languages::{self, Position, Tree};

/// An edge of the tree of some source: the nth edge, in source order, from a node of a kind to an element of a kind.
#[derive(Clone, Copy)]
pub(super) struct Edge {
    pub(super) source: &'static str,
    pub(super) parent: Kind,
    pub(super) child: Kind,
    pub(super) nth: usize,
}

pub(super) const fn edge(source: &'static str, parent: Kind, child: Kind, nth: usize) -> Edge {
    Edge {
        source,
        parent,
        child,
        nth,
    }
}

/// The role the edge plays in the table the parse builds of its source.
pub(super) fn subject(given: Edge) -> Result<&'static str, ()> {
    let language = ParseContract::new(
        Arc::new(SyntaxFake(
            tree_of(given.source).expect("a tree is written for the source"),
        )),
        Arc::new(crate::outbound::notes::factory::postcard()),
    );
    let (mut elements, mut symbols) = (
        languages::Elements::default(),
        languages::Symbols::default(),
    );
    languages::contract::build(
        &language,
        languages::ParseRequest {
            path: languages::SourcePath("src/lib.rs"),
            file: languages::FileId(0),
            bytes: languages::ByteSlice(given.source.as_bytes()),
            notes: languages::NoteRequest::Unwanted,
            fields: languages::FieldRequest::Wanted,
        },
        &mut elements,
        &mut symbols,
    )
    .map_err(drop)?;
    let tree = Tree {
        elements: &elements,
        source: languages::ByteSlice(given.source.as_bytes()),
        file: languages::FileId(0),
    };
    let found = languages::positions(&elements)
        .filter(|at| *at != Position(0))
        .filter(|at| {
            kind(&tree, parent(&tree, *at)) == given.parent && kind(&tree, *at) == given.child
        })
        .nth(given.nth)
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
