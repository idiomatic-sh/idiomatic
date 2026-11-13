//! A verb names one operation, a bench directory places its output, and each operation runs its worker command.
use super::service::{layout, operation, verb_of, worker_arguments};
use super::types::{BenchDirectory, Operation, OperationLayout};
use crate::cases::{Case, CaseName, Expect, Word, run};
use std::borrow::Cow;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

type Input = (&'static str, Operation);
type Value = Cow<'static, str>;
type Failure = &'static str;

/// The refusal of a path that is not a bench directory.
const NOT_A_BENCH: &str = "expected a bench directory languages/<lang>/benchmarks/linters/<linter>";

const LAYOUTS: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a bench directory places its operation under the target"),
        word: Word::Succeeds,
        input: || ("languages/python/benchmarks/linters/ruff", Operation::Measure),
        expect: Expect::Value(Cow::Borrowed("/target/benchmark/python/ruff/measure-valid")),
    },
    Case {
        name: CaseName("an absolute bench directory places its operation by its tail"),
        word: Word::Succeeds,
        input: || ("/repository/crates/ryo/languages/typescript/benchmarks/linters/oxlint", Operation::Parity),
        expect: Expect::Value(Cow::Borrowed("/target/benchmark/typescript/oxlint/parity")),
    },
    Case {
        name: CaseName("a crate directory is not a bench directory"),
        word: Word::Refuses,
        input: || ("crates/x", Operation::Correctness),
        expect: Expect::Error(NOT_A_BENCH),
    },
    Case {
        name: CaseName("a language directory is not a bench directory"),
        word: Word::Refuses,
        input: || ("languages/python", Operation::Correctness),
        expect: Expect::Error(NOT_A_BENCH),
    },
];

fn placed((bench, operation): Input) -> Result<Value, Failure> {
    layout(
        Path::new("/target"),
        &BenchDirectory(PathBuf::from(bench)),
        operation,
    )
    .map(|layout| Cow::Owned(layout.root.display().to_string()))
    .map_err(|error| if error.to_string() == NOT_A_BENCH { NOT_A_BENCH } else { "another refusal" })
}

#[test]
fn succeeds() {
    run(LAYOUTS, Word::Succeeds, placed);
}

#[test]
fn refuses() {
    run(LAYOUTS, Word::Refuses, placed);
}

#[test]
fn verbs_name_their_operations() {
    for each in [Operation::Correctness, Operation::Measure, Operation::Parity] {
        assert_eq!(operation(verb_of(each)), Some(each));
    }
    assert_eq!(operation("measure"), None);
}

#[test]
fn each_operation_runs_its_worker_command() {
    let layout = OperationLayout {
        root: PathBuf::from("/out"),
    };
    let tool = Path::new("/bin/ruff");
    let arguments = |operation| {
        worker_arguments(operation, &layout, tool)
            .into_iter()
            .map(OsString::into_string)
            .collect::<Result<Vec<_>, _>>()
            .expect("UTF-8 arguments")
    };
    assert_eq!(arguments(Operation::Correctness), Vec::<String>::new());
    assert_eq!(
        arguments(Operation::Measure),
        [
            "measure-valid",
            "/bin/ruff",
            "/out/state",
            "/out/collector",
            "--case-inventory=/out/registration.json"
        ]
    );
    assert_eq!(arguments(Operation::Parity), ["parity", "/bin/ruff", "/out"]);
}
