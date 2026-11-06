//! The `roles` service's cases for the pinned files: for every file the fixtures pin, the role of every element is the field of
//! `go/ast` that holds it, as `tests/fixtures/standard/dump.go` reads it with `reflect` and `output.txt` pins it.

use super::fixtures::{Given, table};
use super::service::{field_name, field_of};
use super::types::AstField;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::syntax::service::{kind, kind_name};
use crate::domain::syntax::types::Kind;
use crate::outbound::syntax::fixtures::library::{pinned, roles};
use ryo::languages::{self, Tree};

/// What a row is given.
type Input = Given;
/// The pinned files whose roles differ from `go/ast`'s, joined with ` | `.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
pub(super) const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_pinned_file_gives_each_element_the_field_of_go_ast_that_holds_it"),
        word: Word::Succeeds,
        input: || Given::Pinned,
        expect: Expect::Value(""),
    },
];

/// The roles our tree of `source` gives, as the dump writes `go/ast`'s: `start end N|T kind field`, an edge with no role left out.
fn ours(source: &[u8]) -> Result<Vec<String>, Failure> {
    let (elements, _) = table(source)?;
    let tree = Tree {
        elements: &elements,
        source: languages::ByteSlice(source),
        file: languages::FileId(0),
    };
    let mut lines: Vec<String> = languages::positions(&elements)
        .filter(|at| field_of(&tree, *at) != AstField::None)
        .map(|at| {
            format!(
                "{} {} {} {} {}",
                languages::start_at(&elements, at).0,
                languages::end_at(&elements, at).0,
                if matches!(kind(&tree, at), Kind::Node(_)) {
                    "N"
                } else {
                    "T"
                },
                kind_name(kind(&tree, at)).0,
                field_name(field_of(&tree, at)).0,
            )
        })
        .collect();
    lines.sort();
    Ok(lines)
}

/// The pinned files whose roles differ from `go/ast`'s, by path below `tests/fixtures`, each with its first difference.
fn differing() -> Result<Vec<String>, Failure> {
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let files = pinned(&fixtures);
    if files.is_empty() {
        return Err(());
    }
    let mut found = Vec::new();
    for path in files {
        let bytes = std::fs::read(&path).map_err(drop)?;
        let Some(mut theirs) = roles(&String::from_utf8_lossy(&bytes)) else {
            continue;
        };
        theirs.sort_unstable();
        let ours = ours(&bytes)?;
        let missing = theirs
            .iter()
            .find(|line| !ours.iter().any(|each| each == *line));
        let added = ours.iter().find(|line| !theirs.contains(&line.as_str()));
        if missing.is_some() || added.is_some() {
            found.push(format!(
                "{}: go/ast {:?}, ours {:?}",
                path.strip_prefix(&fixtures).unwrap_or(&path).display(),
                missing,
                added
            ));
        }
    }
    Ok(found)
}

/// The pinned files whose roles differ from `go/ast`'s, joined.
fn subject(_given: Input) -> Result<Value, Failure> {
    Ok(Box::leak(differing()?.join(" | ").into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
