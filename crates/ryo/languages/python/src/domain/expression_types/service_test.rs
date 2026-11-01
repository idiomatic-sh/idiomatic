//! The expression-type index is lazy, memoized and cleared on source replacement.
use super::{
    service::prepare,
    types::{IndexState, Resolution},
};
use crate::domain::model::types::Declarations;
use crate::fixtures::reading::{initialize_model, parsed};
use ryo::languages::{self, ByteSlice, Tree};

#[test]
fn is_lazy_retained_and_invalidated_between_files() {
    let first = parsed(b"x = 1 + 2.0\n").expect("source parses");
    let first_tree = Tree {
        elements: &first.elements,
        source: ByteSlice(first.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    initialize_model(&first_tree, &mut model);
    assert_eq!(model.expression_types.borrow().state, IndexState::Fresh);
    let index = prepare(&first_tree, &model);
    assert_eq!(index.state, IndexState::Complete);
    assert!(
        index
            .entries
            .iter()
            .flatten()
            .any(|value| *value == Resolution::Atom("float"))
    );
    let capacity = index.entries.capacity();
    drop(index);

    let second = parsed(b"x = 'changed'\n").expect("replacement source parses");
    let second_tree = Tree {
        elements: &second.elements,
        source: ByteSlice(second.bytes),
        file: languages::FileId(0),
    };
    initialize_model(&second_tree, &mut model);
    assert_eq!(model.expression_types.borrow().state, IndexState::Fresh);
    let replacement = prepare(&second_tree, &model);
    assert!(replacement.entries.capacity() >= capacity);
    assert!(
        replacement
            .entries
            .iter()
            .flatten()
            .any(|value| *value == Resolution::Atom("string"))
    );
}
