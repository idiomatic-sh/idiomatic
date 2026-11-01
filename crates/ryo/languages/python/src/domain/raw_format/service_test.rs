//! Independent demand, exact raw envelopes and retained per-token parser identities.
use super::service::{body, prepare, result, segments};
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
    ruff_literal_braceformat::RuffLiteralBraceFormat, ruff_literal_cformat::RuffLiteralCFormat,
};
use ryo::languages::{self, ByteSlice, Symbols, Tree};
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
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
        assert_eq!(body(token), expected, "{token:?}");
    }
}

static PARSES: AtomicUsize = AtomicUsize::new(0);
struct Counting;
impl FormatSyntax for Counting {
    fn parse(input: &str, dialect: FormatDialect) -> FormatParseResult {
        let _ = PARSES.fetch_add(1, Ordering::SeqCst);
        RuffLiteralCFormat::parse(input, dialect)
    }
}

#[test]
fn raw_format_cache_is_lazy_shared_and_reset_independently() {
    PARSES.store(0, Ordering::SeqCst);
    let mut file = parsed(b"'\\x25d' '%d'\n'%d'\n").expect("source parses");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    assert_eq!(model.raw_format.borrow().state, FormatParseState::Fresh);
    assert_eq!(PARSES.load(Ordering::SeqCst), 0);
    let index = prepare::<Counting>(&tree, &model, &mut file.symbols);
    assert_eq!(PARSES.load(Ordering::SeqCst), 2);
    assert_eq!(index.values.len(), 2);
    assert_eq!(index.occurrences.len(), 2);
    assert_eq!(index.segments.len(), 3);
    assert_ne!(index.segments[1].token, index.segments[2].token);
    assert_eq!(index.segments[1].cached, index.segments[2].cached);
    assert_eq!(model.format_parsing.borrow().state, FormatParseState::Fresh);
    assert_eq!(model.brace_format.borrow().state, BraceFormatState::Fresh);
    drop(index);
    drop(prepare::<Counting>(&tree, &model, &mut file.symbols));
    assert_eq!(
        PARSES.load(Ordering::SeqCst),
        2,
        "second raw reader uses the retained cache"
    );
    drop(decoded::<RuffLiteralCFormat>(
        &tree,
        &model,
        &mut file.symbols,
    ));
    drop(brace::<RuffLiteralBraceFormat>(
        &tree,
        &model,
        &mut file.symbols,
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
    drop(prepare::<Counting>(&next_tree, &model, &mut next.symbols));
    assert_eq!(PARSES.load(Ordering::SeqCst), 3);
}

#[test]
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
        &model,
        &mut file.symbols,
    ));
    assert_eq!(model.raw_format.borrow().state, FormatParseState::Fresh);
    drop(brace::<RuffLiteralBraceFormat>(
        &tree,
        &model,
        &mut file.symbols,
    ));
    assert_eq!(model.raw_format.borrow().state, FormatParseState::Fresh);
}

#[test]
fn raw_format_malformed_segments_are_atomic_and_success_order_is_flattened() {
    let mut file = parsed(b"'%s' '%d%q' '%x%*.*d'\n").expect("source");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    let index = prepare::<RuffLiteralCFormat>(&tree, &model, &mut file.symbols);
    assert_eq!(index.state, FormatParseState::Complete);
    let pieces = segments(&index, &index.occurrences[0]);
    assert_eq!(pieces.len(), 3);
    assert!(matches!(
        result(&index, &pieces[1]),
        Some(FormatParseResult::Malformed(_))
    ));
    assert_eq!(
        pieces[2].first_field, 1,
        "valid prefix in malformed segment contributes no fields"
    );
    let Some(FormatParseResult::Parsed(last)) = result(&index, &pieces[2]) else {
        panic!("valid last segment")
    };
    assert_eq!(last.fields.len(), 2);
    assert_eq!(last.summary.positional_consumption, 4);
    assert!(index.occurrences[0].complete);
}

struct Unavailable;
impl FormatSyntax for Unavailable {
    fn parse(input: &str, dialect: FormatDialect) -> FormatParseResult {
        if input == "%s" {
            FormatParseResult::Unavailable
        } else {
            RuffLiteralCFormat::parse(input, dialect)
        }
    }
}

#[test]
fn raw_format_unavailable_segment_withholds_precise_global_order() {
    let mut file = parsed(b"'%s' '%d'\n").expect("source");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    let index = prepare::<Unavailable>(&tree, &model, &mut file.symbols);
    assert_eq!(index.state, FormatParseState::Partial);
    assert!(!index.occurrences[0].complete);
    assert!(matches!(
        result(&index, &index.segments[0]),
        Some(FormatParseResult::Unavailable)
    ));
    assert!(matches!(
        result(&index, &index.segments[1]),
        Some(FormatParseResult::Parsed(_))
    ));
}

#[test]
fn raw_format_missing_token_text_is_not_an_empty_valid_body() {
    let file = parsed(b"'%d'\n").expect("source");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    let index = prepare::<RuffLiteralCFormat>(&tree, &model, &mut Symbols::default());
    assert_eq!(index.state, FormatParseState::Partial);
    assert!(index.values.is_empty());
    assert!(!index.occurrences[0].complete);
    assert!(index.segments[0].cached.is_none());
}
