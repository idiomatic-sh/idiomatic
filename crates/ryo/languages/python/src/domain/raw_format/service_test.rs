//! Independent demand, exact raw envelopes and retained per-token parser identities.
use super::service::{body, prepare, result, segments};
use crate::domain::format_syntax::service::flag_text;
use crate::domain::{
    brace_format::{service::prepare as brace, types::BraceFormatState},
    format_parsing::service::prepare as decoded,
    format_syntax::{
        interface::FormatSyntax,
        types::{FormatDialect, FormatParseResult, FormatParseState},
    },
    model::types::Declarations,
};
use crate::fixtures::reading::{initialize_model, parsed};
use crate::outbound::format_syntax::adapters::{
    ruff_literal_braceformat::grammar::RuffLiteralBraceFormat,
    ruff_literal_cformat::grammar::RuffLiteralCFormat,
};
use ryo::languages::{self, ByteSlice, Symbols, Tree};
use std::sync::atomic::{AtomicUsize, Ordering};

fn raw_format_envelopes_borrow_exact_bodies() {
    for (token, expected) in [
        ("''", Some("")),
        ("\"\"", Some("")),
        ("r'%d'", Some("%d")),
        ("R\"%s\"", Some("%s")),
        ("u'%c'", Some("%c")),
        ("U\"%x\"", Some("%x")),
        ("'''%d\r\n%s'''", Some("%d\r\n%s")),
        ("r\"\"\"\\x25d\"\"\"", Some("\\x25d")),
        ("'é\\\n%d'", Some("é\\\n%d")),
        ("", None),
        ("'", None),
        ("'''", None),
        ("'%d\"", None),
        ("b'%d'", None),
        ("f'%d'", None),
        ("t'%d'", None),
        ("rr'%d'", None),
    ] {
        assert_eq!(
            body(crate::domain::raw_format::types::RawFormatToken(token)).map(|body| body.0),
            expected,
            "{token:?}"
        );
    }
}

static PARSES: AtomicUsize = AtomicUsize::new(0);
struct Counting;
impl FormatSyntax for Counting {
    const DECODED_GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity =
        crate::domain::format_syntax::types::GrammarIdentity("test-percent-decoded");
    const RAW_GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity =
        crate::domain::format_syntax::types::GrammarIdentity("test-percent-raw");

    fn parse(
        input: crate::domain::format_syntax::types::FormatInput<'_>,
        dialect: FormatDialect,
    ) -> FormatParseResult {
        let _previous_count = PARSES.fetch_add(1, Ordering::SeqCst);
        RuffLiteralCFormat::parse(input, dialect)
    }
}

fn raw_format_cache_is_lazy_shared_and_reset_independently() {
    PARSES.store(0, Ordering::SeqCst);
    let mut file = parsed(b"'\\x25+d' '%+d'\n'%+d'\n").expect("source parses");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    assert_eq!(model.raw_format.borrow().state, FormatParseState::Fresh);
    assert_eq!(PARSES.load(Ordering::SeqCst), 0);
    let index = prepare::<Counting>(
        &tree,
        &model.raw_format,
        &mut file.symbols,
        crate::domain::semantic_syntax::service::positions(&tree),
        |at, symbols| {
            (crate::domain::semantic_syntax::service::visibility(&tree, &model, at, symbols)
                == crate::domain::semantic_syntax::types::Visibility::Visible)
                .into()
        },
    );
    assert_eq!(PARSES.load(Ordering::SeqCst), 2);
    assert_eq!(index.values.len(), 2);
    assert_eq!(index.occurrences.len(), 2);
    assert_eq!(index.segments.len(), 3);
    assert_ne!(index.segments[1].token, index.segments[2].token);
    assert_eq!(index.segments[1].cached, index.segments[2].cached);
    assert!(matches!(result(&index, &index.segments[1]),
        Some(FormatParseResult::Parsed(parsed)) if flag_text(parsed.fields[0].flags).0 == "+"));
    assert_eq!(model.format_parsing.borrow().state, FormatParseState::Fresh);
    assert_eq!(model.brace_format.borrow().state, BraceFormatState::Fresh);
    drop(index);
    drop(prepare::<Counting>(
        &tree,
        &model.raw_format,
        &mut file.symbols,
        crate::domain::semantic_syntax::service::positions(&tree),
        |at, symbols| {
            (crate::domain::semantic_syntax::service::visibility(&tree, &model, at, symbols)
                == crate::domain::semantic_syntax::types::Visibility::Visible)
                .into()
        },
    ));
    assert_eq!(
        PARSES.load(Ordering::SeqCst),
        2,
        "second raw reader uses the retained cache"
    );
    drop(decoded::<RuffLiteralCFormat>(
        &tree,
        &model.format_parsing,
        &mut file.symbols,
        crate::domain::semantic_syntax::service::positions(&tree),
        |at, symbols| {
            (crate::domain::semantic_syntax::service::visibility(&tree, &model, at, symbols)
                == crate::domain::semantic_syntax::types::Visibility::Visible)
                .into()
        },
    ));
    drop(brace::<RuffLiteralBraceFormat>(
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
    assert_eq!(PARSES.load(Ordering::SeqCst), 2);
    let mut next = parsed(b"'%x'\n").expect("replacement source");
    let next_tree = Tree {
        elements: &next.elements,
        source: ByteSlice(next.bytes),
        file: languages::FileId(0),
    };
    let capacity = model.raw_format.borrow().segments.capacity();
    initialize_model(&next_tree, &mut model);
    assert_eq!(model.raw_format.borrow().state, FormatParseState::Fresh);
    assert!(model.raw_format.borrow().segments.is_empty());
    assert!(model.raw_format.borrow().by_input.is_empty());
    assert_eq!(model.raw_format.borrow().segments.capacity(), capacity);
    drop(prepare::<Counting>(
        &next_tree,
        &model.raw_format,
        &mut next.symbols,
        crate::domain::semantic_syntax::service::positions(&next_tree),
        |at, symbols| {
            (crate::domain::semantic_syntax::service::visibility(&next_tree, &model, at, symbols)
                == crate::domain::semantic_syntax::types::Visibility::Visible)
                .into()
        },
    ));
    assert_eq!(PARSES.load(Ordering::SeqCst), 3);
}

fn raw_format_is_not_demanded_by_decoded_or_brace_readers() {
    let mut file = parsed(b"'%s'\n'{name}'\n").expect("source");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    drop(decoded::<RuffLiteralCFormat>(
        &tree,
        &model.format_parsing,
        &mut file.symbols,
        crate::domain::semantic_syntax::service::positions(&tree),
        |at, symbols| {
            (crate::domain::semantic_syntax::service::visibility(&tree, &model, at, symbols)
                == crate::domain::semantic_syntax::types::Visibility::Visible)
                .into()
        },
    ));
    assert_eq!(model.raw_format.borrow().state, FormatParseState::Fresh);
    drop(brace::<RuffLiteralBraceFormat>(
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
    assert_eq!(model.raw_format.borrow().state, FormatParseState::Fresh);
}

fn raw_format_malformed_segments_are_atomic_and_success_order_is_flattened() {
    let mut file = parsed(b"'%s' '%d%q' '%x%*.*d'\n").expect("source");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    let index = prepare::<RuffLiteralCFormat>(
        &tree,
        &model.raw_format,
        &mut file.symbols,
        crate::domain::semantic_syntax::service::positions(&tree),
        |at, symbols| {
            (crate::domain::semantic_syntax::service::visibility(&tree, &model, at, symbols)
                == crate::domain::semantic_syntax::types::Visibility::Visible)
                .into()
        },
    );
    assert_eq!(index.state, FormatParseState::Complete);
    let pieces = segments(&index, &index.occurrences[0]);
    assert_eq!(pieces.len(), 3);
    assert!(matches!(
        result(&index, &pieces[1]),
        Some(FormatParseResult::Malformed(_))
    ));
    assert_eq!(
        pieces[2].first_field.0, 1,
        "valid prefix in malformed segment contributes no fields"
    );
    let Some(FormatParseResult::Parsed(last)) = result(&index, &pieces[2]) else {
        panic!("valid last segment")
    };
    assert_eq!(last.fields.len(), 2);
    assert_eq!(last.summary.positional_consumption.0, 4);
    assert!(bool::from(index.occurrences[0].complete));
}

struct Unavailable;
impl FormatSyntax for Unavailable {
    const DECODED_GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity =
        crate::domain::format_syntax::types::GrammarIdentity("test-percent-decoded");
    const RAW_GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity =
        crate::domain::format_syntax::types::GrammarIdentity("test-percent-raw");

    fn parse(
        input: crate::domain::format_syntax::types::FormatInput<'_>,
        dialect: FormatDialect,
    ) -> FormatParseResult {
        if input.0 == "%s" {
            FormatParseResult::Unavailable
        } else {
            RuffLiteralCFormat::parse(input, dialect)
        }
    }
}

fn raw_format_unavailable_segment_withholds_precise_global_order() {
    let mut file = parsed(b"'%s' '%d'\n").expect("source");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    let index = prepare::<Unavailable>(
        &tree,
        &model.raw_format,
        &mut file.symbols,
        crate::domain::semantic_syntax::service::positions(&tree),
        |at, symbols| {
            (crate::domain::semantic_syntax::service::visibility(&tree, &model, at, symbols)
                == crate::domain::semantic_syntax::types::Visibility::Visible)
                .into()
        },
    );
    assert_eq!(index.state, FormatParseState::Partial);
    assert!(!bool::from(index.occurrences[0].complete));
    assert!(matches!(
        result(&index, &index.segments[0]),
        Some(FormatParseResult::Unavailable)
    ));
    assert!(matches!(
        result(&index, &index.segments[1]),
        Some(FormatParseResult::Parsed(_))
    ));
}

fn raw_format_missing_token_text_is_not_an_empty_valid_body() {
    let file = parsed(b"'%d'\n").expect("source");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    let index = prepare::<RuffLiteralCFormat>(
        &tree,
        &model.raw_format,
        &mut Symbols::default(),
        crate::domain::semantic_syntax::service::positions(&tree),
        |at, symbols| {
            (crate::domain::semantic_syntax::service::visibility(&tree, &model, at, symbols)
                == crate::domain::semantic_syntax::types::Visibility::Visible)
                .into()
        },
    );
    assert_eq!(index.state, FormatParseState::Partial);
    assert!(index.values.is_empty());
    assert!(!bool::from(index.occurrences[0].complete));
    assert!(index.segments[0].cached.is_none());
}

use crate::cases::{Case, CaseName, Expect, Word, run};
type Scenario = fn();
type Failure = std::convert::Infallible;
const CASES: &[Case<Scenario, (), Failure>] = &[
    Case {
        name: CaseName("raw_format_envelopes_borrow_exact_bodies"),
        word: Word::Succeeds,
        input: || raw_format_envelopes_borrow_exact_bodies as fn(),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("raw_format_cache_is_lazy_shared_and_reset_independently"),
        word: Word::Succeeds,
        input: || raw_format_cache_is_lazy_shared_and_reset_independently as fn(),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("raw_format_is_not_demanded_by_decoded_or_brace_readers"),
        word: Word::Succeeds,
        input: || raw_format_is_not_demanded_by_decoded_or_brace_readers as fn(),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("raw_format_malformed_segments_are_atomic_and_success_order_is_flattened"),
        word: Word::Succeeds,
        input: || raw_format_malformed_segments_are_atomic_and_success_order_is_flattened as fn(),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("raw_format_unavailable_segment_withholds_precise_global_order"),
        word: Word::Succeeds,
        input: || raw_format_unavailable_segment_withholds_precise_global_order as fn(),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("raw_format_missing_token_text_is_not_an_empty_valid_body"),
        word: Word::Succeeds,
        input: || raw_format_missing_token_text_is_not_an_empty_valid_body as fn(),
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
