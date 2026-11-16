//! A verb names one operation, a bench directory places its output under the target, and each operation runs its
//! worker command; an unknown verb and a path that is not a bench directory are refused.
use super::service::{layout, operation, verb_of, worker_arguments};
use super::types::{BenchDirectory, OperationVerb};
use crate::cases::{Case, CaseName, Expect, Word, run, slashed};
use std::borrow::Cow;
use std::path::{Path, PathBuf};

/// A collector verb and the bench directory it is given.
type Input = (&'static str, &'static str);
/// The operation's output directory and its worker command, one line.
type Value = Cow<'static, str>;
/// The refusal.
type Failure = Cow<'static, str>;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "bench places its output under the target, certifies against its registration and reads the verification's times",
        ),
        word: Word::Succeeds,
        input: || ("bench", "languages/python/benchmarks/linters/ruff"),
        expect: Expect::Value(Cow::Borrowed(
            "/target/benchmark/python/ruff/bench: measure-valid /bin/tool /target/benchmark/python/ruff/bench/state /target/benchmark/python/ruff/bench/collector --case-inventory=/target/benchmark/python/ruff/bench/registration.json --verification=/target/benchmark/python/ruff/bench/inventory",
        )),
    },
    Case {
        name: CaseName("an absolute bench directory places the bench by its tail"),
        word: Word::Succeeds,
        input: || {
            (
                "bench",
                "/repository/crates/ryo/languages/typescript/benchmarks/linters/oxlint",
            )
        },
        expect: Expect::Value(Cow::Borrowed(
            "/target/benchmark/typescript/oxlint/bench: measure-valid /bin/tool /target/benchmark/typescript/oxlint/bench/state /target/benchmark/typescript/oxlint/bench/collector --case-inventory=/target/benchmark/typescript/oxlint/bench/registration.json --verification=/target/benchmark/typescript/oxlint/bench/inventory",
        )),
    },
    Case {
        name: CaseName("verify-inventory is no longer an operation"),
        word: Word::Refuses,
        input: || {
            (
                "verify-inventory",
                "languages/python/benchmarks/linters/ruff",
            )
        },
        expect: Expect::Error(Cow::Borrowed("verify-inventory")),
    },
    Case {
        name: CaseName("measure-valid is a worker verb, not a collector operation"),
        word: Word::Refuses,
        input: || ("measure-valid", "languages/python/benchmarks/linters/ruff"),
        expect: Expect::Error(Cow::Borrowed("measure-valid")),
    },
    Case {
        name: CaseName("parity is no longer an operation"),
        word: Word::Refuses,
        input: || ("parity", "languages/python/benchmarks/linters/ruff"),
        expect: Expect::Error(Cow::Borrowed("parity")),
    },
    Case {
        name: CaseName("verify-cases is a worker verb, not a collector operation"),
        word: Word::Refuses,
        input: || ("verify-cases", "languages/python/benchmarks/linters/ruff"),
        expect: Expect::Error(Cow::Borrowed("verify-cases")),
    },
    Case {
        name: CaseName("verify-reference-cases is a worker verb, not a collector operation"),
        word: Word::Refuses,
        input: || {
            (
                "verify-reference-cases",
                "languages/typescript/benchmarks/linters/oxlint",
            )
        },
        expect: Expect::Error(Cow::Borrowed("verify-reference-cases")),
    },
    Case {
        name: CaseName("a verb that names no operation is refused"),
        word: Word::Refuses,
        input: || ("measure", "languages/python/benchmarks/linters/ruff"),
        expect: Expect::Error(Cow::Borrowed("measure")),
    },
    Case {
        name: CaseName("a crate directory is not a bench directory"),
        word: Word::Refuses,
        input: || ("bench", "crates/x"),
        expect: Expect::Error(Cow::Borrowed(
            "expected a bench directory languages/<lang>/benchmarks/linters/<linter>",
        )),
    },
    Case {
        name: CaseName("a language directory is not a bench directory"),
        word: Word::Refuses,
        input: || ("bench", "languages/python"),
        expect: Expect::Error(Cow::Borrowed(
            "expected a bench directory languages/<lang>/benchmarks/linters/<linter>",
        )),
    },
];

fn subject((verb, bench): Input) -> Result<Value, Failure> {
    let chosen = operation(OperationVerb(verb)).ok_or(Cow::Borrowed(verb))?;
    assert_eq!(verb_of(chosen), OperationVerb(verb), "the verb round-trips");
    let placed = layout(
        Path::new("/target"),
        &BenchDirectory(PathBuf::from(bench)),
        chosen,
    )
    .map_err(|error| Cow::Owned(error.to_string()))?;
    let arguments = worker_arguments(chosen, &placed, Path::new("/bin/tool"))
        .iter()
        .map(|argument| {
            let text = argument.to_string_lossy();
            match text.split_once('=') {
                Some((flag, path)) => format!("{flag}={}", slashed(Path::new(path)).0),
                None => slashed(Path::new(&*text)).0,
            }
        })
        .collect::<Vec<_>>();
    Ok(Cow::Owned(format!(
        "{}: {}",
        slashed(&placed.root).0,
        arguments.join(" ")
    )))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
