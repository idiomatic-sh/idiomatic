//! Of a build's compiler-artifact messages, exactly one bench executable of the bench's package is sealed.
use super::bench_executable;
use crate::cases::{Case, CaseName, Expect, Word, run};
use std::borrow::Cow;
use std::path::Path;

/// The bench package's manifest the messages are matched against.
const MANIFEST: &str = "/repository/languages/python/benchmarks/linters/ruff/Cargo.toml";

/// One compiler-artifact message: its manifest, its target kind, whether it is a test build, and its executable.
fn artifact(manifest: &str, kind: &str, test: bool, executable: &str) -> String {
    serde_json::json!({
        "reason": "compiler-artifact",
        "package_id": "path+file:///repository#fixture@0.1.0",
        "manifest_path": manifest,
        "target": {"name": "fixture", "kind": [kind], "crate_types": [kind], "src_path": "/repository/bench.rs", "edition": "2024"},
        "profile": {"opt_level": "3", "debuginfo": 0, "debug_assertions": false, "overflow_checks": false, "test": test},
        "features": [],
        "filenames": [executable],
        "executable": executable,
        "fresh": false
    })
    .to_string()
}

type Input = Vec<String>;
type Value = Cow<'static, str>;
type Failure = &'static str;

/// The refusal of a build without exactly one bench executable.
const ONE_BENCH: &str = "expected exactly one bench executable of the bench's package";

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the one bench test build of the package is chosen among its library and other packages"),
        word: Word::Succeeds,
        input: || {
            vec![
                artifact(MANIFEST, "lib", false, "/target/release/libfixture.rlib"),
                artifact("/repository/crates/other/Cargo.toml", "bench", true, "/target/release/deps/other-1"),
                artifact(MANIFEST, "bench", true, "/target/release/deps/ruff-1"),
                serde_json::json!({"reason": "build-finished", "success": true}).to_string(),
            ]
        },
        expect: Expect::Value(Cow::Borrowed("/target/release/deps/ruff-1")),
    },
    Case {
        name: CaseName("a build without a bench executable of the package is refused"),
        word: Word::Refuses,
        input: || vec![artifact(MANIFEST, "lib", false, "/target/release/libfixture.rlib")],
        expect: Expect::Error(ONE_BENCH),
    },
    Case {
        name: CaseName("two bench executables are refused, not the first taken"),
        word: Word::Refuses,
        input: || {
            vec![
                artifact(MANIFEST, "bench", true, "/target/release/deps/ruff-1"),
                artifact(MANIFEST, "bench", true, "/target/release/deps/ruff-2"),
            ]
        },
        expect: Expect::Error(ONE_BENCH),
    },
    Case {
        name: CaseName("a bench target not built as a test harness is refused"),
        word: Word::Refuses,
        input: || vec![artifact(MANIFEST, "bench", false, "/target/release/deps/ruff-1")],
        expect: Expect::Error(ONE_BENCH),
    },
];

fn subject(messages: Input) -> Result<Value, Failure> {
    bench_executable(messages.join("\n").as_bytes(), Path::new(MANIFEST))
        .map(|path| Cow::Owned(path.display().to_string()))
        .map_err(|error| if error.to_string() == ONE_BENCH { ONE_BENCH } else { "another refusal" })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
