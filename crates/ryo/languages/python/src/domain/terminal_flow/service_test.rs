//! The terminal index is lazy, memoized and cleared on source replacement.
use crate::cases::{Case, CaseName, Expect, Word, run};
use super::{
    service::{outcome, prepare},
    types::IndexState,
};
use crate::domain::model::types::Declarations;
use crate::fixtures::reading::{initialize_model, parsed};
use ryo::languages::{self, ByteSlice, Tree};

fn is_lazy_retained_and_invalidated_between_files() {
    let first = parsed(b"def f():\n    raise ValueError()\n").expect("source parses");
    let first_tree = Tree {
        elements: &first.elements,
        source: ByteSlice(first.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    initialize_model(&first_tree, &mut model);
    assert_eq!(model.terminal_flow.borrow().state, IndexState::Fresh);
    let index = prepare(&first_tree, &model);
    assert_eq!(index.state, IndexState::Complete);
    let function = (0..index.nodes.len()).find_map(|slot| {
        let position = languages::Position(u32::try_from(slot).ok()?);
        (outcome(&index, position) == Some("raise")).then_some(position)
    });
    assert!(function.is_some());
    let capacity = index.nodes.capacity();
    drop(index);

    let second = parsed(b"def g():\n    pass\n").expect("replacement source parses");
    let second_tree = Tree {
        elements: &second.elements,
        source: ByteSlice(second.bytes),
        file: languages::FileId(0),
    };
    initialize_model(&second_tree, &mut model);
    assert_eq!(model.terminal_flow.borrow().state, IndexState::Fresh);
    let replacement = prepare(&second_tree, &model);
    assert!(replacement.nodes.capacity() >= capacity);
    assert!(
        replacement
            .nodes
            .iter()
            .any(|summary| summary.function_terminal == super::types::Outcome::Implicit)
    );
}

/// Each check below states its own source and asserts its own outcome.
const CASES: &[Case<fn(), (), ()>] = &[
    Case {
        name: CaseName("is_lazy_retained_and_invalidated_between_files"),
        word: Word::Succeeds,
        input: || is_lazy_retained_and_invalidated_between_files,
        expect: Expect::Value(()),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |check| {
        check();
        Ok(())
    });
}
