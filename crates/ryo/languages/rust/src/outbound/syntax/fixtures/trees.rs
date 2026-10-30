// ignore: idiomatic.arch/file-length - the trees of the sources the domain's tests read, written out in full, one arm for each: splitting them would split the one table `SOURCES` is the list of
//! The trees of the sources the tests of the domain read, written by hand in the language's kinds, so those tests depend on no parser.
//! The adapter of the port is held to them: its test makes the table of each source and compares it element by element.

use super::script::{Piece, with_error};

/// Every source that has a tree here.
pub(crate) const SOURCES: [&str; 61] = [
    "fn f() {}",
    "fn f() { g(); }",
    "fn f() { a::b(); }",
    "pub fn f() {}",
    "#[a] fn f() {}",
    "fn f<'a>() {}",
    "fn f() { 'a: loop {} }",
    "extern \"C\" fn f() {}",
    "use a as b;",
    "fn f<T>() {}",
    "fn f() { g::<u8>(); }",
    "fn f() where T: A {}",
    "fn f(a: u8) {}",
    "fn f() -> u8 {}",
    "fn f() { g(1); }",
    "mod m { fn f() {} }",
    "impl T { fn f() {} }",
    "struct S { a: u8 }",
    "struct S(u8);",
    "enum E { A }",
    "fn f() { match x { _ => 1 } }",
    "use a::{b};",
    "fn f() { S { a: 1 }; }",
    "fn f() { let S { a } = s; }",
    "use a;",
    "fn f() { let a = b else { return }; }",
    "fn f() { let a = 1; }",
    "fn f() { 1 }",
    "fn f() { x.y; }",
    "fn f<T: A>() {}",
    "fn f() { if a {} }",
    "fn f() { if a {} else {} }",
    "fn f() { loop {} }",
    "fn f() { for a in b {} }",
    "fn f() { a.b(); }",
    "fn f() { a + b; }",
    "fn f() { a[b]; }",
    "fn f() { a..b; }",
    "fn f() { S { ..a }; }",
    "fn f() { match x { _ if y => 1 } }",
    "impl A for B {}",
    "fn f<T = u8>() {}",
    "fn f<const N: usize = 1>() {}",
    "#[cfg(unix)] fn f() {}",
    "fn one() {}\nstruct S;\nimpl S {\n    fn two(&self) {}\n}\n",
    "fn run() {}",
    "fn run(value: u8) {}",
    "fn run() -> u8 { 0 }",
    "fn f() { let x = 1; }",
    "fn f(a: u32) {}",
    "fn f((a, b): (u32, u32)) {}",
    "fn f() { let MAX = 1; }",
    "use a::{b, c, d};",
    "use a::b;",
    "fn f() { a::g(); }",
    "mod a;\nmod b { mod c; }\n#[path = \"x/y.rs\"] mod d;\n",
    "mod r#type;\nuse crate::r#type::r#enum;\n",
    "use crate::a::{b, c as d, self};\nuse super::*;\nfn f() { crate::m::g(); Self::x(); single(); <T as Tr>::f(); }\n",
    "mod outer { mod inner { fn f() { self::a::b(); } } }\n",
    "fn f( {",
    "use crate::a::{b, c};\nfn f() { crate :: m\n    ::g::<u8>(); }\n",
];

/// The tree of `source`; none when no tree is written for it.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "one arm for each source the domain's tests read, its tree written out in full: the arms are the table `SOURCES` lists, which a split would divide"
)]
pub(crate) fn tree_of(source: &str) -> Option<Piece> {
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
