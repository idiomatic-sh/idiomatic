//! The trees of the sources the tests of the domain read, for let bindings.

use super::super::script::Piece;

/// The tree of `source`; none when no tree of this group is written for it.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "one arm for each source of this group, its tree written out in full: the arms are the table"
)]
pub(super) fn tree_of(source: &str) -> Option<Piece> {
    Some(match source {
        "fn f() { let S { a } = s; }" => crate::scripted_tree!(
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
                                RecordPat {
                                    Path { PathSegment { NameRef { Ident("S") } } },
                                    Whitespace(" "),
                                    RecordPatFieldList {
                                        LCurly("{"),
                                        Whitespace(" "),
                                        RecordPatField { IdentPat { Name { Ident("a") } } },
                                        Whitespace(" "),
                                        RCurly("}"),
                                    },
                                },
                                Whitespace(" "),
                                Eq("="),
                                Whitespace(" "),
                                PathExpr { Path { PathSegment { NameRef { Ident("s") } } } },
                                Semicolon(";"),
                            },
                            Whitespace(" "),
                            RCurly("}"),
                        },
                    },
                },
            }
        ),
        "fn f() { let a = 1; }" => crate::scripted_tree!(
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
                                Literal { IntNumber("1") },
                                Semicolon(";"),
                            },
                            Whitespace(" "),
                            RCurly("}"),
                        },
                    },
                },
            }
        ),
        "fn f() { let x = 1; }" => crate::scripted_tree!(
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
                                IdentPat { Name { Ident("x") } },
                                Whitespace(" "),
                                Eq("="),
                                Whitespace(" "),
                                Literal { IntNumber("1") },
                                Semicolon(";"),
                            },
                            Whitespace(" "),
                            RCurly("}"),
                        },
                    },
                },
            }
        ),
        "fn f() { let MAX = 1; }" => crate::scripted_tree!(
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
                                IdentPat { Name { Ident("MAX") } },
                                Whitespace(" "),
                                Eq("="),
                                Whitespace(" "),
                                Literal { IntNumber("1") },
                                Semicolon(";"),
                            },
                            Whitespace(" "),
                            RCurly("}"),
                        },
                    },
                },
            }
        ),
        _ => return None,
    })
}
