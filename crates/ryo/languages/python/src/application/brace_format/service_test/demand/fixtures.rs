//! The scenarios of the group: each runs its subject and records what it observed.
use crate::application::brace_format::service::prepare;
use crate::application::format_parsing::service::prepare as prepare_percent;
use crate::domain::brace_format::service::state;
use crate::domain::model::types::Declarations;
use crate::fixtures::reading::{initialize_model, parsed};
use crate::outbound::brace_format::adapters::ruff_literal_braceformat::grammar::RuffLiteralBraceFormat;
use crate::outbound::format_syntax::adapters::ruff_literal_cformat::grammar::RuffLiteralCFormat;
use ryo::languages::{ByteSlice, Tree};

pub(super) fn percent_only_demand_never_prepares_brace_grammar() -> String {
    let mut observed: Vec<String> = Vec::new();
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
    observed.push(format!("{:?}", model.brace_format.borrow().state));
    observed.join("|")
}

pub(super) fn brace_parser_is_not_run_during_model_initialization() -> String {
    let mut observed: Vec<String> = Vec::new();
    let mut file = parsed(b"'{name}'\n").expect("source parses");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: ryo::languages::FileId(0),
    };
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    observed.push(format!("{:?}", model.brace_format.borrow().state));
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
    observed.push(format!("{:?}", state(&model.brace_format.borrow())));
    observed.join("|")
}

pub(super) fn model_reinitialization_clears_brace_results_for_replacement_sources() -> String {
    let mut observed: Vec<String> = Vec::new();
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
    observed.push(format!(
        "{:?}",
        model.brace_format.borrow().occurrences.len()
    ));

    let mut second = parsed(b"'{{}}'\n").expect("second parses");
    let second_tree = Tree {
        elements: &second.elements,
        source: ByteSlice(second.bytes),
        file: ryo::languages::FileId(0),
    };
    initialize_model(&second_tree, &mut model);
    observed.push(format!("{:?}", model.brace_format.borrow().state));
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
    observed.push(format!("{}", matches!(&index.values[0].result, crate::domain::brace_format::types::BraceFormatResult::Parsed(parsed) if parsed.fields.is_empty())));
    observed.join("|")
}
