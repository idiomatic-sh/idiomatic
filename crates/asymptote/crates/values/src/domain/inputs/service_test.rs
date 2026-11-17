//! What the inputs digest: each file's path and stamp in path order, an edit moving only the pathspecs that name it;
//! and which files of a repository's listing a pathspec written from a product names.

use std::collections::BTreeMap;

use super::service::{beneath, named};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::manifest::types::Pathspec;
use crate::domain::reports::types::Digest;
use crate::domain::reports::types::HashParts;
use crate::domain::violations::types::RepoPath;

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// The digest of `src`'s two files, given out of order, as a readable hash writes it.
    Digest,
    /// After an edit of `src/a.rs`: whether the digest of `docs` kept and that of `src` moved.
    Edited,
    /// What `.` names from `crates/p`.
    Root,
    /// What `src/` names from `crates/p`: its files, not those of `src-old` beside it.
    Directory,
    /// What `Cargo.toml` names from `crates/p`.
    File,
    /// What `../q` names from `crates/p`: the files of a sibling product, written from this one.
    Outside,
    /// What `../*/Cargo.toml` names from `crates/p`: every product's manifest.
    Wildcard,
}

type Input = Question;
type Value = std::borrow::Cow<'static, str>;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_digest_is_each_files_path_and_stamp_in_path_order"),
        word: Word::Succeeds,
        input: || Question::Digest,
        expect: Expect::Value(std::borrow::Cow::Borrowed("[src/a.rs|a1|src/b.rs|b1]")),
    },
    Case {
        name: CaseName("an_edit_moves_only_the_pathspecs_that_name_it"),
        word: Word::Succeeds,
        input: || Question::Edited,
        expect: Expect::Value(std::borrow::Cow::Borrowed("docs kept, src moved")),
    },
    Case {
        name: CaseName("the_root_pathspec_names_every_file_of_the_product"),
        word: Word::Succeeds,
        input: || Question::Root,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "Cargo.toml src-old/c.rs src/a.rs src/b.rs",
        )),
    },
    Case {
        name: CaseName("a_directory_names_the_files_beneath_it_and_no_sibling"),
        word: Word::Succeeds,
        input: || Question::Directory,
        expect: Expect::Value(std::borrow::Cow::Borrowed("src/a.rs src/b.rs")),
    },
    Case {
        name: CaseName("a_file_names_itself"),
        word: Word::Succeeds,
        input: || Question::File,
        expect: Expect::Value(std::borrow::Cow::Borrowed("Cargo.toml")),
    },
    Case {
        name: CaseName("a_path_out_of_the_product_names_its_files_from_the_product"),
        word: Word::Succeeds,
        input: || Question::Outside,
        expect: Expect::Value(std::borrow::Cow::Borrowed("../q/Cargo.toml ../q/lib.rs")),
    },
    Case {
        name: CaseName("wildcards_match_across_directories"),
        word: Word::Succeeds,
        input: || Question::Wildcard,
        expect: Expect::Value(std::borrow::Cow::Borrowed("Cargo.toml ../q/Cargo.toml")),
    },
];

/// A readable digest: the parts joined by `|`, in brackets.
fn readable(parts: HashParts<'_>) -> Digest {
    let joined: Vec<String> = parts
        .0
        .into_iter()
        .map(|part| String::from_utf8_lossy(part).into_owned())
        .collect();
    Digest(format!("[{}]", joined.join("|")).into())
}

/// Three files in two directories, `src/a.rs` stamped `edited`.
fn stamps(edited: &str) -> BTreeMap<RepoPath, Digest> {
    [
        ("src/a.rs", edited),
        ("src/b.rs", "b1"),
        ("docs/notes.md", "n1"),
    ]
    .into_iter()
    .map(|(path, stamp)| (RepoPath(path.into()), Digest(stamp.into())))
    .collect()
}

fn paths(paths: &[&str]) -> Vec<RepoPath> {
    paths.iter().map(|path| RepoPath((*path).into())).collect()
}

/// A repository's listing: two products, `crates/p` and `crates/q`, and a file at the top.
fn listing() -> Vec<RepoPath> {
    let mut listed = paths(&[
        "README.md",
        "crates/p/Cargo.toml",
        "crates/p/src-old/c.rs",
        "crates/p/src/a.rs",
        "crates/p/src/b.rs",
        "crates/q/Cargo.toml",
        "crates/q/lib.rs",
    ]);
    listed.sort();
    listed
}

/// What `spec`, written from `crates/p`, names in the listing, joined by spaces.
fn names(spec: &str) -> String {
    beneath(
        &listing(),
        &RepoPath("crates/p".into()),
        &Pathspec(spec.into()),
    )
    .iter()
    .map(|path| path.0.to_string())
    .collect::<Vec<_>>()
    .join(" ")
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every part's runner takes a subject that answers a Result, as `run` states it; these rows only succeed"
)]
fn subject(question: Input) -> Result<Value, Failure> {
    let before = stamps("a1");
    let text = match question {
        Question::Digest => named(&paths(&["src/b.rs", "src/a.rs"]), &before, &readable)
            .0
            .to_string(),
        Question::Edited => {
            let after = stamps("a2");
            let word = |listed: &[&str]| {
                if named(&paths(listed), &before, &readable)
                    == named(&paths(listed), &after, &readable)
                {
                    "kept"
                } else {
                    "moved"
                }
            };
            format!(
                "docs {}, src {}",
                word(&["docs/notes.md"]),
                word(&["src/a.rs", "src/b.rs"])
            )
        }
        Question::Root => names("."),
        Question::Directory => names("src/"),
        Question::File => names("Cargo.toml"),
        Question::Outside => names("../q"),
        Question::Wildcard => names("../*/Cargo.toml"),
    };
    Ok(std::borrow::Cow::Owned(text))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
