//! A verb names one operation, a bench directory places its output under the target, and each operation runs its
//! worker command; an unknown verb and a path that is not a bench directory are refused.
use super::service::{layout, operation, verb_of, worker_arguments};
use super::types::{BenchDirectory, OperationVerb};
use crate::cases::{Case, CaseName, Expect, Word, run};
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
        name: CaseName("measure places its output under the target and certifies against its registration"),
        word: Word::Succeeds,
        input: || ("measure-valid", "languages/python/benchmarks/linters/ruff"),
        expect: Expect::Value(Cow::Borrowed(
            "/target/benchmark/python/ruff/measure-valid: measure-valid /bin/tool /target/benchmark/python/ruff/measure-valid/state /target/benchmark/python/ruff/measure-valid/collector --case-inventory=/target/benchmark/python/ruff/measure-valid/registration.json",
        )),
    },
    Case {
        name: CaseName("an absolute bench directory places parity by its tail"),
        word: Word::Succeeds,
        input: || ("parity", "/repository/crates/ryo/languages/typescript/benchmarks/linters/oxlint"),
        expect: Expect::Value(Cow::Borrowed(
            "/target/benchmark/typescript/oxlint/parity: parity /bin/tool /target/benchmark/typescript/oxlint/parity",
        )),
    },
    Case {
        name: CaseName("correctness runs no worker"),
        word: Word::Succeeds,
        input: || ("verify-inventory", "languages/python/benchmarks/linters/ruff"),
        expect: Expect::Value(Cow::Borrowed("/target/benchmark/python/ruff/verify-inventory: ")),
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
        input: || ("parity", "crates/x"),
        expect: Expect::Error(Cow::Borrowed(
            "expected a bench directory languages/<lang>/benchmarks/linters/<linter>",
        )),
    },
    Case {
        name: CaseName("a language directory is not a bench directory"),
        word: Word::Refuses,
        input: || ("parity", "languages/python"),
        expect: Expect::Error(Cow::Borrowed(
            "expected a bench directory languages/<lang>/benchmarks/linters/<linter>",
        )),
    },
];

fn subject((verb, bench): Input) -> Result<Value, Failure> {
    let chosen = operation(OperationVerb(verb)).ok_or(Cow::Borrowed(verb))?;
    assert_eq!(verb_of(chosen), OperationVerb(verb), "the verb round-trips");
    let placed = layout(Path::new("/target"), &BenchDirectory(PathBuf::from(bench)), chosen)
        .map_err(|error| Cow::Owned(error.to_string()))?;
    let arguments = worker_arguments(chosen, &placed, Path::new("/bin/tool"))
        .iter()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    Ok(Cow::Owned(format!("{}: {}", placed.root.display(), arguments.join(" "))))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
