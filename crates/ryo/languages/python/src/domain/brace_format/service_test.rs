//! Independent demand, reset and literal-input guards for brace-format parsing.
use super::service::{prepare, state};
use super::types::BraceFormatState;
use crate::domain::format_parsing::service::prepare as prepare_percent;
use crate::domain::model::types::Declarations;
use crate::fixtures::reading::{initialize_model, parsed};
use crate::outbound::format_syntax::adapters::ruff_literal_braceformat::grammar::RuffLiteralBraceFormat;
use crate::outbound::format_syntax::adapters::ruff_literal_cformat::grammar::RuffLiteralCFormat;
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
    let _percent = prepare_percent::<RuffLiteralCFormat>(
        &tree,
        &model.format_parsing,
        &mut file.symbols,
        crate::domain::semantic_syntax::service::positions(&tree),
        |at, symbols| {
            (crate::domain::semantic_syntax::service::visibility(&tree, &model, at, symbols)
                == crate::domain::semantic_syntax::types::Visibility::Visible)
                .into()
        },
    );
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
        &model.brace_format,
        &mut file.symbols,
        crate::domain::semantic_syntax::service::positions(&tree),
        |at, symbols| {
            (crate::domain::semantic_syntax::service::visibility(&tree, &model, at, symbols)
                == crate::domain::semantic_syntax::types::Visibility::Visible)
                .into()
        },
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
        &model.brace_format,
        &mut first.symbols,
        crate::domain::semantic_syntax::service::positions(&first_tree),
        |at, symbols| {
            (crate::domain::semantic_syntax::service::visibility(&first_tree, &model, at, symbols)
                == crate::domain::semantic_syntax::types::Visibility::Visible)
                .into()
        },
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
    let index = prepare::<RuffLiteralBraceFormat>(
        &second_tree,
        &model.brace_format,
        &mut second.symbols,
        crate::domain::semantic_syntax::service::positions(&second_tree),
        |at, symbols| {
            (crate::domain::semantic_syntax::service::visibility(&second_tree, &model, at, symbols)
                == crate::domain::semantic_syntax::types::Visibility::Visible)
                .into()
        },
    );
    assert!(
        matches!(&index.values[0].result, super::types::BraceFormatResult::Parsed(parsed) if parsed.fields.is_empty())
    );
}

use crate::cases::{Case, CaseName, Expect, Word, run};
type Scenario = fn();
type Failure = std::convert::Infallible;
const CASES: &[Case<Scenario, (), Failure>] = &[
    Case {
        name: CaseName("percent_only_demand_never_prepares_brace_grammar"),
        word: Word::Succeeds,
        input: || percent_only_demand_never_prepares_brace_grammar,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("brace_parser_is_not_run_during_model_initialization"),
        word: Word::Succeeds,
        input: || brace_parser_is_not_run_during_model_initialization,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("model_reinitialization_clears_brace_results_for_replacement_sources"),
        word: Word::Succeeds,
        input: || model_reinitialization_clears_brace_results_for_replacement_sources,
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
