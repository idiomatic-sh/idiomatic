//! How the language shows each kind of its trees that is a node.

use crate::domain::nodes::types::Node;
use crate::domain::syntax::constants::{ASSEMBLY, EXPRESSION, LIST, PART, PATTERN, TYPE};
use ryo::languages::{Sample, SampleText, Why};

/// How the language shows a node kind of its trees: the source whose tree holds it, or the reason no relation models it.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "one arm for each kind of the grammar and no wildcard, so a kind the grammar gains fails to compile until it is shown; split, the match would lose that check"
)]
pub(super) const fn sample(kind: Node) -> Sample {
    match kind {
        Node::Attr => Sample::Node(SampleText("#![no_std]")),
        Node::CallExpr => Sample::Node(SampleText("fn main() { helper(); }")),
        Node::ClosureExpr => {
            Sample::Node(SampleText("pub fn run() {\n    rayon::scope(|_| {});\n}"))
        }
        Node::Const => Sample::Node(SampleText("const _: () = ();")),
        Node::ConstParam => Sample::Node(SampleText("impl<const N: usize> Eq for String<N> {}")),
        Node::Enum => Sample::Node(SampleText("enum Void {}")),
        Node::ExternCrate => Sample::Node(SampleText("extern crate std;")),
        Node::Fn => Sample::Node(SampleText("fn run() {}")),
        Node::IdentPat => Sample::Node(SampleText("fn run(value: u8) {}")),
        Node::Impl => Sample::Node(SampleText("impl Name {}")),
        Node::LetStmt => Sample::Node(SampleText("fn run() { let text = \"a\\\tb\"; }")),
        Node::LifetimeParam => Sample::Node(SampleText("impl<'a> Eq for Cursor<'a> {}")),
        Node::Literal => Sample::Node(SampleText("const TAG: u8 = 4;")),
        Node::MacroDef => Sample::Node(SampleText("macro emit() {}")),
        Node::MacroRules => Sample::Node(SampleText("macro_rules! twice { () => {}; }")),
        Node::MethodCallExpr => {
            Sample::Node(SampleText("fn main() {\n    try_main().unwrap();\n}"))
        }
        Node::Module => Sample::Node(SampleText("mod a;")),
        Node::SourceFile => Sample::Node(SampleText("fn main() {}")),
        Node::Static => Sample::Node(SampleText("static DATA: X = X;")),
        Node::Struct => Sample::Node(SampleText("struct X;")),
        Node::Trait => Sample::Node(SampleText("trait Run {}")),
        Node::TupleField => Sample::Node(SampleText("struct Once(X);")),
        Node::TypeAlias => Sample::Node(SampleText("type Foo = ();")),
        Node::TypeParam => Sample::Node(SampleText("struct Boxed<M>(M);")),
        Node::RecordField | Node::Union => Sample::Node(SampleText("union Bits { a: u8 }")),
        Node::Path | Node::Use | Node::UseTree => Sample::Node(SampleText("use toml;")),
        Node::Variant => Sample::Node(SampleText("enum Mode { On }")),
        Node::Abi
        | Node::AssocTypeArg
        | Node::CfgAtom
        | Node::CfgAttrMeta
        | Node::CfgComposite
        | Node::CfgMeta
        | Node::ConstArg
        | Node::ExprStmt
        | Node::ExternBlock
        | Node::FormatArgsArg
        | Node::ForBinder
        | Node::ImplRestriction
        | Node::KeyValueMeta
        | Node::Label
        | Node::LetElse
        | Node::Lifetime
        | Node::LifetimeArg
        | Node::MacroCall
        | Node::MatchArm
        | Node::MatchGuard
        | Node::MutRestriction
        | Node::Name
        | Node::NameRef
        | Node::NotNull
        | Node::Param
        | Node::PathMeta
        | Node::PathSegment
        | Node::RecordExprField
        | Node::RecordPatField
        | Node::Rename
        | Node::ReturnTypeSyntax
        | Node::SelfParam
        | Node::TokenTree
        | Node::TokenTreeMeta
        | Node::TryBlockModifier
        | Node::TypeAnchor
        | Node::TypeArg
        | Node::TypeBound
        | Node::UnsafeMeta
        | Node::UseBoundGenericArgs
        | Node::Visibility
        | Node::VisibilityInner
        | Node::WhereClause
        | Node::WherePred => Sample::Unread(Why(PART)),
        Node::ArgList
        | Node::AssocItemList
        | Node::ExternItemList
        | Node::GenericArgList
        | Node::GenericParamList
        | Node::ItemList
        | Node::MatchArmList
        | Node::ParamList
        | Node::ParenthesizedArgList
        | Node::RecordExprFieldList
        | Node::RecordFieldList
        | Node::RecordPatFieldList
        | Node::StmtList
        | Node::TupleFieldList
        | Node::TypeBoundList
        | Node::UseTreeList
        | Node::VariantList => Sample::Unread(Why(LIST)),
        Node::ArrayExpr
        | Node::AwaitExpr
        | Node::BecomeExpr
        | Node::BinExpr
        | Node::BlockExpr
        | Node::BreakExpr
        | Node::CastExpr
        | Node::ContinueExpr
        | Node::FieldExpr
        | Node::FormatArgsExpr
        | Node::ForExpr
        | Node::IfExpr
        | Node::IncludeBytesExpr
        | Node::IndexExpr
        | Node::LetExpr
        | Node::LoopExpr
        | Node::MacroExpr
        | Node::MatchExpr
        | Node::OffsetOfExpr
        | Node::ParenExpr
        | Node::PathExpr
        | Node::PrefixExpr
        | Node::RangeExpr
        | Node::RecordExpr
        | Node::RefExpr
        | Node::ReturnExpr
        | Node::TryExpr
        | Node::TupleExpr
        | Node::UnderscoreExpr
        | Node::WhileExpr
        | Node::YeetExpr
        | Node::YieldExpr => Sample::Unread(Why(EXPRESSION)),
        Node::ArrayType
        | Node::DynTraitType
        | Node::FnPtrType
        | Node::ForType
        | Node::ImplTraitType
        | Node::InferType
        | Node::MacroType
        | Node::NeverType
        | Node::ParenType
        | Node::PathType
        | Node::PatternType
        | Node::PtrType
        | Node::RefType
        | Node::RetType
        | Node::SliceType
        | Node::TupleType => Sample::Unread(Why(TYPE)),
        Node::AsmClobberAbi
        | Node::AsmConst
        | Node::AsmDirSpec
        | Node::AsmExpr
        | Node::AsmLabel
        | Node::AsmOperandExpr
        | Node::AsmOperandNamed
        | Node::AsmOption
        | Node::AsmOptions
        | Node::AsmRegOperand
        | Node::AsmRegSpec
        | Node::AsmSym => Sample::Unread(Why(ASSEMBLY)),
        Node::BoxPat
        | Node::ConstBlockPat
        | Node::DerefPat
        | Node::LiteralPat
        | Node::MacroPat
        | Node::OrPat
        | Node::ParenPat
        | Node::PathPat
        | Node::RangePat
        | Node::RecordPat
        | Node::RefPat
        | Node::RestPat
        | Node::SlicePat
        | Node::TuplePat
        | Node::TupleStructPat
        | Node::WildcardPat => Sample::Unread(Why(PATTERN)),
        Node::MacroItems => Sample::Unread(Why(
            "produced only by the MacroItems entry point of the parser, which expands a macro into items; the adapter parses the SourceFile entry point",
        )),
        Node::MacroStmts => Sample::Unread(Why(
            "produced only by the MacroStmts entry point of the parser, which expands a macro into statements; the adapter parses the SourceFile entry point",
        )),
    }
}
