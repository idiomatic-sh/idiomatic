//! The reference tool is the first of its name on the search path, a nested build is refused off Unix, and the adapter
//! keeps the port's contract.
use super::adapter::{ExecutablesCargoAdapter, find, refusal};
use super::settings::ExecutablesCargoSettings;
use super::types::BuildPlatform;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::workloads::types::tool_name::value::ToolName;
use std::borrow::Cow;

/// What a row asks of the adapter.
#[derive(Clone, Copy, Debug)]
enum Request {
    /// Find a tool two directories of the search path hold.
    Shadowed,
    /// Find a tool no directory holds.
    Absent,
    /// Build where the outer build keeps its lock.
    NestedOffUnix,
    /// Keep the port's contract.
    Contract,
}

type Input = Request;
type Value = Cow<'static, str>;
type Failure = Cow<'static, str>;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the first directory of the search path holding the tool wins"),
        word: Word::Succeeds,
        input: || Request::Shadowed,
        expect: Expect::Value(Cow::Borrowed("first/fixture-tool")),
    },
    Case {
        name: CaseName("the adapter keeps the port's contract"),
        word: Word::Succeeds,
        input: || Request::Contract,
        expect: Expect::Value(Cow::Borrowed("kept")),
    },
    Case {
        name: CaseName("a tool no directory holds is refused by name"),
        word: Word::Refuses,
        input: || Request::Absent,
        expect: Expect::Error(Cow::Borrowed(
            "reference tool not found on the search path: absent-tool",
        )),
    },
    Case {
        name: CaseName("a nested build is refused off Unix"),
        word: Word::Refuses,
        input: || Request::NestedOffUnix,
        expect: Expect::Error(Cow::Borrowed(
            "bench operations run on Unix only: on this platform the outer build holds the lock a nested build waits on",
        )),
    },
];

fn subject(request: Input) -> Result<Value, Failure> {
    let root = tempfile::tempdir().expect("search path fixture");
    let (first, second) = (root.path().join("first"), root.path().join("second"));
    for directory in [&first, &second] {
        std::fs::create_dir_all(directory).expect("search directory");
        std::fs::write(directory.join("fixture-tool"), "tool").expect("tool");
    }
    let search =
        std::env::join_paths([&root.path().join("empty"), &first, &second]).expect("search path");
    let canonical = root.path().canonicalize().expect("root");
    let refused = |error: ryo_benchmark_contracts::domain::reports::errors::ReportsError| {
        Cow::Owned(error.to_string())
    };
    match request {
        Request::Shadowed | Request::Absent => {
            let name = if matches!(request, Request::Shadowed) {
                "fixture-tool"
            } else {
                "absent-tool"
            };
            find(&search, &ToolName::from(name))
                .map(|path| {
                    Cow::Owned(
                        path.strip_prefix(&canonical)
                            .expect("inside the fixture")
                            .display()
                            .to_string(),
                    )
                })
                .map_err(refused)
        }
        Request::NestedOffUnix => refusal(BuildPlatform::Other)
            .map(|()| Cow::Borrowed("built"))
            .map_err(refused),
        Request::Contract => {
            crate::outbound::executables::fixtures::contract::check(&ExecutablesCargoAdapter::new(
                ExecutablesCargoSettings {
                    cargo: "cargo".into(),
                    search_path: std::ffi::OsString::new(),
                },
            ));
            Ok(Cow::Borrowed("kept"))
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
