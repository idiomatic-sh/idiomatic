//! Independent demand, reset and literal-input guards for brace-format parsing.
use super::service::{prepare, state};
use super::types::BraceFormatState;
use crate::domain::format_parsing::service::prepare as prepare_percent;
use crate::domain::model::types::Declarations;
use crate::fixtures::reading::{initialize_model, parsed};
use crate::outbound::format_syntax::adapters::ruff_literal_braceformat::RuffLiteralBraceFormat;
use crate::outbound::format_syntax::adapters::ruff_literal_cformat::RuffLiteralCFormat;
use ryo::languages::{ByteSlice, Tree};

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
        matches!(&index.values[0].result, super::types::BraceFormatResult::Parsed(parsed) if parsed.fields.is_empty())
    );
}

fn retained_format_specs_are_charged_by_the_existing_grammar_budget() {
    use crate::domain::brace_format::types::{
        BraceFieldContext, BraceFormatField, BraceFormatResult, BraceSelector, ParsedBraceFormat,
    };
    let with_capacity = |capacity| {
        BraceFormatResult::Parsed(ParsedBraceFormat {
            fields: vec![BraceFormatField {
                ordinal: crate::domain::format_syntax::types::FormatFieldOrdinal(0),
                context: BraceFieldContext::Literal,
                selector: BraceSelector::Automatic {
                    index: crate::domain::brace_format::types::BraceArgumentIndex(0),
                    overflow: crate::domain::brace_format::types::SelectorRange::Representable,
                },
                format_spec: Some(crate::domain::brace_format::types::BraceSpecText(
                    String::with_capacity(capacity),
                )),
                selector_text: crate::domain::brace_format::types::BraceSelectorText(String::new()),
                conversion: None,
            }],
            ..Default::default()
        })
    };
    // Retained allocation counts even when the decoded specification is empty.
    let small = with_capacity(0);
    let reserved = with_capacity(8192);
    assert!(
        super::service::owned_bytes(&reserved).0 >= super::service::owned_bytes(&small).0 + 8192
    );
}

use crate::cases::{Case, CaseName, Expect, Word, run};
type Scenario = fn();
type Failure = std::convert::Infallible;
const CASES: &[Case<Scenario, (), Failure>] = &[
    Case {
        name: CaseName("percent_only_demand_never_prepares_brace_grammar"),
        word: Word::Succeeds,
        input: || percent_only_demand_never_prepares_brace_grammar as fn(),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("brace_parser_is_not_run_during_model_initialization"),
        word: Word::Succeeds,
        input: || brace_parser_is_not_run_during_model_initialization as fn(),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("model_reinitialization_clears_brace_results_for_replacement_sources"),
        word: Word::Succeeds,
        input: || model_reinitialization_clears_brace_results_for_replacement_sources as fn(),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("retained_format_specs_are_charged_by_the_existing_grammar_budget"),
        word: Word::Succeeds,
        input: || retained_format_specs_are_charged_by_the_existing_grammar_budget as fn(),
        expect: Expect::Value(()),
    },
];
fn subject(scenario: Scenario) -> Result<(), Failure> {
    scenario();
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
