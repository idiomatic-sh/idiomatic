//! How the language shows each kind of its trees that is a token.

use crate::domain::syntax::constants::{KEYWORD, LEXICAL, PUNCTUATION};
use crate::domain::tokens::types::Token;
use ryo::languages::{Sample, SampleText, Why};

/// How the language shows a token kind of its trees: the source whose tree holds it, or the reason no relation models it.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "one arm for each kind of the grammar and no wildcard, so a kind the grammar gains fails to compile until it is shown; split, the match would lose that check"
)]
pub(super) const fn sample(kind: Token) -> Sample {
    match kind {
        Token::Comment => Sample::Token(SampleText("//")),
        Token::Ident => Sample::Token(SampleText("mod a;")),
        Token::Dollar
        | Token::Semicolon
        | Token::Comma
        | Token::LParen
        | Token::RParen
        | Token::LCurly
        | Token::RCurly
        | Token::LBrack
        | Token::RBrack
        | Token::At
        | Token::Pound
        | Token::Tilde
        | Token::Underscore
        | Token::Dot
        | Token::Colon
        | Token::Colon2
        | Token::FatArrow
        | Token::ThinArrow
        | Token::Byte
        | Token::ByteString
        | Token::Char
        | Token::CString
        | Token::FloatNumber
        | Token::IntNumber
        | Token::String
        | Token::Error
        | Token::Frontmatter
        | Token::LifetimeIdent
        | Token::Shebang
        | Token::Whitespace => Sample::Unread(Why(LEXICAL)),
        Token::LAngle
        | Token::RAngle
        | Token::Question
        | Token::Amp
        | Token::Pipe
        | Token::Plus
        | Token::Star
        | Token::Slash
        | Token::Caret
        | Token::Percent
        | Token::Dot2
        | Token::Dot3
        | Token::Dot2eq
        | Token::Eq
        | Token::Eq2
        | Token::Bang
        | Token::Neq
        | Token::Minus
        | Token::Lteq
        | Token::Gteq
        | Token::Pluseq
        | Token::Minuseq
        | Token::Pipeeq
        | Token::Ampeq
        | Token::Careteq
        | Token::Slasheq
        | Token::Stareq
        | Token::Percenteq
        | Token::Amp2
        | Token::Pipe2
        | Token::Shl
        | Token::Shr
        | Token::Shleq
        | Token::Shreq => Sample::Unread(Why(PUNCTUATION)),
        Token::SelfTypeKw
        | Token::AbstractKw
        | Token::AsKw
        | Token::BecomeKw
        | Token::BoxKw
        | Token::BreakKw
        | Token::ConstKw
        | Token::ContinueKw
        | Token::CrateKw
        | Token::DoKw
        | Token::ElseKw
        | Token::EnumKw
        | Token::ExternKw
        | Token::FalseKw
        | Token::FinalKw
        | Token::FnKw
        | Token::ForKw
        | Token::IfKw
        | Token::ImplKw
        | Token::InKw
        | Token::LetKw
        | Token::LoopKw
        | Token::MacroKw
        | Token::MatchKw
        | Token::ModKw
        | Token::MoveKw
        | Token::MutKw
        | Token::OverrideKw
        | Token::PrivKw
        | Token::PubKw
        | Token::RefKw
        | Token::ReturnKw
        | Token::SelfKw
        | Token::StaticKw
        | Token::StructKw
        | Token::SuperKw
        | Token::TraitKw
        | Token::TrueKw
        | Token::TypeKw
        | Token::TypeofKw
        | Token::UnsafeKw
        | Token::UnsizedKw
        | Token::UseKw
        | Token::VirtualKw
        | Token::WhereKw
        | Token::WhileKw
        | Token::YieldKw
        | Token::AsmKw
        | Token::AsyncKw
        | Token::AttSyntaxKw
        | Token::AutoKw
        | Token::AwaitKw
        | Token::BuiltinKw
        | Token::CfgAttrKw
        | Token::CfgKw
        | Token::ClobberAbiKw
        | Token::DefaultKw
        | Token::DerefKw
        | Token::DynKw
        | Token::FormatArgsKw
        | Token::GenKw
        | Token::GlobalAsmKw
        | Token::IncludeBytesKw
        | Token::InlateoutKw
        | Token::InoutKw
        | Token::IsKw
        | Token::LabelKw
        | Token::LateoutKw
        | Token::MacroRulesKw
        | Token::MayUnwindKw
        | Token::NakedAsmKw
        | Token::NomemKw
        | Token::NoreturnKw
        | Token::NostackKw
        | Token::OffsetOfKw
        | Token::OptionsKw
        | Token::OutKw
        | Token::PatternTypeKw
        | Token::PreservesFlagsKw
        | Token::PureKw
        | Token::RawKw
        | Token::ReadonlyKw
        | Token::SafeKw
        | Token::SymKw
        | Token::TryKw
        | Token::UnionKw
        | Token::YeetKw => Sample::Unread(Why(KEYWORD)),
        Token::BikeshedKw => Sample::Unread(Why(
            "a contextual keyword the grammar defines, but no construct of the SourceFile entry point of the parser produces it: the corpus of Rust files, the standard library and probes of every builtin construct gave none",
        )),
        Token::NullKw => Sample::Unread(Why(
            "a contextual keyword the grammar defines for pattern types, but the parser reads `!null` as a name and a not-null pattern, so no tree holds it",
        )),
        Token::Newline => Sample::Unread(Why(
            "defined by the grammar, but no tree built from the SourceFile entry point holds it: the corpus of Rust files, the standard library and a probe of frontmatter gave none",
        )),
    }
}
