//! Certify the actual parser's end state, and reject token coverage without that proof.
use super::lexical::{complete, covered};
use ruff_python_ast::PySourceType;
use ruff_python_ast::token::{Token, TokenFlags, TokenKind};
use ruff_python_parser::{ParseOptions, parse_unchecked};
use ruff_text_size::{TextRange, TextSize};

#[test]
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

#[test]
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

#[test]
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
    let invalid = Token::new(
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
