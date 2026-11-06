//! Certify the actual parser's end state, and reject token coverage without that proof.

use super::adapter::SyntaxRuffPythonParserAdapter;
use super::lexical::{complete, covered};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use crate::outbound::syntax::fixtures::contract::elements_of;
use crate::outbound::syntax::interface::SyntaxPort as _;
use crate::outbound::syntax::types::{PythonSourceKind, SourceText};
use crate::settings::{PythonParserSettings, PythonTargetVersion};
use ruff_python_ast::PySourceType;
use ruff_python_ast::token::{Token as RuffToken, TokenFlags, TokenKind};
use ruff_python_parser::{ParseOptions, parse_unchecked};
use ruff_text_size::{TextRange, TextSize};
use ryo::languages::{self, ByteSlice, Elements, Symbols, TreeImpact};

fn unclosed_delimiters_preserve_fully_read_comments() {
    for source in [
        "x= (\n",
        "if ( # retained comment\n",
        "items = [ # retained comment\n",
        "mapping = { # retained comment\n",
        "if ready:\n    call( # retained comment\n",
    ] {
        let parsed = parse_unchecked(source, ParseOptions::from(PySourceType::Python));
        assert!(!parsed.errors().is_empty(), "{source:?}");
        assert!(
            complete(&parsed, source.as_bytes()),
            "{source:?}: {:?}",
            parsed.errors()
        );
    }
}

fn complete_byte_coverage_does_not_override_lost_lexer_state() {
    for source in [
        "value = 'unterminated\n# not certainly a comment\n",
        "value = '''unterminated\n# inside a string\n",
        "value = f\"{unfinished\n",
        "value = f\"unterminated",
        "value = (\\\n",
        "\\\n",
        "value = (\"unterminated\n# not certainly a comment\n",
    ] {
        let parsed = parse_unchecked(source, ParseOptions::from(PySourceType::Python));
        assert!(!parsed.errors().is_empty(), "{source:?}");
        assert!(
            !complete(&parsed, source.as_bytes()),
            "{source:?}: {:?}",
            parsed.errors()
        );
    }
}

fn truncated_or_invalid_token_stream_cannot_certify_absence() {
    let source = "first = 1\n# retained\nsecond = 2\n";
    let parsed = parse_unchecked(source, ParseOptions::from(PySourceType::Python));
    assert!(complete(&parsed, source.as_bytes()));
    let stop = parsed
        .tokens()
        .iter()
        .position(|token| token.kind() == TokenKind::Comment)
        .unwrap();
    assert!(!covered(&parsed.tokens()[..stop], source.as_bytes(), false));
    let invalid = RuffToken::new(
        TokenKind::Unknown,
        TextRange::empty(TextSize::new(0)),
        TokenFlags::empty(),
    );
    assert!(
        !covered(&[invalid], b"", false),
        "a zero-width error still needs end-state evidence"
    );
    assert!(
        !covered(&[invalid], source.as_bytes(), true),
        "an interior error cannot be an EOF marker"
    );
}

fn parser_errors_distinguish_complete_tokens_from_lexical_failure() {
    let impacts = |source: &str| {
        let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
        let mut builder =
            languages::table_builder(&mut elements, &mut symbols, ByteSlice(source.as_bytes()));
        SyntaxRuffPythonParserAdapter::default().read(
            SourceText(source.as_bytes()),
            PythonSourceKind::Python,
            &PythonParserSettings::new(PythonTargetVersion::Py310),
            &mut builder,
        );
        let impacts: Vec<_> = languages::diagnostics(&elements)
            .map(|diagnostic| diagnostic.impact)
            .collect();
        (
            impacts,
            languages::has_incomplete_syntax(&elements),
            languages::has_incomplete_lexical_stream(&elements),
            languages::syntax_acceptance(&elements),
        )
    };

    for prefix in ["ur", "ru", "UR", "bu"] {
        let source = format!("value = {prefix}\"text\" # retained comment\n");
        let (impacts, incomplete_syntax, incomplete_lexical, acceptance) = impacts(&source);
        assert!(!impacts.is_empty(), "{prefix} must be rejected for PY310");
        assert!(
            impacts
                .iter()
                .all(|impact| *impact == TreeImpact::LexicallyComplete),
            "{prefix}: {impacts:?}"
        );
        assert!(incomplete_syntax);
        assert!(!incomplete_lexical);
        assert_eq!(acceptance, languages::SyntaxAcceptance::Rejected);
    }

    for source in ["x= (\n", "if ( # retained comment\n"] {
        let (seen, syntax, lexical, acceptance) = impacts(source);
        assert!(!seen.is_empty());
        assert!(
            seen.iter()
                .all(|impact| *impact == TreeImpact::LexicallyComplete)
        );
        assert!(syntax);
        assert!(!lexical);
        assert_eq!(acceptance, languages::SyntaxAcceptance::Rejected);
    }

    for source in [
        "value = 'unterminated\n",
        "if ready:\n    work(\"unfinished\n",
    ] {
        let (impacts, incomplete_syntax, incomplete_lexical, acceptance) = impacts(source);
        assert!(!impacts.is_empty(), "source must report a parse error");
        assert!(incomplete_syntax);
        assert!(
            incomplete_lexical,
            "{source:?} must not certify lexical facts"
        );
        assert_eq!(acceptance, languages::SyntaxAcceptance::Rejected);
    }
}

fn parser_refusal_with_complete_tokens_keeps_lexical_facts_available() {
    let source = "try:\n    work()\nelse:\n    recover()\n";
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder =
        languages::table_builder(&mut elements, &mut symbols, ByteSlice(source.as_bytes()));
    SyntaxRuffPythonParserAdapter::default().read(
        SourceText(source.as_bytes()),
        PythonSourceKind::Python,
        &PythonParserSettings::default(),
        &mut builder,
    );
    let diagnostics: Vec<_> = languages::diagnostics(&elements).collect();
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == "else-without-except")
    );
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.phase == languages::DiagnosticPhase::Parse
            && diagnostic.impact == TreeImpact::LexicallyComplete
    }));
    assert!(languages::has_incomplete_syntax(&elements));
    assert!(!languages::has_incomplete_lexical_stream(&elements));
}

fn invalid_utf8_never_certifies_a_complete_lexical_stream() {
    let source = b"x = ( # before invalid byte\n\xff\n";
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = languages::table_builder(&mut elements, &mut symbols, ByteSlice(source));
    SyntaxRuffPythonParserAdapter::default().read(
        SourceText(source),
        PythonSourceKind::Python,
        &PythonParserSettings::default(),
        &mut builder,
    );
    assert!(languages::has_incomplete_syntax(&elements));
    assert!(languages::has_incomplete_lexical_stream(&elements));
}

fn invalid_syntax_keeps_actual_comment_tokens() {
    let source = "if ( # retained comment\n";
    let elements = elements_of(&SyntaxRuffPythonParserAdapter::default(), source);
    assert!(elements.iter().any(|(kind, text)| {
        *kind == Kind::Token(Token::Comment) && text == "# retained comment"
    }));
}

/// Which of the file's cases a row runs.
#[derive(Clone, Copy, Debug)]
enum Body {
    /// The case `unclosed_delimiters_preserve_fully_read_comments`.
    UnclosedDelimitersPreserveFullyReadComments,
    /// The case `complete_byte_coverage_does_not_override_lost_lexer_state`.
    CompleteByteCoverageDoesNotOverrideLostLexerState,
    /// The case `truncated_or_invalid_token_stream_cannot_certify_absence`.
    TruncatedOrInvalidTokenStreamCannotCertifyAbsence,
    /// The case `parser_errors_distinguish_complete_tokens_from_lexical_failure`.
    ParserErrorsDistinguishCompleteTokensFromLexicalFailure,
    /// The case `parser_refusal_with_complete_tokens_keeps_lexical_facts_available`.
    ParserRefusalWithCompleteTokensKeepsLexicalFactsAvailable,
    /// The case `invalid_utf8_never_certifies_a_complete_lexical_stream`.
    InvalidUtf8NeverCertifiesACompleteLexicalStream,
    /// The case `invalid_syntax_keeps_actual_comment_tokens`.
    InvalidSyntaxKeepsActualCommentTokens,
}

/// The rows of the cases whose bodies assert for themselves.
const CASES: &[Case<Body, (), ()>] = &[
    Case {
        name: CaseName("unclosed_delimiters_preserve_fully_read_comments"),
        word: Word::Succeeds,
        input: || Body::UnclosedDelimitersPreserveFullyReadComments,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("complete_byte_coverage_does_not_override_lost_lexer_state"),
        word: Word::Succeeds,
        input: || Body::CompleteByteCoverageDoesNotOverrideLostLexerState,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("truncated_or_invalid_token_stream_cannot_certify_absence"),
        word: Word::Succeeds,
        input: || Body::TruncatedOrInvalidTokenStreamCannotCertifyAbsence,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("parser_errors_distinguish_complete_tokens_from_lexical_failure"),
        word: Word::Succeeds,
        input: || Body::ParserErrorsDistinguishCompleteTokensFromLexicalFailure,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("parser_refusal_with_complete_tokens_keeps_lexical_facts_available"),
        word: Word::Succeeds,
        input: || Body::ParserRefusalWithCompleteTokensKeepsLexicalFactsAvailable,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("invalid_utf8_never_certifies_a_complete_lexical_stream"),
        word: Word::Succeeds,
        input: || Body::InvalidUtf8NeverCertifiesACompleteLexicalStream,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("invalid_syntax_keeps_actual_comment_tokens"),
        word: Word::Succeeds,
        input: || Body::InvalidSyntaxKeepsActualCommentTokens,
        expect: Expect::Value(()),
    },
];

/// Runs the body of one case.
fn body(given: Body) -> Result<(), ()> {
    match given {
        Body::UnclosedDelimitersPreserveFullyReadComments => {
            unclosed_delimiters_preserve_fully_read_comments()
        }
        Body::CompleteByteCoverageDoesNotOverrideLostLexerState => {
            complete_byte_coverage_does_not_override_lost_lexer_state()
        }
        Body::TruncatedOrInvalidTokenStreamCannotCertifyAbsence => {
            truncated_or_invalid_token_stream_cannot_certify_absence()
        }
        Body::ParserErrorsDistinguishCompleteTokensFromLexicalFailure => {
            parser_errors_distinguish_complete_tokens_from_lexical_failure()
        }
        Body::ParserRefusalWithCompleteTokensKeepsLexicalFactsAvailable => {
            parser_refusal_with_complete_tokens_keeps_lexical_facts_available()
        }
        Body::InvalidUtf8NeverCertifiesACompleteLexicalStream => {
            invalid_utf8_never_certifies_a_complete_lexical_stream()
        }
        Body::InvalidSyntaxKeepsActualCommentTokens => invalid_syntax_keeps_actual_comment_tokens(),
    }
    Ok(())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, body);
}
