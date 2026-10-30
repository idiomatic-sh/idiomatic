//! The `constraints` service's cases: which comments of a file are build constraint lines, as `go/build` reads its header.

use super::service::constraint_at;
use super::types::Constraint;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::syntax::service::kind;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use crate::fixtures::reading::parsed;
use ryo::languages::{self, ByteSlice, Tree};

/// A source.
type Input = &'static str;
/// What each comment of it is, in order.
type Value = &'static [Constraint];
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_go_build_line_in_the_header_is_a_constraint"),
        word: Word::Succeeds,
        input: || "//go:build linux && amd64\n\npackage p\n",
        expect: Expect::Value(&[Constraint::GoBuild]),
    },
    Case {
        name: CaseName("a_go_build_line_needs_no_blank_line_after_it"),
        word: Word::Succeeds,
        input: || "// Copyright.\n//go:build linux\npackage p\n",
        expect: Expect::Value(&[Constraint::Other, Constraint::GoBuild]),
    },
    Case {
        name: CaseName("a_plus_build_line_above_a_blank_line_is_a_constraint"),
        word: Word::Succeeds,
        input: || "// +build linux darwin\n//+build !cgo\n\n// Package p.\npackage p\n",
        expect: Expect::Value(&[
            Constraint::PlusBuild,
            Constraint::PlusBuild,
            Constraint::Other,
        ]),
    },
    Case {
        name: CaseName(
            "a_plus_build_line_with_no_blank_line_before_the_package_is_package_documentation",
        ),
        word: Word::Succeeds,
        input: || "// +build linux\npackage p\n",
        expect: Expect::Value(&[Constraint::Other]),
    },
    Case {
        name: CaseName("a_plus_build_line_after_a_block_comment_is_no_constraint"),
        word: Word::Succeeds,
        input: || "/* licence */\n// +build linux\n\npackage p\n",
        expect: Expect::Value(&[Constraint::Other, Constraint::Other]),
    },
    Case {
        name: CaseName("a_go_build_line_after_a_block_comment_is_a_constraint"),
        word: Word::Succeeds,
        input: || "/* licence\n*/\n//go:build linux\n\npackage p\n",
        expect: Expect::Value(&[Constraint::Other, Constraint::GoBuild]),
    },
    Case {
        name: CaseName("a_constraint_line_after_the_package_clause_is_no_constraint"),
        word: Word::Succeeds,
        input: || "package p\n\n//go:build linux\n\n// +build linux\n\nvar x = 1\n",
        expect: Expect::Value(&[Constraint::Other, Constraint::Other]),
    },
    Case {
        name: CaseName("a_word_that_only_starts_with_build_is_no_constraint"),
        word: Word::Succeeds,
        input: || "//go:buildx linux\n// +builds linux\n// go:build linux\n\npackage p\n",
        expect: Expect::Value(&[Constraint::Other, Constraint::Other, Constraint::Other]),
    },
    Case {
        name: CaseName("a_bare_constraint_word_is_a_constraint"),
        word: Word::Succeeds,
        input: || "//go:build\n// +build\n\npackage p\n",
        expect: Expect::Value(&[Constraint::GoBuild, Constraint::PlusBuild]),
    },
    Case {
        name: CaseName("a_constraint_line_after_text_on_its_line_is_no_constraint"),
        word: Word::Succeeds,
        input: || "/* a */ //go:build linux\n\npackage p\n",
        expect: Expect::Value(&[Constraint::Other, Constraint::Other]),
    },
    Case {
        name: CaseName("a_constraint_line_with_carriage_returns_is_a_constraint"),
        word: Word::Succeeds,
        input: || "//go:build linux\r\n// +build linux\r\n\r\npackage p\r\n",
        expect: Expect::Value(&[Constraint::GoBuild, Constraint::PlusBuild]),
    },
    Case {
        name: CaseName("a_go_build_line_with_a_tab_after_the_word_is_a_constraint"),
        word: Word::Succeeds,
        input: || "//go:build\tlinux\n\npackage p\n",
        expect: Expect::Value(&[Constraint::GoBuild]),
    },
    Case {
        name: CaseName("a_plus_build_line_with_many_spaces_is_a_constraint"),
        word: Word::Succeeds,
        input: || "//   +build\n\npackage p\n",
        expect: Expect::Value(&[Constraint::PlusBuild]),
    },
    Case {
        name: CaseName("unicode_white_space_after_the_word_ends_it_as_go_trims_it"),
        word: Word::Succeeds,
        input: || "//go:build\u{a0}linux\u{85}\n//\u{a0}+build\u{2003}linux\n\npackage p\n",
        expect: Expect::Value(&[Constraint::GoBuild, Constraint::PlusBuild]),
    },
    Case {
        name: CaseName("a_vertical_tab_after_the_word_ends_it_as_go_trims_it"),
        word: Word::Succeeds,
        input: || "//go:build\u{b}linux\n\npackage p\n",
        expect: Expect::Value(&[Constraint::GoBuild]),
    },
    Case {
        name: CaseName("a_general_comment_is_no_constraint"),
        word: Word::Succeeds,
        input: || "/* +build linux */\n/*go:build linux*/\n\npackage p\n",
        expect: Expect::Value(&[Constraint::Other, Constraint::Other]),
    },
];

fn subject(source: Input) -> Result<Value, Failure> {
    let mut file = parsed(source.as_bytes()).map_err(drop)?;
    let elements = std::mem::take(&mut file.elements);
    let tree = Tree {
        elements: &elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let found: Vec<Constraint> = languages::positions(&elements)
        .filter(|at| kind(&tree, *at) == Kind::Token(Token::Comment))
        .map(|at| constraint_at(&tree, at))
        .collect();
    Ok(&*Box::leak(found.into_boxed_slice()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
