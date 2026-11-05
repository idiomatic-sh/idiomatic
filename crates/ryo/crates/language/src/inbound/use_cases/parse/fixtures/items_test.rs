//! The shared rows of a language's items, run against an item of the catalog: one that writes `comment`, a relation with columns.

use super::check;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::answers::types::{Expansions, ReadItem, Reader, Reads, Visiting, declared};
use crate::domain::elements::types::Tree;
use ryo_relations::domain::relations::collections::relations::comment::item::{Comment, CommentRow};
use ryo_relations::domain::relations::types::{Position, Rows};
use crate::domain::symbols::types::Symbols;
use std::marker::PhantomData;

/// An item that reads `comment` and writes nothing.
struct CommentItem;

impl declared::Syntax for CommentItem {}

impl Visiting<Comment, ()> for CommentItem {
    fn visit(
        &self,
        _tree: &Tree<'_>,
        _model: &(),
        _symbols: &mut Symbols,
        _at: Position,
        _out: &mut dyn Rows<CommentRow>,
    ) {
    }
}

impl ReadItem for CommentItem {
    type Relation = Comment;
    type Model = ();

    const READER: Reader<Comment, ()> = Reader {
        reads: Reads::Every,
        visit: &CommentItem,
        expansions: Expansions::Ignored,
        relation: PhantomData,
    };
}

/// What a row asks: nothing.
type Input = ();
/// Whether the shared rows held.
type Value = ();
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("an_item_of_a_relation_with_columns_passes_the_shared_rows"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(()),
}];

fn subject((): Input) -> Result<Value, Failure> {
    check::<CommentItem>();
    Ok(())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
