//! The path retains its borrowed identity and rejects escaping component forms.
use super::value::RelativeArtifactPath;
use ryo::cases::{Case, CaseName, Expect, Word, run};
use std::path::Path;
type Failure = ();
const CASES: &[Case<&str, (), Failure>] = &[
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
fn subject(input: &str) -> Result<(), Failure> {
    let path = Path::new(input);
    let relative = RelativeArtifactPath::try_from(path).map_err(|_| ())?;
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
