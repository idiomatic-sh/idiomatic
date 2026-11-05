//! How the language shows each kind of its trees that is a node.

use crate::domain::nodes::types::RustNode;
use crate::domain::syntax::constants::{ASSEMBLY, EXPRESSION, LIST, PART, PATTERN, TYPE};
use ryo::languages::{Sample, SampleText, Why};

/// How the language shows a node kind of its trees: the source whose tree holds it, or the reason no relation models it.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "one arm for each kind of the grammar and no wildcard, so a kind the grammar gains fails to compile until it is shown; split, the match would lose that check"
)]
pub(super) const fn sample(kind: RustNode) -> Sample {
    match kind {
        RustNode::Attr => Sample::Node(SampleText("#![no_std]")),
        RustNode::CallExpr => Sample::Node(SampleText("fn main() { helper(); }")),
        RustNode::ClosureExpr => {
            Sample::Node(SampleText("pub fn run() {\n    rayon::scope(|_| {});\n}"))
        }
        RustNode::Const => Sample::Node(SampleText("const _: () = ();")),
        RustNode::ConstParam => Sample::Node(SampleText("impl<const N: usize> Eq for String<N> {}")),
        RustNode::Enum => Sample::Node(SampleText("enum Void {}")),
        RustNode::ExternCrate => Sample::Node(SampleText("extern crate std;")),
        RustNode::Fn => Sample::Node(SampleText("fn run() {}")),
        RustNode::IdentPat => Sample::Node(SampleText("fn run(value: u8) {}")),
        RustNode::Impl => Sample::Node(SampleText("impl Name {}")),
        RustNode::LetStmt => Sample::Node(SampleText("fn run() { let text = \"a\\\tb\"; }")),
        RustNode::LifetimeParam => Sample::Node(SampleText("impl<'a> Eq for Cursor<'a> {}")),
        RustNode::Literal => Sample::Node(SampleText("const TAG: u8 = 4;")),
        RustNode::MacroDef => Sample::Node(SampleText("macro emit() {}")),
        RustNode::MacroRules => Sample::Node(SampleText("macro_rules! twice { () => {}; }")),
        RustNode::MethodCallExpr => {
            Sample::Node(SampleText("fn main() {\n    try_main().unwrap();\n}"))
        }
        RustNode::Module => Sample::Node(SampleText("mod a;")),
        RustNode::SourceFile => Sample::Node(SampleText("fn main() {}")),
        RustNode::Static => Sample::Node(SampleText("static DATA: X = X;")),
        RustNode::Struct => Sample::Node(SampleText("struct X;")),
        RustNode::Trait => Sample::Node(SampleText("trait Run {}")),
        RustNode::TupleField => Sample::Node(SampleText("struct Once(X);")),
        RustNode::TypeAlias => Sample::Node(SampleText("type Foo = ();")),
        RustNode::TypeParam => Sample::Node(SampleText("struct Boxed<M>(M);")),
        RustNode::RecordField | RustNode::Union => Sample::Node(SampleText("union Bits { a: u8 }")),
        RustNode::Path | RustNode::Use | RustNode::UseTree => Sample::Node(SampleText("use toml;")),
        RustNode::Variant => Sample::Node(SampleText("enum Mode { On }")),
        RustNode::Abi
        | RustNode::AssocTypeArg
        | RustNode::CfgAtom
        | RustNode::CfgAttrMeta
        | RustNode::CfgComposite
        | RustNode::CfgMeta
        | RustNode::ConstArg
        | RustNode::ExprStmt
        | RustNode::ExternBlock
        | RustNode::FormatArgsArg
        | RustNode::ForBinder
        | RustNode::ImplRestriction
        | RustNode::KeyValueMeta
        | RustNode::Label
        | RustNode::LetElse
        | RustNode::Lifetime
        | RustNode::LifetimeArg
        | RustNode::MacroCall
        | RustNode::MatchArm
        | RustNode::MatchGuard
        | RustNode::MutRestriction
        | RustNode::Name
        | RustNode::NameRef
        | RustNode::NotNull
        | RustNode::Param
        | RustNode::PathMeta
        | RustNode::PathSegment
        | RustNode::RecordExprField
        | RustNode::RecordPatField
        | RustNode::Rename
        | RustNode::ReturnTypeSyntax
        | RustNode::SelfParam
        | RustNode::TokenTree
        | RustNode::TokenTreeMeta
        | RustNode::TryBlockModifier
        | RustNode::TypeAnchor
        | RustNode::TypeArg
        | RustNode::TypeBound
        | RustNode::UnsafeMeta
        | RustNode::UseBoundGenericArgs
        | RustNode::Visibility
        | RustNode::VisibilityInner
        | RustNode::WhereClause
        | RustNode::WherePred => Sample::Unread(Why(PART)),
        RustNode::ArgList
        | RustNode::AssocItemList
        | RustNode::ExternItemList
        | RustNode::GenericArgList
        | RustNode::GenericParamList
        | RustNode::ItemList
        | RustNode::MatchArmList
        | RustNode::ParamList
        | RustNode::ParenthesizedArgList
        | RustNode::RecordExprFieldList
        | RustNode::RecordFieldList
        | RustNode::RecordPatFieldList
        | RustNode::StmtList
        | RustNode::TupleFieldList
        | RustNode::TypeBoundList
        | RustNode::UseTreeList
        | RustNode::VariantList => Sample::Unread(Why(LIST)),
        RustNode::ArrayExpr
        | RustNode::AwaitExpr
        | RustNode::BecomeExpr
        | RustNode::BinExpr
        | RustNode::BlockExpr
        | RustNode::BreakExpr
        | RustNode::CastExpr
        | RustNode::ContinueExpr
        | RustNode::FieldExpr
        | RustNode::FormatArgsExpr
        | RustNode::ForExpr
        | RustNode::IfExpr
        | RustNode::IncludeBytesExpr
        | RustNode::IndexExpr
        | RustNode::LetExpr
        | RustNode::LoopExpr
        | RustNode::MacroExpr
        | RustNode::MatchExpr
        | RustNode::OffsetOfExpr
        | RustNode::ParenExpr
        | RustNode::PathExpr
        | RustNode::PrefixExpr
        | RustNode::RangeExpr
        | RustNode::RecordExpr
        | RustNode::RefExpr
        | RustNode::ReturnExpr
        | RustNode::TryExpr
        | RustNode::TupleExpr
        | RustNode::UnderscoreExpr
        | RustNode::WhileExpr
        | RustNode::YeetExpr
        | RustNode::YieldExpr => Sample::Unread(Why(EXPRESSION)),
        RustNode::ArrayType
        | RustNode::DynTraitType
        | RustNode::FnPtrType
        | RustNode::ForType
        | RustNode::ImplTraitType
        | RustNode::InferType
        | RustNode::MacroType
        | RustNode::NeverType
        | RustNode::ParenType
        | RustNode::PathType
        | RustNode::PatternType
        | RustNode::PtrType
        | RustNode::RefType
        | RustNode::RetType
        | RustNode::SliceType
        | RustNode::TupleType => Sample::Unread(Why(TYPE)),
        RustNode::AsmClobberAbi
        | RustNode::AsmConst
        | RustNode::AsmDirSpec
        | RustNode::AsmExpr
        | RustNode::AsmLabel
        | RustNode::AsmOperandExpr
        | RustNode::AsmOperandNamed
        | RustNode::AsmOption
        | RustNode::AsmOptions
        | RustNode::AsmRegOperand
        | RustNode::AsmRegSpec
        | RustNode::AsmSym => Sample::Unread(Why(ASSEMBLY)),
        RustNode::BoxPat
        | RustNode::ConstBlockPat
        | RustNode::DerefPat
        | RustNode::LiteralPat
        | RustNode::MacroPat
        | RustNode::OrPat
        | RustNode::ParenPat
        | RustNode::PathPat
        | RustNode::RangePat
        | RustNode::RecordPat
        | RustNode::RefPat
        | RustNode::RestPat
        | RustNode::SlicePat
        | RustNode::TuplePat
        | RustNode::TupleStructPat
        | RustNode::WildcardPat => Sample::Unread(Why(PATTERN)),
        RustNode::MacroItems => Sample::Unread(Why(
            "produced only by the MacroItems entry point of the parser, which expands a macro into items; the adapter parses the SourceFile entry point",
        )),
        RustNode::MacroStmts => Sample::Unread(Why(
            "produced only by the MacroStmts entry point of the parser, which expands a macro into statements; the adapter parses the SourceFile entry point",
        )),
    }
}
