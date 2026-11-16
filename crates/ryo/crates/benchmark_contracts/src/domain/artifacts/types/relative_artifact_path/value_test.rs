//! The path retains its borrowed identity and rejects escaping component forms.
use super::value::RelativeArtifactPath;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::artifacts::errors::ArtifactsError;
use std::path::Path;
type Input = &'static str;
type Value = ();
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("nested source retains borrowed address"),
        word: Word::Succeeds,
        input: || "pkg/λ.py",
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("empty address is refused"),
        word: Word::Refuses,
        input: || "",
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("absolute address is refused"),
        word: Word::Refuses,
        input: || "/pkg/file.py",
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("parent traversal is refused"),
        word: Word::Refuses,
        input: || "pkg/../file.py",
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("current directory alone is refused"),
        word: Word::Refuses,
        input: || ".",
        expect: Expect::Error(()),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    let path = Path::new(input);
    let relative = RelativeArtifactPath::try_from(path).map_err(|error| match error {
        ArtifactsError::Invalid(_) => (),
    })?;
    assert!(
        std::ptr::eq(relative.as_ref(), path),
        "validation borrows the caller's address"
    );
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

/// The address spelled with `/`, from a path joined component by component as the platform joins them.
#[test]
fn slashed_joins_the_components_with_a_slash() {
    let path = Path::new("pkg").join("sub").join("λ.py");
    let address = RelativeArtifactPath::try_from(path.as_path()).expect("a relative address");
    assert_eq!(address.slashed().expect("UTF-8 names").0, "pkg/sub/λ.py");
}
