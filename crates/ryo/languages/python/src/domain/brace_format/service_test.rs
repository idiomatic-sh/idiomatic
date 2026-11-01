//! Independent demand, reset and literal-input guards for brace-format parsing.
use super::service::{prepare, state};
use super::types::BraceFormatState;
use crate::domain::format_parsing::service::prepare as prepare_percent;
use crate::domain::model::types::Declarations;
use crate::fixtures::reading::{initialize_model, parsed};
use crate::outbound::format_syntax::adapters::ruff_literal_braceformat::RuffLiteralBraceFormat;
use crate::outbound::format_syntax::adapters::ruff_literal_cformat::RuffLiteralCFormat;
use ryo::languages::{ByteSlice, Tree};

#[test]
fn percent_only_demand_never_prepares_brace_grammar() {
    let source = b"'%s'.format(value)\n'{value}'\n";
    let mut file = parsed(source).expect("source parses");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: ryo::languages::FileId(0),
    };
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    let _percent = prepare_percent::<RuffLiteralCFormat>(&tree, &model, &mut file.symbols);
    assert_eq!(model.brace_format.borrow().state, BraceFormatState::Fresh);
}

#[test]
fn brace_parser_is_not_run_during_model_initialization() {
    let mut file = parsed(b"'{name}'\n").expect("source parses");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: ryo::languages::FileId(0),
    };
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    assert_eq!(model.brace_format.borrow().state, BraceFormatState::Fresh);
    drop(prepare::<RuffLiteralBraceFormat>(
        &tree,
        &model,
        &mut file.symbols,
    ));
    assert_eq!(
        state(&model.brace_format.borrow()),
        BraceFormatState::Complete
    );
}

#[test]
fn model_reinitialization_clears_brace_results_for_replacement_sources() {
    let mut first = parsed(b"'{first}'\n").expect("first parses");
    let first_tree = Tree {
        elements: &first.elements,
        source: ByteSlice(first.bytes),
        file: ryo::languages::FileId(0),
    };
    let mut model = Declarations::default();
    initialize_model(&first_tree, &mut model);
    drop(prepare::<RuffLiteralBraceFormat>(
        &first_tree,
        &model,
        &mut first.symbols,
    ));
    assert_eq!(model.brace_format.borrow().occurrences.len(), 1);

    let mut second = parsed(b"'{{}}'\n").expect("second parses");
    let second_tree = Tree {
        elements: &second.elements,
        source: ByteSlice(second.bytes),
        file: ryo::languages::FileId(0),
    };
    initialize_model(&second_tree, &mut model);
    assert_eq!(model.brace_format.borrow().state, BraceFormatState::Fresh);
    let index = prepare::<RuffLiteralBraceFormat>(&second_tree, &model, &mut second.symbols);
    assert!(
        matches!(index.values[0].result, super::types::BraceFormatResult::Parsed(ref parsed) if parsed.fields.is_empty())
    );
}
