//! The trees of the sources the tests of the domain read, for use declarations.

use super::super::script::Piece;

/// The tree of `source`; none when no tree of this group is written for it.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "one arm for each source of this group, its tree written out in full: the arms are the table"
)]
pub(super) fn tree_of(source: &str) -> Option<Piece> {
    Some(match source {
        "use a as b;" => crate::scripted_tree!(
            SourceFile {
                Use {
                    UseKw("use"),
                    Whitespace(" "),
                    UseTree {
                        Path { PathSegment { NameRef { Ident("a") } } },
                        Whitespace(" "),
                        Rename { AsKw("as"), Whitespace(" "), Name { Ident("b") } },
                    },
                    Semicolon(";"),
                },
            }
        ),
        "use a::{b};" => crate::scripted_tree!(
            SourceFile {
                Use {
                    UseKw("use"),
                    Whitespace(" "),
                    UseTree {
                        Path { PathSegment { NameRef { Ident("a") } } },
                        Colon2("::"),
                        UseTreeList {
                            LCurly("{"),
                            UseTree { Path { PathSegment { NameRef { Ident("b") } } } },
                            RCurly("}"),
                        },
                    },
                    Semicolon(";"),
                },
            }
        ),
        "use a;" => crate::scripted_tree!(
            SourceFile {
                Use {
                    UseKw("use"),
                    Whitespace(" "),
                    UseTree { Path { PathSegment { NameRef { Ident("a") } } } },
                    Semicolon(";"),
                },
            }
        ),
        "use a::{b, c, d};" => crate::scripted_tree!(
            SourceFile {
                Use {
                    UseKw("use"),
                    Whitespace(" "),
                    UseTree {
                        Path { PathSegment { NameRef { Ident("a") } } },
                        Colon2("::"),
                        UseTreeList {
                            LCurly("{"),
                            UseTree { Path { PathSegment { NameRef { Ident("b") } } } },
                            Comma(","),
                            Whitespace(" "),
                            UseTree { Path { PathSegment { NameRef { Ident("c") } } } },
                            Comma(","),
                            Whitespace(" "),
                            UseTree { Path { PathSegment { NameRef { Ident("d") } } } },
                            RCurly("}"),
                        },
                    },
                    Semicolon(";"),
                },
            }
        ),
        "use a::b;" => crate::scripted_tree!(
            SourceFile {
                Use {
                    UseKw("use"),
                    Whitespace(" "),
                    UseTree {
                        Path {
                            Path { PathSegment { NameRef { Ident("a") } } },
                            Colon2("::"),
                            PathSegment { NameRef { Ident("b") } },
                        },
                    },
                    Semicolon(";"),
                },
            }
        ),
        "use crate::a::{b, c as d, self};\nuse super::*;\nfn f() { crate::m::g(); Self::x(); single(); <T as Tr>::f(); }\n" =>
        {
            crate::scripted_tree!(
                SourceFile {
                    Use {
                        UseKw("use"),
                        Whitespace(" "),
                        UseTree {
                            Path {
                                Path { PathSegment { NameRef { CrateKw("crate") } } },
                                Colon2("::"),
                                PathSegment { NameRef { Ident("a") } },
                            },
                            Colon2("::"),
                            UseTreeList {
                                LCurly("{"),
                                UseTree { Path { PathSegment { NameRef { Ident("b") } } } },
                                Comma(","),
                                Whitespace(" "),
                                UseTree {
                                    Path { PathSegment { NameRef { Ident("c") } } },
                                    Whitespace(" "),
                                    Rename { AsKw("as"), Whitespace(" "), Name { Ident("d") } },
                                },
                                Comma(","),
                                Whitespace(" "),
                                UseTree { Path { PathSegment { NameRef { SelfKw("self") } } } },
                                RCurly("}"),
                            },
                        },
                        Semicolon(";"),
                    },
                    Whitespace("\n"),
                    Use {
                        UseKw("use"),
                        Whitespace(" "),
                        UseTree { Path { PathSegment { NameRef { SuperKw("super") } } }, Colon2("::"), Star("*") },
                        Semicolon(";"),
                    },
                    Whitespace("\n"),
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
                                        PathExpr {
                                            Path {
                                                Path {
                                                    Path { PathSegment { NameRef { CrateKw("crate") } } },
                                                    Colon2("::"),
                                                    PathSegment { NameRef { Ident("m") } },
                                                },
                                                Colon2("::"),
                                                PathSegment { NameRef { Ident("g") } },
                                            },
                                        },
                                        ArgList { LParen("("), RParen(")") },
                                    },
                                    Semicolon(";"),
                                },
                                Whitespace(" "),
                                ExprStmt {
                                    CallExpr {
                                        PathExpr {
                                            Path {
                                                Path { PathSegment { NameRef { SelfTypeKw("Self") } } },
                                                Colon2("::"),
                                                PathSegment { NameRef { Ident("x") } },
                                            },
                                        },
                                        ArgList { LParen("("), RParen(")") },
                                    },
                                    Semicolon(";"),
                                },
                                Whitespace(" "),
                                ExprStmt {
                                    CallExpr {
                                        PathExpr { Path { PathSegment { NameRef { Ident("single") } } } },
                                        ArgList { LParen("("), RParen(")") },
                                    },
                                    Semicolon(";"),
                                },
                                Whitespace(" "),
                                ExprStmt {
                                    CallExpr {
                                        PathExpr {
                                            Path {
                                                Path {
                                                    PathSegment {
                                                        TypeAnchor {
                                                            LAngle("<"),
                                                            PathType { Path { PathSegment { NameRef { Ident("T") } } } },
                                                            Whitespace(" "),
                                                            AsKw("as"),
                                                            Whitespace(" "),
                                                            PathType { Path { PathSegment { NameRef { Ident("Tr") } } } },
                                                            RAngle(">"),
                                                        },
                                                    },
                                                },
                                                Colon2("::"),
                                                PathSegment { NameRef { Ident("f") } },
                                            },
                                        },
                                        ArgList { LParen("("), RParen(")") },
                                    },
                                    Semicolon(";"),
                                },
                                Whitespace(" "),
                                RCurly("}"),
                            },
                        },
                    },
                    Whitespace("\n"),
                }
            )
        }
        "use crate::a::{b, c};\nfn f() { crate :: m\n    ::g::<u8>(); }\n" => {
            crate::scripted_tree!(
                SourceFile {
                    Use {
                        UseKw("use"),
                        Whitespace(" "),
                        UseTree {
                            Path {
                                Path { PathSegment { NameRef { CrateKw("crate") } } },
                                Colon2("::"),
                                PathSegment { NameRef { Ident("a") } },
                            },
                            Colon2("::"),
                            UseTreeList {
                                LCurly("{"),
                                UseTree { Path { PathSegment { NameRef { Ident("b") } } } },
                                Comma(","),
                                Whitespace(" "),
                                UseTree { Path { PathSegment { NameRef { Ident("c") } } } },
                                RCurly("}"),
                            },
                        },
                        Semicolon(";"),
                    },
                    Whitespace("\n"),
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
                                        PathExpr {
                                            Path {
                                                Path {
                                                    Path { PathSegment { NameRef { CrateKw("crate") } } },
                                                    Whitespace(" "),
                                                    Colon2("::"),
                                                    Whitespace(" "),
                                                    PathSegment { NameRef { Ident("m") } },
                                                },
                                                Whitespace("\n    "),
                                                Colon2("::"),
                                                PathSegment {
                                                    NameRef { Ident("g") },
                                                    GenericArgList {
                                                        Colon2("::"),
                                                        LAngle("<"),
                                                        TypeArg {
                                                            PathType { Path { PathSegment { NameRef { Ident("u8") } } } },
                                                        },
                                                        RAngle(">"),
                                                    },
                                                },
                                            },
                                        },
                                        ArgList { LParen("("), RParen(")") },
                                    },
                                    Semicolon(";"),
                                },
                                Whitespace(" "),
                                RCurly("}"),
                            },
                        },
                    },
                    Whitespace("\n"),
                }
            )
        }
        _ => return None,
    })
}
