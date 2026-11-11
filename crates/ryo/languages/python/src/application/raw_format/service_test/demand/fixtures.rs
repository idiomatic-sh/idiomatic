//! The support of the group's tests.
use crate::application::brace_format::service::prepare as brace;
use crate::application::format_parsing::service::prepare as decoded;
use crate::application::raw_format::service::prepare;
use crate::domain::format_syntax::service::flag_text;
use crate::domain::format_syntax::types::FormatDialect;
use crate::domain::raw_format::service::{body, result, segments};
use crate::domain::{format_syntax::types::FormatParseResult, model::types::Declarations};
use crate::fixtures::reading::{initialize_model, parsed};
use crate::outbound::brace_format::adapters::ruff_literal_braceformat::grammar::RuffLiteralBraceFormat;
use crate::outbound::format_syntax::adapters::ruff_literal_cformat::grammar::RuffLiteralCFormat;
use crate::outbound::format_syntax::interface::FormatSyntaxPort;
use ryo::languages::{self, ByteSlice, Symbols, Tree};
use std::sync::atomic::{AtomicUsize, Ordering};

pub(super) static PARSES: AtomicUsize = AtomicUsize::new(0);

pub(super) struct Counting;

impl FormatSyntaxPort for Counting {
    const DECODED_GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity =
        crate::domain::format_syntax::types::GrammarIdentity("test-percent-decoded");
    const RAW_GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity =
        crate::domain::format_syntax::types::GrammarIdentity("test-percent-raw");

    fn parse(
        &self,
        input: crate::domain::format_syntax::types::FormatInput<'_>,
        dialect: FormatDialect,
    ) -> FormatParseResult {
        let _previous_count = PARSES.fetch_add(1, Ordering::SeqCst);
        RuffLiteralCFormat.parse(input, dialect)
    }
}

pub(super) struct Unavailable;

impl FormatSyntaxPort for Unavailable {
    const DECODED_GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity =
        crate::domain::format_syntax::types::GrammarIdentity("test-percent-decoded");
    const RAW_GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity =
        crate::domain::format_syntax::types::GrammarIdentity("test-percent-raw");

    fn parse(
        &self,
        input: crate::domain::format_syntax::types::FormatInput<'_>,
        dialect: FormatDialect,
    ) -> FormatParseResult {
        if input.0 == "%s" {
            FormatParseResult::Unavailable
        } else {
            RuffLiteralCFormat.parse(input, dialect)
        }
    }
}

pub(super) fn raw_format_envelopes_borrow_exact_bodies() -> String {
    let mut observed: Vec<String> = Vec::new();
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
        observed.push(format!(
            "{}",
            body(crate::domain::raw_format::types::RawFormatToken(token)).map(|body| body.0)
                == expected
        ));
    }
    observed.join("|")
}

pub(super) fn raw_format_cache_is_lazy_shared_and_reset_independently() -> String {
    let mut observed: Vec<String> = Vec::new();
    PARSES.store(0, Ordering::SeqCst);
    let mut file = parsed(b"'\\x25+d' '%+d'\n'%+d'\n").expect("source parses");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    observed.push(format!("{:?}", model.raw_format.borrow().state));
    observed.push(format!("{:?}", PARSES.load(Ordering::SeqCst)));
    let index = prepare(
        &Counting,
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
    observed.push(format!("{:?}", PARSES.load(Ordering::SeqCst)));
    observed.push(format!("{:?}", index.values.len()));
    observed.push(format!("{:?}", index.occurrences.len()));
    observed.push(format!("{:?}", index.segments.len()));
    observed.push(format!(
        "{}",
        index.segments[1].token != index.segments[2].token
    ));
    observed.push(format!("{:?}", index.segments[1].cached));
    observed.push(format!(
        "{}",
        matches!(result(&index, &index.segments[1]),
        Some(FormatParseResult::Parsed(parsed)) if flag_text(parsed.fields[0].flags).0 == "+")
    ));
    observed.push(format!("{:?}", model.format_parsing.borrow().state));
    observed.push(format!("{:?}", model.brace_format.borrow().state));
    drop(index);
    drop(prepare(
        &Counting,
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
    observed.push(format!("{:?}", PARSES.load(Ordering::SeqCst)));
    drop(decoded(
        &RuffLiteralCFormat,
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
    drop(brace(
        &RuffLiteralBraceFormat,
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
    observed.push(format!("{:?}", PARSES.load(Ordering::SeqCst)));
    let mut next = parsed(b"'%x'\n").expect("replacement source");
    let next_tree = Tree {
        elements: &next.elements,
        source: ByteSlice(next.bytes),
        file: languages::FileId(0),
    };
    let capacity = model.raw_format.borrow().segments.capacity();
    initialize_model(&next_tree, &mut model);
    observed.push(format!("{:?}", model.raw_format.borrow().state));
    observed.push(format!("{}", model.raw_format.borrow().segments.is_empty()));
    observed.push(format!("{}", model.raw_format.borrow().by_input.is_empty()));
    observed.push(format!(
        "{}",
        model.raw_format.borrow().segments.capacity() == capacity
    ));
    drop(prepare(
        &Counting,
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
    observed.push(format!("{:?}", PARSES.load(Ordering::SeqCst)));
    observed.join("|")
}

pub(super) fn raw_format_is_not_demanded_by_decoded_or_brace_readers() -> String {
    let mut observed: Vec<String> = Vec::new();
    let mut file = parsed(b"'%s'\n'{name}'\n").expect("source");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    drop(decoded(
        &RuffLiteralCFormat,
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
    observed.push(format!("{:?}", model.raw_format.borrow().state));
    drop(brace(
        &RuffLiteralBraceFormat,
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
    observed.push(format!("{:?}", model.raw_format.borrow().state));
    observed.join("|")
}

pub(super) fn raw_format_malformed_segments_are_atomic_and_success_order_is_flattened() -> String {
    let mut observed: Vec<String> = Vec::new();
    let mut file = parsed(b"'%s' '%d%q' '%x%*.*d'\n").expect("source");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    let index = prepare(
        &RuffLiteralCFormat,
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
    observed.push(format!("{:?}", index.state));
    let pieces = segments(&index, &index.occurrences[0]);
    observed.push(format!("{:?}", pieces.len()));
    observed.push(format!(
        "{}",
        matches!(
            result(&index, &pieces[1]),
            Some(FormatParseResult::Malformed(_))
        )
    ));
    observed.push(format!("{:?}", pieces[2].first_field.0));
    let Some(FormatParseResult::Parsed(last)) = result(&index, &pieces[2]) else {
        panic!("valid last segment")
    };
    observed.push(format!("{:?}", last.fields.len()));
    observed.push(format!("{:?}", last.summary.positional_consumption.0));
    observed.push(format!("{}", bool::from(index.occurrences[0].complete)));
    observed.join("|")
}

pub(super) fn raw_format_unavailable_segment_withholds_precise_global_order() -> String {
    let mut observed: Vec<String> = Vec::new();
    let mut file = parsed(b"'%s' '%d'\n").expect("source");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    let index = prepare(
        &Unavailable,
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
    observed.push(format!("{:?}", index.state));
    observed.push(format!("{}", !bool::from(index.occurrences[0].complete)));
    observed.push(format!(
        "{}",
        matches!(
            result(&index, &index.segments[0]),
            Some(FormatParseResult::Unavailable)
        )
    ));
    observed.push(format!(
        "{}",
        matches!(
            result(&index, &index.segments[1]),
            Some(FormatParseResult::Parsed(_))
        )
    ));
    observed.join("|")
}

pub(super) fn raw_format_missing_token_text_is_not_an_empty_valid_body() -> String {
    let mut observed: Vec<String> = Vec::new();
    let file = parsed(b"'%d'\n").expect("source");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    let index = prepare(
        &RuffLiteralCFormat,
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
    observed.push(format!("{:?}", index.state));
    observed.push(format!("{}", index.values.is_empty()));
    observed.push(format!("{}", !bool::from(index.occurrences[0].complete)));
    observed.push(format!("{}", index.segments[0].cached.is_none()));
    observed.join("|")
}
