//! The trees of the sources the tests of the domain read, for expressions: calls, operators, indexing, ranges and struct literals.

use super::super::script::Piece;

/// The tree of `source`; none when no tree of this group is written for it.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "one arm for each source of this group, its tree written out in full: the arms are the table"
)]
pub(super) fn tree_of(source: &str) -> Option<Piece> {
    Some(match source {
        "fn f() { g(); }" => crate::scripted_tree!(
            SourceFile {
                Fn {
                    FnKw("fn"),
                    Whitespace(" "),
                    Name { Ident("f") },
                    ParamList { LParen("("), RParen(")") },
                    Whitespace(" "),
                    BlockExpr {
                        StmtList {
                            LCurly("{"),
                            Whitespace(" "),
                            ExprStmt {
                                CallExpr {
                                    PathExpr { Path { PathSegment { NameRef { Ident("g") } } } },
                                    ArgList { LParen("("), RParen(")") },
                                },
                                Semicolon(";"),
                            },
                            Whitespace(" "),
                            RCurly("}"),
                        },
                    },
                },
            }
        ),
        "#[a] fn f() {}" => crate::scripted_tree!(
            SourceFile {
                Fn {
                    Attr {
                        Pound("#"),
                        LBrack("["),
                        PathMeta { Path { PathSegment { NameRef { Ident("a") } } } },
                        RBrack("]"),
                    },
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
        "fn f() { g(1); }" => crate::scripted_tree!(
            SourceFile {
                Fn {
                    FnKw("fn"),
                    Whitespace(" "),
                    Name { Ident("f") },
                    ParamList { LParen("("), RParen(")") },
                    Whitespace(" "),
                    BlockExpr {
                        StmtList {
                            LCurly("{"),
                            Whitespace(" "),
                            ExprStmt {
                                CallExpr {
                                    PathExpr { Path { PathSegment { NameRef { Ident("g") } } } },
                                    ArgList { LParen("("), Literal { IntNumber("1") }, RParen(")") },
                                },
                                Semicolon(";"),
                            },
                            Whitespace(" "),
                            RCurly("}"),
                        },
                    },
                },
            }
        ),
        "fn f() { S { a: 1 }; }" => crate::scripted_tree!(
            SourceFile {
                Fn {
                    FnKw("fn"),
                    Whitespace(" "),
                    Name { Ident("f") },
                    ParamList { LParen("("), RParen(")") },
                    Whitespace(" "),
                    BlockExpr {
                        StmtList {
                            LCurly("{"),
                            Whitespace(" "),
                            ExprStmt {
                                RecordExpr {
                                    Path { PathSegment { NameRef { Ident("S") } } },
                                    Whitespace(" "),
                                    RecordExprFieldList {
                                        LCurly("{"),
                                        Whitespace(" "),
                                        RecordExprField {
                                            NameRef { Ident("a") },
                                            Colon(":"),
                                            Whitespace(" "),
                                            Literal { IntNumber("1") },
                                        },
                                        Whitespace(" "),
                                        RCurly("}"),
                                    },
                                },
                                Semicolon(";"),
                            },
                            Whitespace(" "),
                            RCurly("}"),
                        },
                    },
                },
            }
        ),
        "fn f() { 1 }" => crate::scripted_tree!(
            SourceFile {
                Fn {
                    FnKw("fn"),
                    Whitespace(" "),
                    Name { Ident("f") },
                    ParamList { LParen("("), RParen(")") },
                    Whitespace(" "),
                    BlockExpr {
                        StmtList {
                            LCurly("{"),
                            Whitespace(" "),
                            Literal { IntNumber("1") },
                            Whitespace(" "),
                            RCurly("}"),
                        },
                    },
                },
            }
        ),
        "fn f() { x.y; }" => crate::scripted_tree!(
            SourceFile {
                Fn {
                    FnKw("fn"),
                    Whitespace(" "),
                    Name { Ident("f") },
                    ParamList { LParen("("), RParen(")") },
                    Whitespace(" "),
                    BlockExpr {
                        StmtList {
                            LCurly("{"),
                            Whitespace(" "),
                            ExprStmt {
                                FieldExpr {
                                    PathExpr { Path { PathSegment { NameRef { Ident("x") } } } },
                                    Dot("."),
                                    NameRef { Ident("y") },
                                },
                                Semicolon(";"),
                            },
                            Whitespace(" "),
                            RCurly("}"),
                        },
                    },
                },
            }
        ),
        "fn f() { a.b(); }" => crate::scripted_tree!(
            SourceFile {
                Fn {
                    FnKw("fn"),
                    Whitespace(" "),
                    Name { Ident("f") },
                    ParamList { LParen("("), RParen(")") },
                    Whitespace(" "),
                    BlockExpr {
                        StmtList {
                            LCurly("{"),
                            Whitespace(" "),
                            ExprStmt {
                                MethodCallExpr {
                                    PathExpr { Path { PathSegment { NameRef { Ident("a") } } } },
                                    Dot("."),
                                    NameRef { Ident("b") },
                                    ArgList { LParen("("), RParen(")") },
                                },
                                Semicolon(";"),
                            },
                            Whitespace(" "),
                            RCurly("}"),
                        },
                    },
                },
            }
        ),
        "fn f() { a + b; }" => crate::scripted_tree!(
            SourceFile {
                Fn {
                    FnKw("fn"),
                    Whitespace(" "),
                    Name { Ident("f") },
                    ParamList { LParen("("), RParen(")") },
                    Whitespace(" "),
                    BlockExpr {
                        StmtList {
                            LCurly("{"),
                            Whitespace(" "),
                            ExprStmt {
                                BinExpr {
                                    PathExpr { Path { PathSegment { NameRef { Ident("a") } } } },
                                    Whitespace(" "),
                                    Plus("+"),
                                    Whitespace(" "),
                                    PathExpr { Path { PathSegment { NameRef { Ident("b") } } } },
                                },
                                Semicolon(";"),
                            },
                            Whitespace(" "),
                            RCurly("}"),
                        },
                    },
                },
            }
        ),
        "fn f() { a[b]; }" => crate::scripted_tree!(
            SourceFile {
                Fn {
                    FnKw("fn"),
                    Whitespace(" "),
                    Name { Ident("f") },
                    ParamList { LParen("("), RParen(")") },
                    Whitespace(" "),
                    BlockExpr {
                        StmtList {
                            LCurly("{"),
                            Whitespace(" "),
                            ExprStmt {
                                IndexExpr {
                                    PathExpr { Path { PathSegment { NameRef { Ident("a") } } } },
                                    LBrack("["),
                                    PathExpr { Path { PathSegment { NameRef { Ident("b") } } } },
                                    RBrack("]"),
                                },
                                Semicolon(";"),
                            },
                            Whitespace(" "),
                            RCurly("}"),
                        },
                    },
                },
            }
        ),
        "fn f() { a..b; }" => crate::scripted_tree!(
            SourceFile {
                Fn {
                    FnKw("fn"),
                    Whitespace(" "),
                    Name { Ident("f") },
                    ParamList { LParen("("), RParen(")") },
                    Whitespace(" "),
                    BlockExpr {
                        StmtList {
                            LCurly("{"),
                            Whitespace(" "),
                            ExprStmt {
                                RangeExpr {
                                    PathExpr { Path { PathSegment { NameRef { Ident("a") } } } },
                                    Dot2(".."),
                                    PathExpr { Path { PathSegment { NameRef { Ident("b") } } } },
                                },
                                Semicolon(";"),
                            },
                            Whitespace(" "),
                            RCurly("}"),
                        },
                    },
                },
            }
        ),
        "fn f() { S { ..a }; }" => crate::scripted_tree!(
            SourceFile {
                Fn {
                    FnKw("fn"),
                    Whitespace(" "),
                    Name { Ident("f") },
                    ParamList { LParen("("), RParen(")") },
                    Whitespace(" "),
                    BlockExpr {
                        StmtList {
                            LCurly("{"),
                            Whitespace(" "),
                            ExprStmt {
                                RecordExpr {
                                    Path { PathSegment { NameRef { Ident("S") } } },
                                    Whitespace(" "),
                                    RecordExprFieldList {
                                        LCurly("{"),
                                        Whitespace(" "),
                                        Dot2(".."),
                                        PathExpr { Path { PathSegment { NameRef { Ident("a") } } } },
                                        Whitespace(" "),
                                        RCurly("}"),
                                    },
                                },
                                Semicolon(";"),
                            },
                            Whitespace(" "),
                            RCurly("}"),
                        },
                    },
                },
            }
        ),
        "#[cfg(unix)] fn f() {}" => crate::scripted_tree!(
            SourceFile {
                Fn {
                    Attr {
                        Pound("#"),
                        LBrack("["),
                        CfgMeta { CfgKw("cfg"), LParen("("), CfgAtom { Ident("unix") }, RParen(")") },
                        RBrack("]"),
                    },
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
        "fn one() {}\nstruct S;\nimpl S {\n    fn two(&self) {}\n}\n" => crate::scripted_tree!(
            SourceFile {
                Fn {
                    FnKw("fn"),
                    Whitespace(" "),
                    Name { Ident("one") },
                    ParamList { LParen("("), RParen(")") },
                    Whitespace(" "),
                    BlockExpr { StmtList { LCurly("{"), RCurly("}") } },
                },
                Whitespace("\n"),
                Struct { StructKw("struct"), Whitespace(" "), Name { Ident("S") }, Semicolon(";") },
                Whitespace("\n"),
                Impl {
                    ImplKw("impl"),
                    Whitespace(" "),
                    PathType { Path { PathSegment { NameRef { Ident("S") } } } },
                    Whitespace(" "),
                    AssocItemList {
                        LCurly("{"),
                        Whitespace("\n    "),
                        Fn {
                            FnKw("fn"),
                            Whitespace(" "),
                            Name { Ident("two") },
                            ParamList { LParen("("), SelfParam { Amp("&"), Name { SelfKw("self") } }, RParen(")") },
                            Whitespace(" "),
                            BlockExpr { StmtList { LCurly("{"), RCurly("}") } },
                        },
                        Whitespace("\n"),
                        RCurly("}"),
                    },
                },
                Whitespace("\n"),
            }
        ),
        _ => return None,
    })
}
