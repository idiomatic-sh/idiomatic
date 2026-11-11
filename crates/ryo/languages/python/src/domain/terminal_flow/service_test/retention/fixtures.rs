//! The scenarios of the group: each runs its subject and records what it observed.
use super::super::super::service::{outcome, prepare};
use crate::domain::model::types::Declarations;
use crate::fixtures::reading::{initialize_model, parsed};
use ryo::languages::{self, ByteSlice, Tree};

pub(super) fn is_lazy_retained_and_invalidated_between_files() -> String {
    let mut observed: Vec<String> = Vec::new();
    let first = parsed(b"def f():\n    raise ValueError()\n").expect("source parses");
    let first_tree = Tree {
        elements: &first.elements,
        source: ByteSlice(first.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    initialize_model(&first_tree, &mut model);
    observed.push(format!("{:?}", model.terminal_flow.borrow().state));
    let index = prepare(&first_tree, &model.terminal_flow);
    observed.push(format!("{:?}", index.state));
    let function = (0..index.nodes.len()).find_map(|slot| {
        let position = languages::Position(u32::try_from(slot).ok()?);
        (outcome(&index, position) == Some(languages::NameText("raise"))).then_some(position)
    });
    observed.push(format!("{}", function.is_some()));
    let capacity = index.nodes.capacity();
    drop(index);

    let second = parsed(b"def g():\n    pass\n").expect("replacement source parses");
    let second_tree = Tree {
        elements: &second.elements,
        source: ByteSlice(second.bytes),
        file: languages::FileId(0),
    };
    initialize_model(&second_tree, &mut model);
    observed.push(format!("{:?}", model.terminal_flow.borrow().state));
    let replacement = prepare(&second_tree, &model.terminal_flow);
    observed.push(format!("{}", replacement.nodes.capacity() >= capacity));
    observed.push(format!(
        "{}",
        replacement
            .nodes
            .iter()
            .any(|summary| summary.function_terminal
                == super::super::super::types::TerminalOutcome::Implicit)
    ));
    observed.join("|")
}
