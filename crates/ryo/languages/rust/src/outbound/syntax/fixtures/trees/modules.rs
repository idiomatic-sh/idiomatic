//! The trees of the sources the tests of the domain read, for module declarations and paths.

use super::super::script::Piece;

/// The tree of `source`; none when no tree of this group is written for it.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "one arm for each source of this group, its tree written out in full: the arms are the table"
)]
pub(super) fn tree_of(source: &str) -> Option<Piece> {
    Some(match source {
        "fn f() { a::b(); }" => crate::scripted_tree!(
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
                                    PathExpr {
                                        Path {
                                            Path { PathSegment { NameRef { Ident("a") } } },
                                            Colon2("::"),
                                            PathSegment { NameRef { Ident("b") } },
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
            }
        ),
        "fn f() { g::<u8>(); }" => crate::scripted_tree!(
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
                                    PathExpr {
                                        Path {
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
            }
        ),
        "mod m { fn f() {} }" => crate::scripted_tree!(
            SourceFile {
                Module {
                    ModKw("mod"),
                    Whitespace(" "),
                    Name { Ident("m") },
                    Whitespace(" "),
                    ItemList {
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
        "fn f() { a::g(); }" => crate::scripted_tree!(
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
                                    PathExpr {
                                        Path {
                                            Path { PathSegment { NameRef { Ident("a") } } },
                                            Colon2("::"),
                                            PathSegment { NameRef { Ident("g") } },
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
            }
        ),
        "mod r#type;\nuse crate::r#type::r#enum;\n" => crate::scripted_tree!(
            SourceFile {
                Module { ModKw("mod"), Whitespace(" "), Name { Ident("r#type") }, Semicolon(";") },
                Whitespace("\n"),
                Use {
                    UseKw("use"),
                    Whitespace(" "),
                    UseTree {
                        Path {
                            Path {
                                Path { PathSegment { NameRef { CrateKw("crate") } } },
                                Colon2("::"),
                                PathSegment { NameRef { Ident("r#type") } },
                            },
                            Colon2("::"),
                            PathSegment { NameRef { Ident("r#enum") } },
                        },
                    },
                    Semicolon(";"),
                },
                Whitespace("\n"),
            }
        ),
        "mod a;\nmod b { mod c; }\n#[path = \"x/y.rs\"] mod d;\n" => crate::scripted_tree!(
            SourceFile {
                Module { ModKw("mod"), Whitespace(" "), Name { Ident("a") }, Semicolon(";") },
                Whitespace("\n"),
                Module {
                    ModKw("mod"),
                    Whitespace(" "),
                    Name { Ident("b") },
                    Whitespace(" "),
                    ItemList {
                        LCurly("{"),
                        Whitespace(" "),
                        Module { ModKw("mod"), Whitespace(" "), Name { Ident("c") }, Semicolon(";") },
                        Whitespace(" "),
                        RCurly("}"),
                    },
                },
                Whitespace("\n"),
                Module {
                    Attr {
                        Pound("#"),
                        LBrack("["),
                        KeyValueMeta {
                            Path { PathSegment { NameRef { Ident("path") } } },
                            Whitespace(" "),
                            Eq("="),
                            Whitespace(" "),
                            Literal { String("\"x/y.rs\"") },
                        },
                        RBrack("]"),
                    },
                    Whitespace(" "),
                    ModKw("mod"),
                    Whitespace(" "),
                    Name { Ident("d") },
                    Semicolon(";"),
                },
                Whitespace("\n"),
            }
        ),
        "mod outer { mod inner { fn f() { self::a::b(); } } }\n" => crate::scripted_tree!(
            SourceFile {
                Module {
                    ModKw("mod"),
                    Whitespace(" "),
                    Name { Ident("outer") },
                    Whitespace(" "),
                    ItemList {
                        LCurly("{"),
                        Whitespace(" "),
                        Module {
                            ModKw("mod"),
                            Whitespace(" "),
                            Name { Ident("inner") },
                            Whitespace(" "),
                            ItemList {
                                LCurly("{"),
                                Whitespace(" "),
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
                                                                Path { PathSegment { NameRef { SelfKw("self") } } },
                                                                Colon2("::"),
                                                                PathSegment { NameRef { Ident("a") } },
                                                            },
                                                            Colon2("::"),
                                                            PathSegment { NameRef { Ident("b") } },
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
                                Whitespace(" "),
                                RCurly("}"),
                            },
                        },
                        Whitespace(" "),
                        RCurly("}"),
                    },
                },
                Whitespace("\n"),
            }
        ),
        _ => return None,
    })
}
