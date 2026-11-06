//! The trees of the sources the tests of the domain read, for function signatures: names, parameters, generics, return types, qualifiers and attributes.

use super::super::script::{Piece, with_error};

/// The tree of `source`; none when no tree of this group is written for it.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "one arm for each source of this group, its tree written out in full: the arms are the table"
)]
pub(super) fn tree_of(source: &str) -> Option<Piece> {
    Some(match source {
        "fn f() {}" => crate::scripted_tree!(
            SourceFile {
                Fn {
                    FnKw("fn"),
                    Whitespace(" "),
                    Name { Ident("f") },
                    ParamList { LParen("("), RParen(")") },
                    Whitespace(" "),
                    BlockExpr { StmtList { LCurly("{"), RCurly("}") } },
                },
            }
        ),
        "pub fn f() {}" => crate::scripted_tree!(
            SourceFile {
                Fn {
                    Visibility { PubKw("pub") },
                    Whitespace(" "),
                    FnKw("fn"),
                    Whitespace(" "),
                    Name { Ident("f") },
                    ParamList { LParen("("), RParen(")") },
                    Whitespace(" "),
                    BlockExpr { StmtList { LCurly("{"), RCurly("}") } },
                },
            }
        ),
        "fn f<'a>() {}" => crate::scripted_tree!(
            SourceFile {
                Fn {
                    FnKw("fn"),
                    Whitespace(" "),
                    Name { Ident("f") },
                    GenericParamList { LAngle("<"), LifetimeParam { Lifetime { LifetimeIdent("'a") } }, RAngle(">") },
                    ParamList { LParen("("), RParen(")") },
                    Whitespace(" "),
                    BlockExpr { StmtList { LCurly("{"), RCurly("}") } },
                },
            }
        ),
        "extern \"C\" fn f() {}" => crate::scripted_tree!(
            SourceFile {
                Fn {
                    Abi { ExternKw("extern"), Whitespace(" "), String("\"C\"") },
                    Whitespace(" "),
                    FnKw("fn"),
                    Whitespace(" "),
                    Name { Ident("f") },
                    ParamList { LParen("("), RParen(")") },
                    Whitespace(" "),
                    BlockExpr { StmtList { LCurly("{"), RCurly("}") } },
                },
            }
        ),
        "fn f<T>() {}" => crate::scripted_tree!(
            SourceFile {
                Fn {
                    FnKw("fn"),
                    Whitespace(" "),
                    Name { Ident("f") },
                    GenericParamList { LAngle("<"), TypeParam { Name { Ident("T") } }, RAngle(">") },
                    ParamList { LParen("("), RParen(")") },
                    Whitespace(" "),
                    BlockExpr { StmtList { LCurly("{"), RCurly("}") } },
                },
            }
        ),
        "fn f() where T: A {}" => crate::scripted_tree!(
            SourceFile {
                Fn {
                    FnKw("fn"),
                    Whitespace(" "),
                    Name { Ident("f") },
                    ParamList { LParen("("), RParen(")") },
                    Whitespace(" "),
                    WhereClause {
                        WhereKw("where"),
                        Whitespace(" "),
                        WherePred {
                            PathType { Path { PathSegment { NameRef { Ident("T") } } } },
                            Colon(":"),
                            Whitespace(" "),
                            TypeBoundList { TypeBound { PathType { Path { PathSegment { NameRef { Ident("A") } } } } } },
                        },
                    },
                    Whitespace(" "),
                    BlockExpr { StmtList { LCurly("{"), RCurly("}") } },
                },
            }
        ),
        "fn f(a: u8) {}" => crate::scripted_tree!(
            SourceFile {
                Fn {
                    FnKw("fn"),
                    Whitespace(" "),
                    Name { Ident("f") },
                    ParamList {
                        LParen("("),
                        Param {
                            IdentPat { Name { Ident("a") } },
                            Colon(":"),
                            Whitespace(" "),
                            PathType { Path { PathSegment { NameRef { Ident("u8") } } } },
                        },
                        RParen(")"),
                    },
                    Whitespace(" "),
                    BlockExpr { StmtList { LCurly("{"), RCurly("}") } },
                },
            }
        ),
        "fn f() -> u8 {}" => crate::scripted_tree!(
            SourceFile {
                Fn {
                    FnKw("fn"),
                    Whitespace(" "),
                    Name { Ident("f") },
                    ParamList { LParen("("), RParen(")") },
                    Whitespace(" "),
                    RetType {
                        ThinArrow("->"),
                        Whitespace(" "),
                        PathType { Path { PathSegment { NameRef { Ident("u8") } } } },
                    },
                    Whitespace(" "),
                    BlockExpr { StmtList { LCurly("{"), RCurly("}") } },
                },
            }
        ),
        "fn f<T: A>() {}" => crate::scripted_tree!(
            SourceFile {
                Fn {
                    FnKw("fn"),
                    Whitespace(" "),
                    Name { Ident("f") },
                    GenericParamList {
                        LAngle("<"),
                        TypeParam {
                            Name { Ident("T") },
                            Colon(":"),
                            Whitespace(" "),
                            TypeBoundList { TypeBound { PathType { Path { PathSegment { NameRef { Ident("A") } } } } } },
                        },
                        RAngle(">"),
                    },
                    ParamList { LParen("("), RParen(")") },
                    Whitespace(" "),
                    BlockExpr { StmtList { LCurly("{"), RCurly("}") } },
                },
            }
        ),
        "fn f<T = u8>() {}" => crate::scripted_tree!(
            SourceFile {
                Fn {
                    FnKw("fn"),
                    Whitespace(" "),
                    Name { Ident("f") },
                    GenericParamList {
                        LAngle("<"),
                        TypeParam {
                            Name { Ident("T") },
                            Whitespace(" "),
                            Eq("="),
                            Whitespace(" "),
                            PathType { Path { PathSegment { NameRef { Ident("u8") } } } },
                        },
                        RAngle(">"),
                    },
                    ParamList { LParen("("), RParen(")") },
                    Whitespace(" "),
                    BlockExpr { StmtList { LCurly("{"), RCurly("}") } },
                },
            }
        ),
        "fn f<const N: usize = 1>() {}" => crate::scripted_tree!(
            SourceFile {
                Fn {
                    FnKw("fn"),
                    Whitespace(" "),
                    Name { Ident("f") },
                    GenericParamList {
                        LAngle("<"),
                        ConstParam {
                            ConstKw("const"),
                            Whitespace(" "),
                            Name { Ident("N") },
                            Colon(":"),
                            Whitespace(" "),
                            PathType { Path { PathSegment { NameRef { Ident("usize") } } } },
                            Whitespace(" "),
                            Eq("="),
                            Whitespace(" "),
                            ConstArg { Literal { IntNumber("1") } },
                        },
                        RAngle(">"),
                    },
                    ParamList { LParen("("), RParen(")") },
                    Whitespace(" "),
                    BlockExpr { StmtList { LCurly("{"), RCurly("}") } },
                },
            }
        ),
        "fn run() {}" => crate::scripted_tree!(
            SourceFile {
                Fn {
                    FnKw("fn"),
                    Whitespace(" "),
                    Name { Ident("run") },
                    ParamList { LParen("("), RParen(")") },
                    Whitespace(" "),
                    BlockExpr { StmtList { LCurly("{"), RCurly("}") } },
                },
            }
        ),
        "fn run(value: u8) {}" => crate::scripted_tree!(
            SourceFile {
                Fn {
                    FnKw("fn"),
                    Whitespace(" "),
                    Name { Ident("run") },
                    ParamList {
                        LParen("("),
                        Param {
                            IdentPat { Name { Ident("value") } },
                            Colon(":"),
                            Whitespace(" "),
                            PathType { Path { PathSegment { NameRef { Ident("u8") } } } },
                        },
                        RParen(")"),
                    },
                    Whitespace(" "),
                    BlockExpr { StmtList { LCurly("{"), RCurly("}") } },
                },
            }
        ),
        "fn run() -> u8 { 0 }" => crate::scripted_tree!(
            SourceFile {
                Fn {
                    FnKw("fn"),
                    Whitespace(" "),
                    Name { Ident("run") },
                    ParamList { LParen("("), RParen(")") },
                    Whitespace(" "),
                    RetType {
                        ThinArrow("->"),
                        Whitespace(" "),
                        PathType { Path { PathSegment { NameRef { Ident("u8") } } } },
                    },
                    Whitespace(" "),
                    BlockExpr {
                        StmtList {
                            LCurly("{"),
                            Whitespace(" "),
                            Literal { IntNumber("0") },
                            Whitespace(" "),
                            RCurly("}"),
                        },
                    },
                },
            }
        ),
        "fn f(a: u32) {}" => crate::scripted_tree!(
            SourceFile {
                Fn {
                    FnKw("fn"),
                    Whitespace(" "),
                    Name { Ident("f") },
                    ParamList {
                        LParen("("),
                        Param {
                            IdentPat { Name { Ident("a") } },
                            Colon(":"),
                            Whitespace(" "),
                            PathType { Path { PathSegment { NameRef { Ident("u32") } } } },
                        },
                        RParen(")"),
                    },
                    Whitespace(" "),
                    BlockExpr { StmtList { LCurly("{"), RCurly("}") } },
                },
            }
        ),
        "fn f((a, b): (u32, u32)) {}" => crate::scripted_tree!(
            SourceFile {
                Fn {
                    FnKw("fn"),
                    Whitespace(" "),
                    Name { Ident("f") },
                    ParamList {
                        LParen("("),
                        Param {
                            TuplePat {
                                LParen("("),
                                IdentPat { Name { Ident("a") } },
                                Comma(","),
                                Whitespace(" "),
                                IdentPat { Name { Ident("b") } },
                                RParen(")"),
                            },
                            Colon(":"),
                            Whitespace(" "),
                            TupleType {
                                LParen("("),
                                PathType { Path { PathSegment { NameRef { Ident("u32") } } } },
                                Comma(","),
                                Whitespace(" "),
                                PathType { Path { PathSegment { NameRef { Ident("u32") } } } },
                                RParen(")"),
                            },
                        },
                        RParen(")"),
                    },
                    Whitespace(" "),
                    BlockExpr { StmtList { LCurly("{"), RCurly("}") } },
                },
            }
        ),
        "fn f( {" => with_error(
            crate::scripted_tree!(
            SourceFile {
                Fn {
                    FnKw("fn"),
                    Whitespace(" "),
                    Name { Ident("f") },
                    ParamList { LParen("(") },
                    Whitespace(" "),
                    BlockExpr { StmtList { LCurly("{") } },
                },
            }
            ),
            "expected value parameter",
        ),
        _ => return None,
    })
}
