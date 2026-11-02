//! Certify lexical facts from the parser's consumed token stream and lexer state errors.
use ruff_python_ast::Mod;
use ruff_python_ast::token::{Token, TokenKind};
use ruff_python_parser::{LexicalErrorType, ParseErrorType, Parsed};
use ruff_text_size::Ranged as _;

/// Parser recovery does not by itself make comment or indentation facts incomplete.
/// The parser consumes its token source through EOF. An unclosed delimiter reports
/// a synthetic zero-width lexer error there, after all source tokens were read.
/// Other lexer failures can lose string, interpolation or continuation state and
/// remain incomplete even when their token ranges happen to cover every byte.
pub(super) fn complete(parsed: &Parsed<Mod>, source: &[u8]) -> bool {
    let mut delimiter_eof = false;
    for error in parsed.errors() {
        match &error.error {
            ParseErrorType::Lexical(LexicalErrorType::Eof)
                if error.location.is_empty() && error.location.end().to_usize() == source.len() =>
            {
                delimiter_eof = true;
            }
            ParseErrorType::Lexical(_)
            | ParseErrorType::FStringError(_)
            | ParseErrorType::TStringError(_) => return false,
            _ => {}
        }
    }
    if delimiter_eof && !has_unclosed_delimiter(parsed.tokens()) {
        return false;
    }
    covered(parsed.tokens(), source, delimiter_eof)
}

/// A delimiter witness distinguishes the EOF marker from failed continuations.
/// Mismatched delimiters do not certify the lexer's nesting state.
fn has_unclosed_delimiter(tokens: &[Token]) -> bool {
    let mut expected = Vec::new();
    for token in tokens {
        match token.kind() {
            TokenKind::Lpar => expected.push(TokenKind::Rpar),
            TokenKind::Lsqb => expected.push(TokenKind::Rsqb),
            TokenKind::Lbrace => expected.push(TokenKind::Rbrace),
            closing @ (TokenKind::Rpar | TokenKind::Rsqb | TokenKind::Rbrace) => {
                if expected.pop() != Some(closing) {
                    return false;
                }
            }
            _ => {}
        }
    }
    !expected.is_empty()
}

/// Ordered, bounded tokens must cover every non-whitespace source byte.
/// Only a separately certified synthetic EOF error token may be ignored.
pub(super) fn covered(tokens: &[Token], source: &[u8], delimiter_eof: bool) -> bool {
    if u32::try_from(source.len()).is_err() {
        return false;
    }
    let mut previous_end = 0_u32;
    for (index, token) in tokens.iter().enumerate() {
        let start = token.start().to_u32();
        let end = token.end().to_u32();
        let start_index = usize::try_from(start).unwrap_or(usize::MAX);
        let end_index = usize::try_from(end).unwrap_or(usize::MAX);
        if start < previous_end
            || start > end
            || end_index > source.len()
            || !source
                .get(usize::try_from(previous_end).unwrap_or(usize::MAX)..start_index)
                .is_some_and(|gap| gap.iter().all(u8::is_ascii_whitespace))
        {
            return false;
        }
        let kind = super::schemas::token_kind(token.kind());
        if kind
            == crate::domain::syntax::types::Kind::Token(
                crate::domain::tokens::types::Token::Errortoken,
            )
            && !(delimiter_eof
                && token.kind() == TokenKind::Unknown
                && start == end
                && end_index == source.len())
        {
            return false;
        }
        if token.kind() == TokenKind::EndOfFile
            && (index + 1 != tokens.len() || end_index != source.len())
        {
            return false;
        }
        previous_end = end;
    }
    source
        .get(usize::try_from(previous_end).unwrap_or(usize::MAX)..)
        .is_some_and(|gap| gap.iter().all(u8::is_ascii_whitespace))
}
