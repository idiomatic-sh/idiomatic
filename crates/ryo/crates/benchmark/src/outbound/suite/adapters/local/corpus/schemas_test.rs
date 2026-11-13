//! The accepted and refused corpus documents.
use crate::cases::{Case, CaseName, Expect, Word, run};
type Input = &'static str;
type Value = usize;
type Failure = &'static str;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("one complete project is a valid corpus"),
        word: Word::Succeeds,
        input: || "language='python'\n[[projects]]\nname='one'\nrepository='https://example.invalid/source.git'\nrevision='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'\npatch='patches/one.patch'\n",
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("two distinct project identities are valid"),
        word: Word::Succeeds,
        input: || "language='python'\n[[projects]]\nname='one'\nrepository='https://example.invalid/source.git'\nrevision='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'\npatch='patches/one.patch'\n[[projects]]\nname='two'\nrepository='https://example.invalid/source.git'\nrevision='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'\npatch='patches/one.patch'\n",
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a corpus without projects is refused"),
        word: Word::Refuses,
        input: || "language='python'\nprojects=[]\n",
        expect: Expect::Error("corpus identity missing"),
    },
    Case {
        name: CaseName("a blank language is refused"),
        word: Word::Refuses,
        input: || "language=' '\n[[projects]]\nname='one'\nrepository='https://example.invalid/source.git'\nrevision='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'\npatch='patches/one.patch'\n",
        expect: Expect::Error("corpus identity missing"),
    },
    Case {
        name: CaseName("two projects cannot share an identity"),
        word: Word::Refuses,
        input: || "language='python'\n[[projects]]\nname='one'\nrepository='https://example.invalid/source.git'\nrevision='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'\npatch='patches/one.patch'\n[[projects]]\nname='one'\nrepository='https://example.invalid/source.git'\nrevision='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'\npatch='patches/one.patch'\n",
        expect: Expect::Error("project names collapse"),
    },
    Case {
        name: CaseName("a blank project name is refused"),
        word: Word::Refuses,
        input: || "language='python'\n[[projects]]\nname=' '\nrepository='https://example.invalid/source.git'\nrevision='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'\npatch='patches/one.patch'\n",
        expect: Expect::Error("project names collapse"),
    },
    Case {
        name: CaseName("a project missing its required fields is a schema error"),
        word: Word::Refuses,
        input: || "language='python'\n[[projects]]\nname='one'\n",
        expect: Expect::Error("schema error"),
    },
];
/// Decodes one corpus document and counts its projects.
fn subject(text: Input) -> Result<Value, Failure> {
    super::suite(text)
        .map(|suite| suite.projects.len())
        .map_err(|error| match error.to_string().as_str() {
            "corpus must name a language and at least one project" => "corpus identity missing",
            "corpus project names must be nonempty and unique" => "project names collapse",
            _ => "schema error",
        })
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
