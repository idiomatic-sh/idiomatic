//! The trees of the sources the tests of the domain read, for control flow: conditions, loops and matches.

use super::super::script::Piece;

/// The tree of `source`; none when no tree of this group is written for it.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "one arm for each source of this group, its tree written out in full: the arms are the table"
)]
pub(super) fn tree_of(source: &str) -> Option<Piece> {
    Some(match source {
        "fn f() { 'a: loop {} }" => crate::scripted_tree!(
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
                            LoopExpr {
                                Label { Lifetime { LifetimeIdent("'a") }, Colon(":") },
                                Whitespace(" "),
                                LoopKw("loop"),
                                Whitespace(" "),
                                BlockExpr { StmtList { LCurly("{"), RCurly("}") } },
                            },
                            Whitespace(" "),
                            RCurly("}"),
                        },
                    },
                },
            }
        ),
        "fn f() { match x { _ => 1 } }" => crate::scripted_tree!(
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
                            MatchExpr {
                                MatchKw("match"),
                                Whitespace(" "),
                                PathExpr { Path { PathSegment { NameRef { Ident("x") } } } },
                                Whitespace(" "),
                                MatchArmList {
                                    LCurly("{"),
                                    Whitespace(" "),
                                    MatchArm {
                                        WildcardPat { Underscore("_") },
                                        Whitespace(" "),
                                        FatArrow("=>"),
                                        Whitespace(" "),
                                        Literal { IntNumber("1") },
                                    },
                                    Whitespace(" "),
                                    RCurly("}"),
                                },
                            },
                            Whitespace(" "),
                            RCurly("}"),
                        },
                    },
                },
            }
        ),
        "fn f() { let a = b else { return }; }" => crate::scripted_tree!(
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
                            LetStmt {
                                LetKw("let"),
                                Whitespace(" "),
                                IdentPat { Name { Ident("a") } },
                                Whitespace(" "),
                                Eq("="),
                                Whitespace(" "),
                                PathExpr { Path { PathSegment { NameRef { Ident("b") } } } },
                                Whitespace(" "),
                                LetElse {
                                    ElseKw("else"),
                                    Whitespace(" "),
                                    BlockExpr {
                                        StmtList {
                                            LCurly("{"),
                                            Whitespace(" "),
                                            ReturnExpr { ReturnKw("return") },
                                            Whitespace(" "),
                                            RCurly("}"),
                                        },
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
        "fn f() { if a {} }" => crate::scripted_tree!(
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
                            IfExpr {
                                IfKw("if"),
                                Whitespace(" "),
                                PathExpr { Path { PathSegment { NameRef { Ident("a") } } } },
                                Whitespace(" "),
                                BlockExpr { StmtList { LCurly("{"), RCurly("}") } },
                            },
                            Whitespace(" "),
                            RCurly("}"),
                        },
                    },
                },
            }
        ),
        "fn f() { if a {} else {} }" => crate::scripted_tree!(
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
                            IfExpr {
                                IfKw("if"),
                                Whitespace(" "),
                                PathExpr { Path { PathSegment { NameRef { Ident("a") } } } },
                                Whitespace(" "),
                                BlockExpr { StmtList { LCurly("{"), RCurly("}") } },
                                Whitespace(" "),
                                ElseKw("else"),
                                Whitespace(" "),
                                BlockExpr { StmtList { LCurly("{"), RCurly("}") } },
                            },
                            Whitespace(" "),
                            RCurly("}"),
                        },
                    },
                },
            }
        ),
        "fn f() { loop {} }" => crate::scripted_tree!(
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
                            LoopExpr {
                                LoopKw("loop"),
                                Whitespace(" "),
                                BlockExpr { StmtList { LCurly("{"), RCurly("}") } },
                            },
                            Whitespace(" "),
                            RCurly("}"),
                        },
                    },
                },
            }
        ),
        "fn f() { for a in b {} }" => crate::scripted_tree!(
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
                            ForExpr {
                                ForKw("for"),
                                Whitespace(" "),
                                IdentPat { Name { Ident("a") } },
                                Whitespace(" "),
                                InKw("in"),
                                Whitespace(" "),
                                PathExpr { Path { PathSegment { NameRef { Ident("b") } } } },
                                Whitespace(" "),
                                BlockExpr { StmtList { LCurly("{"), RCurly("}") } },
                            },
                            Whitespace(" "),
                            RCurly("}"),
                        },
                    },
                },
            }
        ),
        "fn f() { match x { _ if y => 1 } }" => crate::scripted_tree!(
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
                            MatchExpr {
                                MatchKw("match"),
                                Whitespace(" "),
                                PathExpr { Path { PathSegment { NameRef { Ident("x") } } } },
                                Whitespace(" "),
                                MatchArmList {
                                    LCurly("{"),
                                    Whitespace(" "),
                                    MatchArm {
                                        WildcardPat { Underscore("_") },
                                        Whitespace(" "),
                                        MatchGuard {
                                            IfKw("if"),
                                            Whitespace(" "),
                                            PathExpr { Path { PathSegment { NameRef { Ident("y") } } } },
                                        },
                                        Whitespace(" "),
                                        FatArrow("=>"),
                                        Whitespace(" "),
                                        Literal { IntNumber("1") },
                                    },
                                    Whitespace(" "),
                                    RCurly("}"),
                                },
                            },
                            Whitespace(" "),
                            RCurly("}"),
                        },
                    },
                },
            }
        ),
        "impl A for B {}" => crate::scripted_tree!(
            SourceFile {
                Impl {
                    ImplKw("impl"),
                    Whitespace(" "),
                    PathType { Path { PathSegment { NameRef { Ident("A") } } } },
                    Whitespace(" "),
                    ForKw("for"),
                    Whitespace(" "),
                    PathType { Path { PathSegment { NameRef { Ident("B") } } } },
                    Whitespace(" "),
                    AssocItemList { LCurly("{"), RCurly("}") },
                },
            }
        ),
        _ => return None,
    })
}
