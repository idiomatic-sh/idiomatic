// ignore: idiomatic.arch/file-length - the one schemas.rs the language spec places for this adapter, holding the library text it converts: the kind map, one arm for each of the grammar's kinds with no catch-all, and the numeric literal schema; no placed role splits them
//! What the library gives, which the adapter does not control, and its conversion into the adapter's own types: the parser's events into
//! steps, a numeric literal's text into its value, and the grammar's kinds into the language's own, one arm for each with no
//! catch-all, so a kind the library gains stops this from compiling until it is given a kind of the language's (or `Other`) on
//! purpose.

use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::tokens::types::Token;
use ra_ap_parser::StrStep;
use ra_ap_syntax::SyntaxKind;
use ryo::languages::{Count, FloatBits, NameText, Offset};

use std::borrow::Cow;

use super::constants::{BASES, F32_SUFFIX, F64_SUFFIX, INTEGER_SUFFIXES, UNREADABLE};
use super::types::{DecodedNumber, IntegerValue, NumberValue, ParserMessage, Repaired, Step};
use crate::domain::syntax::types::{Kind, TokenText};
use crate::outbound::syntax::types::SourceText;

/// One step of the parser's events.
pub(super) struct StepSchema<'a>(StrStep<'a>);

impl From<StepSchema<'_>> for Step {
    fn from(schema: StepSchema<'_>) -> Self {
        match schema.0 {
            StrStep::Enter { kind } => Self::Enter(kind),
            StrStep::Token { kind, text } => Self::Token(kind, Count(text.len())),
            StrStep::Exit => Self::Exit,
            StrStep::Error { msg, pos } => Self::Error(
                ParserMessage(msg.into()),
                Offset(u32::try_from(pos).unwrap_or(u32::MAX)),
            ),
        }
    }
}

/// A step of the parser's events as the adapter holds it.
pub(super) fn step_of(step: StrStep<'_>) -> Step {
    Step::from(StepSchema(step))
}

/// The language's kind for the library's `kind`: the kind of the same name; the parser's three markers, which no tree it builds holds,
/// are `Other`.
#[expect(
    clippy::too_many_lines,
    reason = "one arm for each kind of the library, to the kind of the same name, and no wildcard: a kind the library gains fails to compile until it is mapped"
)]
pub(super) const fn kind_of(kind: SyntaxKind) -> Kind {
    match kind {
        SyntaxKind::DOLLAR => Kind::Token(Token::Dollar),
        SyntaxKind::SEMICOLON => Kind::Token(Token::Semicolon),
        SyntaxKind::COMMA => Kind::Token(Token::Comma),
        SyntaxKind::L_PAREN => Kind::Token(Token::LParen),
        SyntaxKind::R_PAREN => Kind::Token(Token::RParen),
        SyntaxKind::L_CURLY => Kind::Token(Token::LCurly),
        SyntaxKind::R_CURLY => Kind::Token(Token::RCurly),
        SyntaxKind::L_BRACK => Kind::Token(Token::LBrack),
        SyntaxKind::R_BRACK => Kind::Token(Token::RBrack),
        SyntaxKind::L_ANGLE => Kind::Token(Token::LAngle),
        SyntaxKind::R_ANGLE => Kind::Token(Token::RAngle),
        SyntaxKind::AT => Kind::Token(Token::At),
        SyntaxKind::POUND => Kind::Token(Token::Pound),
        SyntaxKind::TILDE => Kind::Token(Token::Tilde),
        SyntaxKind::QUESTION => Kind::Token(Token::Question),
        SyntaxKind::AMP => Kind::Token(Token::Amp),
        SyntaxKind::PIPE => Kind::Token(Token::Pipe),
        SyntaxKind::PLUS => Kind::Token(Token::Plus),
        SyntaxKind::STAR => Kind::Token(Token::Star),
        SyntaxKind::SLASH => Kind::Token(Token::Slash),
        SyntaxKind::CARET => Kind::Token(Token::Caret),
        SyntaxKind::PERCENT => Kind::Token(Token::Percent),
        SyntaxKind::UNDERSCORE => Kind::Token(Token::Underscore),
        SyntaxKind::DOT => Kind::Token(Token::Dot),
        SyntaxKind::DOT2 => Kind::Token(Token::Dot2),
        SyntaxKind::DOT3 => Kind::Token(Token::Dot3),
        SyntaxKind::DOT2EQ => Kind::Token(Token::Dot2eq),
        SyntaxKind::COLON => Kind::Token(Token::Colon),
        SyntaxKind::COLON2 => Kind::Token(Token::Colon2),
        SyntaxKind::EQ => Kind::Token(Token::Eq),
        SyntaxKind::EQ2 => Kind::Token(Token::Eq2),
        SyntaxKind::FAT_ARROW => Kind::Token(Token::FatArrow),
        SyntaxKind::BANG => Kind::Token(Token::Bang),
        SyntaxKind::NEQ => Kind::Token(Token::Neq),
        SyntaxKind::MINUS => Kind::Token(Token::Minus),
        SyntaxKind::THIN_ARROW => Kind::Token(Token::ThinArrow),
        SyntaxKind::LTEQ => Kind::Token(Token::Lteq),
        SyntaxKind::GTEQ => Kind::Token(Token::Gteq),
        SyntaxKind::PLUSEQ => Kind::Token(Token::Pluseq),
        SyntaxKind::MINUSEQ => Kind::Token(Token::Minuseq),
        SyntaxKind::PIPEEQ => Kind::Token(Token::Pipeeq),
        SyntaxKind::AMPEQ => Kind::Token(Token::Ampeq),
        SyntaxKind::CARETEQ => Kind::Token(Token::Careteq),
        SyntaxKind::SLASHEQ => Kind::Token(Token::Slasheq),
        SyntaxKind::STAREQ => Kind::Token(Token::Stareq),
        SyntaxKind::PERCENTEQ => Kind::Token(Token::Percenteq),
        SyntaxKind::AMP2 => Kind::Token(Token::Amp2),
        SyntaxKind::PIPE2 => Kind::Token(Token::Pipe2),
        SyntaxKind::SHL => Kind::Token(Token::Shl),
        SyntaxKind::SHR => Kind::Token(Token::Shr),
        SyntaxKind::SHLEQ => Kind::Token(Token::Shleq),
        SyntaxKind::SHREQ => Kind::Token(Token::Shreq),
        SyntaxKind::SELF_TYPE_KW => Kind::Token(Token::SelfTypeKw),
        SyntaxKind::ABSTRACT_KW => Kind::Token(Token::AbstractKw),
        SyntaxKind::AS_KW => Kind::Token(Token::AsKw),
        SyntaxKind::BECOME_KW => Kind::Token(Token::BecomeKw),
        SyntaxKind::BOX_KW => Kind::Token(Token::BoxKw),
        SyntaxKind::BREAK_KW => Kind::Token(Token::BreakKw),
        SyntaxKind::CONST_KW => Kind::Token(Token::ConstKw),
        SyntaxKind::CONTINUE_KW => Kind::Token(Token::ContinueKw),
        SyntaxKind::CRATE_KW => Kind::Token(Token::CrateKw),
        SyntaxKind::DO_KW => Kind::Token(Token::DoKw),
        SyntaxKind::ELSE_KW => Kind::Token(Token::ElseKw),
        SyntaxKind::ENUM_KW => Kind::Token(Token::EnumKw),
        SyntaxKind::EXTERN_KW => Kind::Token(Token::ExternKw),
        SyntaxKind::FALSE_KW => Kind::Token(Token::FalseKw),
        SyntaxKind::FINAL_KW => Kind::Token(Token::FinalKw),
        SyntaxKind::FN_KW => Kind::Token(Token::FnKw),
        SyntaxKind::FOR_KW => Kind::Token(Token::ForKw),
        SyntaxKind::IF_KW => Kind::Token(Token::IfKw),
        SyntaxKind::IMPL_KW => Kind::Token(Token::ImplKw),
        SyntaxKind::IN_KW => Kind::Token(Token::InKw),
        SyntaxKind::LET_KW => Kind::Token(Token::LetKw),
        SyntaxKind::LOOP_KW => Kind::Token(Token::LoopKw),
        SyntaxKind::MACRO_KW => Kind::Token(Token::MacroKw),
        SyntaxKind::MATCH_KW => Kind::Token(Token::MatchKw),
        SyntaxKind::MOD_KW => Kind::Token(Token::ModKw),
        SyntaxKind::MOVE_KW => Kind::Token(Token::MoveKw),
        SyntaxKind::MUT_KW => Kind::Token(Token::MutKw),
        SyntaxKind::OVERRIDE_KW => Kind::Token(Token::OverrideKw),
        SyntaxKind::PRIV_KW => Kind::Token(Token::PrivKw),
        SyntaxKind::PUB_KW => Kind::Token(Token::PubKw),
        SyntaxKind::REF_KW => Kind::Token(Token::RefKw),
        SyntaxKind::RETURN_KW => Kind::Token(Token::ReturnKw),
        SyntaxKind::SELF_KW => Kind::Token(Token::SelfKw),
        SyntaxKind::STATIC_KW => Kind::Token(Token::StaticKw),
        SyntaxKind::STRUCT_KW => Kind::Token(Token::StructKw),
        SyntaxKind::SUPER_KW => Kind::Token(Token::SuperKw),
        SyntaxKind::TRAIT_KW => Kind::Token(Token::TraitKw),
        SyntaxKind::TRUE_KW => Kind::Token(Token::TrueKw),
        SyntaxKind::TYPE_KW => Kind::Token(Token::TypeKw),
        SyntaxKind::TYPEOF_KW => Kind::Token(Token::TypeofKw),
        SyntaxKind::UNSAFE_KW => Kind::Token(Token::UnsafeKw),
        SyntaxKind::UNSIZED_KW => Kind::Token(Token::UnsizedKw),
        SyntaxKind::USE_KW => Kind::Token(Token::UseKw),
        SyntaxKind::VIRTUAL_KW => Kind::Token(Token::VirtualKw),
        SyntaxKind::WHERE_KW => Kind::Token(Token::WhereKw),
        SyntaxKind::WHILE_KW => Kind::Token(Token::WhileKw),
        SyntaxKind::YIELD_KW => Kind::Token(Token::YieldKw),
        SyntaxKind::ASM_KW => Kind::Token(Token::AsmKw),
        SyntaxKind::ASYNC_KW => Kind::Token(Token::AsyncKw),
        SyntaxKind::ATT_SYNTAX_KW => Kind::Token(Token::AttSyntaxKw),
        SyntaxKind::AUTO_KW => Kind::Token(Token::AutoKw),
        SyntaxKind::AWAIT_KW => Kind::Token(Token::AwaitKw),
        SyntaxKind::BIKESHED_KW => Kind::Token(Token::BikeshedKw),
        SyntaxKind::BUILTIN_KW => Kind::Token(Token::BuiltinKw),
        SyntaxKind::CFG_ATTR_KW => Kind::Token(Token::CfgAttrKw),
        SyntaxKind::CFG_KW => Kind::Token(Token::CfgKw),
        SyntaxKind::CLOBBER_ABI_KW => Kind::Token(Token::ClobberAbiKw),
        SyntaxKind::DEFAULT_KW => Kind::Token(Token::DefaultKw),
        SyntaxKind::DEREF_KW => Kind::Token(Token::DerefKw),
        SyntaxKind::DYN_KW => Kind::Token(Token::DynKw),
        SyntaxKind::FORMAT_ARGS_KW => Kind::Token(Token::FormatArgsKw),
        SyntaxKind::GEN_KW => Kind::Token(Token::GenKw),
        SyntaxKind::GLOBAL_ASM_KW => Kind::Token(Token::GlobalAsmKw),
        SyntaxKind::INCLUDE_BYTES_KW => Kind::Token(Token::IncludeBytesKw),
        SyntaxKind::INLATEOUT_KW => Kind::Token(Token::InlateoutKw),
        SyntaxKind::INOUT_KW => Kind::Token(Token::InoutKw),
        SyntaxKind::IS_KW => Kind::Token(Token::IsKw),
        SyntaxKind::LABEL_KW => Kind::Token(Token::LabelKw),
        SyntaxKind::LATEOUT_KW => Kind::Token(Token::LateoutKw),
        SyntaxKind::MACRO_RULES_KW => Kind::Token(Token::MacroRulesKw),
        SyntaxKind::MAY_UNWIND_KW => Kind::Token(Token::MayUnwindKw),
        SyntaxKind::NAKED_ASM_KW => Kind::Token(Token::NakedAsmKw),
        SyntaxKind::NOMEM_KW => Kind::Token(Token::NomemKw),
        SyntaxKind::NORETURN_KW => Kind::Token(Token::NoreturnKw),
        SyntaxKind::NOSTACK_KW => Kind::Token(Token::NostackKw),
        SyntaxKind::NULL_KW => Kind::Token(Token::NullKw),
        SyntaxKind::OFFSET_OF_KW => Kind::Token(Token::OffsetOfKw),
        SyntaxKind::OPTIONS_KW => Kind::Token(Token::OptionsKw),
        SyntaxKind::OUT_KW => Kind::Token(Token::OutKw),
        SyntaxKind::PATTERN_TYPE_KW => Kind::Token(Token::PatternTypeKw),
        SyntaxKind::PRESERVES_FLAGS_KW => Kind::Token(Token::PreservesFlagsKw),
        SyntaxKind::PURE_KW => Kind::Token(Token::PureKw),
        SyntaxKind::RAW_KW => Kind::Token(Token::RawKw),
        SyntaxKind::READONLY_KW => Kind::Token(Token::ReadonlyKw),
        SyntaxKind::SAFE_KW => Kind::Token(Token::SafeKw),
        SyntaxKind::SYM_KW => Kind::Token(Token::SymKw),
        SyntaxKind::TRY_KW => Kind::Token(Token::TryKw),
        SyntaxKind::UNION_KW => Kind::Token(Token::UnionKw),
        SyntaxKind::YEET_KW => Kind::Token(Token::YeetKw),
        SyntaxKind::BYTE => Kind::Token(Token::Byte),
        SyntaxKind::BYTE_STRING => Kind::Token(Token::ByteString),
        SyntaxKind::CHAR => Kind::Token(Token::Char),
        SyntaxKind::C_STRING => Kind::Token(Token::CString),
        SyntaxKind::FLOAT_NUMBER => Kind::Token(Token::FloatNumber),
        SyntaxKind::INT_NUMBER => Kind::Token(Token::IntNumber),
        SyntaxKind::STRING => Kind::Token(Token::String),
        SyntaxKind::COMMENT => Kind::Token(Token::Comment),
        SyntaxKind::ERROR => Kind::Token(Token::Error),
        SyntaxKind::FRONTMATTER => Kind::Token(Token::Frontmatter),
        SyntaxKind::IDENT => Kind::Token(Token::Ident),
        SyntaxKind::LIFETIME_IDENT => Kind::Token(Token::LifetimeIdent),
        SyntaxKind::NEWLINE => Kind::Token(Token::Newline),
        SyntaxKind::SHEBANG => Kind::Token(Token::Shebang),
        SyntaxKind::WHITESPACE => Kind::Token(Token::Whitespace),
        SyntaxKind::ABI => Kind::Node(Node::Abi),
        SyntaxKind::ARG_LIST => Kind::Node(Node::ArgList),
        SyntaxKind::ARRAY_EXPR => Kind::Node(Node::ArrayExpr),
        SyntaxKind::ARRAY_TYPE => Kind::Node(Node::ArrayType),
        SyntaxKind::ASM_CLOBBER_ABI => Kind::Node(Node::AsmClobberAbi),
        SyntaxKind::ASM_CONST => Kind::Node(Node::AsmConst),
        SyntaxKind::ASM_DIR_SPEC => Kind::Node(Node::AsmDirSpec),
        SyntaxKind::ASM_EXPR => Kind::Node(Node::AsmExpr),
        SyntaxKind::ASM_LABEL => Kind::Node(Node::AsmLabel),
        SyntaxKind::ASM_OPERAND_EXPR => Kind::Node(Node::AsmOperandExpr),
        SyntaxKind::ASM_OPERAND_NAMED => Kind::Node(Node::AsmOperandNamed),
        SyntaxKind::ASM_OPTION => Kind::Node(Node::AsmOption),
        SyntaxKind::ASM_OPTIONS => Kind::Node(Node::AsmOptions),
        SyntaxKind::ASM_REG_OPERAND => Kind::Node(Node::AsmRegOperand),
        SyntaxKind::ASM_REG_SPEC => Kind::Node(Node::AsmRegSpec),
        SyntaxKind::ASM_SYM => Kind::Node(Node::AsmSym),
        SyntaxKind::ASSOC_ITEM_LIST => Kind::Node(Node::AssocItemList),
        SyntaxKind::ASSOC_TYPE_ARG => Kind::Node(Node::AssocTypeArg),
        SyntaxKind::ATTR => Kind::Node(Node::Attr),
        SyntaxKind::AWAIT_EXPR => Kind::Node(Node::AwaitExpr),
        SyntaxKind::BECOME_EXPR => Kind::Node(Node::BecomeExpr),
        SyntaxKind::BIN_EXPR => Kind::Node(Node::BinExpr),
        SyntaxKind::BLOCK_EXPR => Kind::Node(Node::BlockExpr),
        SyntaxKind::BOX_PAT => Kind::Node(Node::BoxPat),
        SyntaxKind::BREAK_EXPR => Kind::Node(Node::BreakExpr),
        SyntaxKind::CALL_EXPR => Kind::Node(Node::CallExpr),
        SyntaxKind::CAST_EXPR => Kind::Node(Node::CastExpr),
        SyntaxKind::CFG_ATOM => Kind::Node(Node::CfgAtom),
        SyntaxKind::CFG_ATTR_META => Kind::Node(Node::CfgAttrMeta),
        SyntaxKind::CFG_COMPOSITE => Kind::Node(Node::CfgComposite),
        SyntaxKind::CFG_META => Kind::Node(Node::CfgMeta),
        SyntaxKind::CLOSURE_EXPR => Kind::Node(Node::ClosureExpr),
        SyntaxKind::CONST => Kind::Node(Node::Const),
        SyntaxKind::CONST_ARG => Kind::Node(Node::ConstArg),
        SyntaxKind::CONST_BLOCK_PAT => Kind::Node(Node::ConstBlockPat),
        SyntaxKind::CONST_PARAM => Kind::Node(Node::ConstParam),
        SyntaxKind::CONTINUE_EXPR => Kind::Node(Node::ContinueExpr),
        SyntaxKind::DEREF_PAT => Kind::Node(Node::DerefPat),
        SyntaxKind::DYN_TRAIT_TYPE => Kind::Node(Node::DynTraitType),
        SyntaxKind::ENUM => Kind::Node(Node::Enum),
        SyntaxKind::EXPR_STMT => Kind::Node(Node::ExprStmt),
        SyntaxKind::EXTERN_BLOCK => Kind::Node(Node::ExternBlock),
        SyntaxKind::EXTERN_CRATE => Kind::Node(Node::ExternCrate),
        SyntaxKind::EXTERN_ITEM_LIST => Kind::Node(Node::ExternItemList),
        SyntaxKind::FIELD_EXPR => Kind::Node(Node::FieldExpr),
        SyntaxKind::FN => Kind::Node(Node::Fn),
        SyntaxKind::FN_PTR_TYPE => Kind::Node(Node::FnPtrType),
        SyntaxKind::FORMAT_ARGS_ARG => Kind::Node(Node::FormatArgsArg),
        SyntaxKind::FORMAT_ARGS_EXPR => Kind::Node(Node::FormatArgsExpr),
        SyntaxKind::FOR_BINDER => Kind::Node(Node::ForBinder),
        SyntaxKind::FOR_EXPR => Kind::Node(Node::ForExpr),
        SyntaxKind::FOR_TYPE => Kind::Node(Node::ForType),
        SyntaxKind::GENERIC_ARG_LIST => Kind::Node(Node::GenericArgList),
        SyntaxKind::GENERIC_PARAM_LIST => Kind::Node(Node::GenericParamList),
        SyntaxKind::IDENT_PAT => Kind::Node(Node::IdentPat),
        SyntaxKind::IF_EXPR => Kind::Node(Node::IfExpr),
        SyntaxKind::IMPL => Kind::Node(Node::Impl),
        SyntaxKind::IMPL_RESTRICTION => Kind::Node(Node::ImplRestriction),
        SyntaxKind::IMPL_TRAIT_TYPE => Kind::Node(Node::ImplTraitType),
        SyntaxKind::INCLUDE_BYTES_EXPR => Kind::Node(Node::IncludeBytesExpr),
        SyntaxKind::INDEX_EXPR => Kind::Node(Node::IndexExpr),
        SyntaxKind::INFER_TYPE => Kind::Node(Node::InferType),
        SyntaxKind::ITEM_LIST => Kind::Node(Node::ItemList),
        SyntaxKind::KEY_VALUE_META => Kind::Node(Node::KeyValueMeta),
        SyntaxKind::LABEL => Kind::Node(Node::Label),
        SyntaxKind::LET_ELSE => Kind::Node(Node::LetElse),
        SyntaxKind::LET_EXPR => Kind::Node(Node::LetExpr),
        SyntaxKind::LET_STMT => Kind::Node(Node::LetStmt),
        SyntaxKind::LIFETIME => Kind::Node(Node::Lifetime),
        SyntaxKind::LIFETIME_ARG => Kind::Node(Node::LifetimeArg),
        SyntaxKind::LIFETIME_PARAM => Kind::Node(Node::LifetimeParam),
        SyntaxKind::LITERAL => Kind::Node(Node::Literal),
        SyntaxKind::LITERAL_PAT => Kind::Node(Node::LiteralPat),
        SyntaxKind::LOOP_EXPR => Kind::Node(Node::LoopExpr),
        SyntaxKind::MACRO_CALL => Kind::Node(Node::MacroCall),
        SyntaxKind::MACRO_DEF => Kind::Node(Node::MacroDef),
        SyntaxKind::MACRO_EXPR => Kind::Node(Node::MacroExpr),
        SyntaxKind::MACRO_ITEMS => Kind::Node(Node::MacroItems),
        SyntaxKind::MACRO_PAT => Kind::Node(Node::MacroPat),
        SyntaxKind::MACRO_RULES => Kind::Node(Node::MacroRules),
        SyntaxKind::MACRO_STMTS => Kind::Node(Node::MacroStmts),
        SyntaxKind::MACRO_TYPE => Kind::Node(Node::MacroType),
        SyntaxKind::MATCH_ARM => Kind::Node(Node::MatchArm),
        SyntaxKind::MATCH_ARM_LIST => Kind::Node(Node::MatchArmList),
        SyntaxKind::MATCH_EXPR => Kind::Node(Node::MatchExpr),
        SyntaxKind::MATCH_GUARD => Kind::Node(Node::MatchGuard),
        SyntaxKind::METHOD_CALL_EXPR => Kind::Node(Node::MethodCallExpr),
        SyntaxKind::MODULE => Kind::Node(Node::Module),
        SyntaxKind::MUT_RESTRICTION => Kind::Node(Node::MutRestriction),
        SyntaxKind::NAME => Kind::Node(Node::Name),
        SyntaxKind::NAME_REF => Kind::Node(Node::NameRef),
        SyntaxKind::NEVER_TYPE => Kind::Node(Node::NeverType),
        SyntaxKind::NOT_NULL => Kind::Node(Node::NotNull),
        SyntaxKind::OFFSET_OF_EXPR => Kind::Node(Node::OffsetOfExpr),
        SyntaxKind::OR_PAT => Kind::Node(Node::OrPat),
        SyntaxKind::PARAM => Kind::Node(Node::Param),
        SyntaxKind::PARAM_LIST => Kind::Node(Node::ParamList),
        SyntaxKind::PARENTHESIZED_ARG_LIST => Kind::Node(Node::ParenthesizedArgList),
        SyntaxKind::PAREN_EXPR => Kind::Node(Node::ParenExpr),
        SyntaxKind::PAREN_PAT => Kind::Node(Node::ParenPat),
        SyntaxKind::PAREN_TYPE => Kind::Node(Node::ParenType),
        SyntaxKind::PATH => Kind::Node(Node::Path),
        SyntaxKind::PATH_EXPR => Kind::Node(Node::PathExpr),
        SyntaxKind::PATH_META => Kind::Node(Node::PathMeta),
        SyntaxKind::PATH_PAT => Kind::Node(Node::PathPat),
        SyntaxKind::PATH_SEGMENT => Kind::Node(Node::PathSegment),
        SyntaxKind::PATH_TYPE => Kind::Node(Node::PathType),
        SyntaxKind::PATTERN_TYPE => Kind::Node(Node::PatternType),
        SyntaxKind::PREFIX_EXPR => Kind::Node(Node::PrefixExpr),
        SyntaxKind::PTR_TYPE => Kind::Node(Node::PtrType),
        SyntaxKind::RANGE_EXPR => Kind::Node(Node::RangeExpr),
        SyntaxKind::RANGE_PAT => Kind::Node(Node::RangePat),
        SyntaxKind::RECORD_EXPR => Kind::Node(Node::RecordExpr),
        SyntaxKind::RECORD_EXPR_FIELD => Kind::Node(Node::RecordExprField),
        SyntaxKind::RECORD_EXPR_FIELD_LIST => Kind::Node(Node::RecordExprFieldList),
        SyntaxKind::RECORD_FIELD => Kind::Node(Node::RecordField),
        SyntaxKind::RECORD_FIELD_LIST => Kind::Node(Node::RecordFieldList),
        SyntaxKind::RECORD_PAT => Kind::Node(Node::RecordPat),
        SyntaxKind::RECORD_PAT_FIELD => Kind::Node(Node::RecordPatField),
        SyntaxKind::RECORD_PAT_FIELD_LIST => Kind::Node(Node::RecordPatFieldList),
        SyntaxKind::REF_EXPR => Kind::Node(Node::RefExpr),
        SyntaxKind::REF_PAT => Kind::Node(Node::RefPat),
        SyntaxKind::REF_TYPE => Kind::Node(Node::RefType),
        SyntaxKind::RENAME => Kind::Node(Node::Rename),
        SyntaxKind::REST_PAT => Kind::Node(Node::RestPat),
        SyntaxKind::RETURN_EXPR => Kind::Node(Node::ReturnExpr),
        SyntaxKind::RETURN_TYPE_SYNTAX => Kind::Node(Node::ReturnTypeSyntax),
        SyntaxKind::RET_TYPE => Kind::Node(Node::RetType),
        SyntaxKind::SELF_PARAM => Kind::Node(Node::SelfParam),
        SyntaxKind::SLICE_PAT => Kind::Node(Node::SlicePat),
        SyntaxKind::SLICE_TYPE => Kind::Node(Node::SliceType),
        SyntaxKind::SOURCE_FILE => Kind::Node(Node::SourceFile),
        SyntaxKind::STATIC => Kind::Node(Node::Static),
        SyntaxKind::STMT_LIST => Kind::Node(Node::StmtList),
        SyntaxKind::STRUCT => Kind::Node(Node::Struct),
        SyntaxKind::TOKEN_TREE => Kind::Node(Node::TokenTree),
        SyntaxKind::TOKEN_TREE_META => Kind::Node(Node::TokenTreeMeta),
        SyntaxKind::TRAIT => Kind::Node(Node::Trait),
        SyntaxKind::TRY_BLOCK_MODIFIER => Kind::Node(Node::TryBlockModifier),
        SyntaxKind::TRY_EXPR => Kind::Node(Node::TryExpr),
        SyntaxKind::TUPLE_EXPR => Kind::Node(Node::TupleExpr),
        SyntaxKind::TUPLE_FIELD => Kind::Node(Node::TupleField),
        SyntaxKind::TUPLE_FIELD_LIST => Kind::Node(Node::TupleFieldList),
        SyntaxKind::TUPLE_PAT => Kind::Node(Node::TuplePat),
        SyntaxKind::TUPLE_STRUCT_PAT => Kind::Node(Node::TupleStructPat),
        SyntaxKind::TUPLE_TYPE => Kind::Node(Node::TupleType),
        SyntaxKind::TYPE_ALIAS => Kind::Node(Node::TypeAlias),
        SyntaxKind::TYPE_ANCHOR => Kind::Node(Node::TypeAnchor),
        SyntaxKind::TYPE_ARG => Kind::Node(Node::TypeArg),
        SyntaxKind::TYPE_BOUND => Kind::Node(Node::TypeBound),
        SyntaxKind::TYPE_BOUND_LIST => Kind::Node(Node::TypeBoundList),
        SyntaxKind::TYPE_PARAM => Kind::Node(Node::TypeParam),
        SyntaxKind::UNDERSCORE_EXPR => Kind::Node(Node::UnderscoreExpr),
        SyntaxKind::UNION => Kind::Node(Node::Union),
        SyntaxKind::UNSAFE_META => Kind::Node(Node::UnsafeMeta),
        SyntaxKind::USE => Kind::Node(Node::Use),
        SyntaxKind::USE_BOUND_GENERIC_ARGS => Kind::Node(Node::UseBoundGenericArgs),
        SyntaxKind::USE_TREE => Kind::Node(Node::UseTree),
        SyntaxKind::USE_TREE_LIST => Kind::Node(Node::UseTreeList),
        SyntaxKind::VARIANT => Kind::Node(Node::Variant),
        SyntaxKind::VARIANT_LIST => Kind::Node(Node::VariantList),
        SyntaxKind::VISIBILITY => Kind::Node(Node::Visibility),
        SyntaxKind::VISIBILITY_INNER => Kind::Node(Node::VisibilityInner),
        SyntaxKind::WHERE_CLAUSE => Kind::Node(Node::WhereClause),
        SyntaxKind::WHERE_PRED => Kind::Node(Node::WherePred),
        SyntaxKind::WHILE_EXPR => Kind::Node(Node::WhileExpr),
        SyntaxKind::WILDCARD_PAT => Kind::Node(Node::WildcardPat),
        SyntaxKind::YEET_EXPR => Kind::Node(Node::YeetExpr),
        SyntaxKind::YIELD_EXPR => Kind::Node(Node::YieldExpr),
        SyntaxKind::TOMBSTONE | SyntaxKind::EOF | SyntaxKind::__LAST => Kind::Extra(Extra::Other),
    }
}

/// A numeric literal as the lexer gave its token: digits with a base prefix or none, `_` between them, a fraction and an exponent for a
/// float, then a type suffix (the Rust Reference, Integer literals and Floating-point literals).
struct NumberSchema<'a>(&'a str);

/// Why a compiler refuses a numeric literal.
#[derive(Debug)]
enum NumberSchemaError {
    /// A suffix that names no numeric type of the literal's base: `1.0u8`, `0b1f32`, `1f16`.
    Suffix,
    /// Digits with no value: one outside the base, an integer past 128 bits.
    Digits,
}

/// The integer `digits` stand for in `radix`, each `_` skipped; none without a digit, with a digit outside the radix, or past 128 bits.
fn integer(digits: &str, radix: u32) -> Option<IntegerValue> {
    let mut value: u128 = 0;
    let mut counted = false;
    for character in digits.chars().filter(|character| *character != '_') {
        let digit = character.to_digit(radix)?;
        value = value
            .checked_mul(u128::from(radix))?
            .checked_add(u128::from(digit))?;
        counted = true;
    }
    counted.then_some(IntegerValue(value))
}

/// The binary64 value of the decimal `digits`, each `_` skipped: rounded to `f32` first when the suffix is `f32`, to `f64` when it is
/// `f64` or absent; refused for another suffix (`f16` and `f128` have no value here).
fn float(digits: &str, suffix: &str) -> Result<FloatBits, NumberSchemaError> {
    let text: Cow<'_, str> = if digits.contains('_') {
        Cow::Owned(digits.replace('_', ""))
    } else {
        Cow::Borrowed(digits)
    };
    // A parse that fails is digits the lexer took that are no number of this width: they have no value.
    if suffix == F32_SUFFIX {
        match text.parse::<f32>() {
            Ok(value) => Ok(FloatBits(f64::from(value).to_bits())),
            Err(_) => Err(NumberSchemaError::Digits),
        }
    } else if suffix.is_empty() || suffix == F64_SUFFIX {
        match text.parse::<f64>() {
            Ok(value) => Ok(FloatBits(value.to_bits())),
            Err(_) => Err(NumberSchemaError::Digits),
        }
    } else {
        Err(NumberSchemaError::Suffix)
    }
}

/// Where the decimal number at the start of `text` ends, and whether a fraction or an exponent made it a float.
fn decimal_end(text: &str) -> (usize, bool) {
    let bytes = text.as_bytes();
    let digits_from = |from: usize| {
        from + bytes
            .get(from..)
            .unwrap_or_default()
            .iter()
            .take_while(|byte| byte.is_ascii_digit() || **byte == b'_')
            .count()
    };
    let mut end = digits_from(0);
    let mut floating = false;
    if bytes.get(end) == Some(&b'.') {
        end = digits_from(end + 1);
        floating = true;
    }
    if matches!(bytes.get(end), Some(b'e' | b'E')) {
        let signed = end + 1 + usize::from(matches!(bytes.get(end + 1), Some(b'+' | b'-')));
        let after = digits_from(signed);
        if bytes
            .get(signed..after)
            .is_some_and(|exponent| exponent.iter().any(u8::is_ascii_digit))
        {
            end = after;
            floating = true;
        }
    }
    (end, floating)
}

impl<'a> TryFrom<NumberSchema<'a>> for DecodedNumber<'a> {
    type Error = NumberSchemaError;

    fn try_from(schema: NumberSchema<'a>) -> Result<Self, Self::Error> {
        let text = schema.0;
        let (suffix, value) = if let Some((rest, radix)) = BASES
            .iter()
            .find_map(|(prefix, radix)| Some((text.strip_prefix(prefix)?, *radix)))
        {
            let end = rest
                .find(|character: char| character != '_' && !character.is_digit(radix.max(10)))
                .unwrap_or(rest.len());
            let (digits, suffix) = rest.split_at(end);
            if !suffix.is_empty() && !INTEGER_SUFFIXES.contains(&suffix) {
                return Err(NumberSchemaError::Suffix);
            }
            let value = integer(digits, radix).ok_or(NumberSchemaError::Digits)?;
            (suffix, NumberValue::Integer(value))
        } else {
            let (end, floating) = decimal_end(text);
            let (digits, suffix) = text.split_at(end);
            let value = if !floating && (suffix.is_empty() || INTEGER_SUFFIXES.contains(&suffix)) {
                NumberValue::Integer(integer(digits, 10).ok_or(NumberSchemaError::Digits)?)
            } else {
                NumberValue::Float(float(digits, suffix)?)
            };
            (suffix, value)
        };
        Ok(Self {
            value,
            suffix: (!suffix.is_empty()).then_some(NameText(suffix)),
        })
    }
}

/// What the numeric literal written as `written` stands for; none when a compiler refuses it.
pub(super) fn number_of(written: TokenText<'_>) -> Option<DecodedNumber<'_>> {
    // The refusal says why a compiler refuses the literal; the table holds no value for it either way.
    match DecodedNumber::try_from(NumberSchema(written.0)) {
        Ok(number) => Some(number),
        Err(NumberSchemaError::Suffix | NumberSchemaError::Digits) => None,
    }
}

/// The text of `source`. Bytes that are not text are read, not refused: each byte that is not part of a character is read as a control
/// character the lexer reports, so the valid text keeps its tree and the rest is an error token with a diagnostic, at the same offsets.
pub(super) fn repaired(source: SourceText<'_>) -> Repaired<'_> {
    if let Ok(text) = std::str::from_utf8(source.0) {
        return Repaired(Cow::Borrowed(text));
    }
    let mut text = String::with_capacity(source.0.len());
    for chunk in source.0.utf8_chunks() {
        text.push_str(chunk.valid());
        text.extend(chunk.invalid().iter().map(|_| UNREADABLE));
    }
    Repaired(Cow::Owned(text))
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
