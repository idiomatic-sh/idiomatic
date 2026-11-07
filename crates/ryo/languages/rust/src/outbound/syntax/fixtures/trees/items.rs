//! The trees of the sources the tests of the domain read, for structs, enums and impls.

use super::super::script::Piece;

/// The tree of `source`; none when no tree of this group is written for it.
#[must_use]
pub(super) fn tree_of(source: &str) -> Option<Piece> {
    Some(match source {
        "impl T { fn f() {} }" => crate::scripted_tree!(
            SourceFile {
                Impl {
                    ImplKw("impl"),
                    Whitespace(" "),
                    PathType { Path { PathSegment { NameRef { Ident("T") } } } },
                    Whitespace(" "),
                    AssocItemList {
                        LCurly("{"),
                        Whitespace(" "),
                        Fn {
                            FnKw("fn"),
                            Whitespace(" "),
                            Name { Ident("f") },
                            ParamList { LParen("("), RParen(")") },
                            Whitespace(" "),
                            BlockExpr { StmtList { LCurly("{"), RCurly("}") } },
                        },
                        Whitespace(" "),
                        RCurly("}"),
                    },
                },
            }
        ),
        "struct S { a: u8 }" => crate::scripted_tree!(
            SourceFile {
                Struct {
                    StructKw("struct"),
                    Whitespace(" "),
                    Name { Ident("S") },
                    Whitespace(" "),
                    RecordFieldList {
                        LCurly("{"),
                        Whitespace(" "),
                        RecordField {
                            Name { Ident("a") },
                            Colon(":"),
                            Whitespace(" "),
                            PathType { Path { PathSegment { NameRef { Ident("u8") } } } },
                        },
                        Whitespace(" "),
                        RCurly("}"),
                    },
                },
            }
        ),
        "struct S(u8);" => crate::scripted_tree!(
            SourceFile {
                Struct {
                    StructKw("struct"),
                    Whitespace(" "),
                    Name { Ident("S") },
                    TupleFieldList {
                        LParen("("),
                        TupleField { PathType { Path { PathSegment { NameRef { Ident("u8") } } } } },
                        RParen(")"),
                    },
                    Semicolon(";"),
                },
            }
        ),
        "enum E { A }" => crate::scripted_tree!(
            SourceFile {
                Enum {
                    EnumKw("enum"),
                    Whitespace(" "),
                    Name { Ident("E") },
                    Whitespace(" "),
                    VariantList {
                        LCurly("{"),
                        Whitespace(" "),
                        Variant { Name { Ident("A") } },
                        Whitespace(" "),
                        RCurly("}"),
                    },
                },
            }
        ),
        _ => return None,
    })
}
