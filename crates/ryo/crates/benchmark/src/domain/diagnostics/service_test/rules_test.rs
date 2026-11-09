//! Per-rule comparison pairs findings by file and start, and positions count lines and characters.
use super::super::service::{compare_rules, position};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo::linter::{RelativePath, RuleId, SourceText};
use ryo_benchmark_contracts::domain::reports::types::{
    BenchmarkFinding, CoverageAccounting, ExecutionMetadata, ProjectContext, RunReport,
    SourceByteOffset,
};

/// A per-rule comparison of two reports, or the position of one offset in one source.
enum Check {
    Rules(RunReport, RunReport),
    Position(&'static str, usize),
}

/// Counts of each selected rule as (matched, end differences, missing, extra), or a line and column.
#[derive(Debug, PartialEq, Eq)]
enum Answer {
    Rules([(usize, usize, usize, usize); 2]),
    Position(usize, usize),
}

type Input = Check;
type Value = Answer;
type Failure = ();

fn finding(rule: &str, start: usize, end: usize) -> BenchmarkFinding {
    BenchmarkFinding {
        rule: RuleId::try_from(rule).expect("fixture rule"),
        file: RelativePath("a.py".into()),
        start: SourceByteOffset(start),
        end: SourceByteOffset(end),
    }
}

fn report(diagnostics: Vec<BenchmarkFinding>) -> RunReport {
    RunReport {
        execution_metadata: ExecutionMetadata::Complete,
        project_context: ProjectContext::NotRequested,
        diagnostics,
        skipped: Vec::new(),
        coverage: CoverageAccounting::Unaccounted,
        files: vec![RelativePath("a.py".into())],
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "findings pair by file and start; unpaired ones are missing or extra, an unselected rule is left out",
        ),
        word: Word::Succeeds,
        input: || {
            Check::Rules(
                report(vec![
                    finding("A", 0, 2),
                    finding("A", 4, 5),
                    finding("Z", 0, 1),
                ]),
                report(vec![
                    finding("A", 0, 3),
                    finding("A", 0, 2),
                    finding("A", 9, 9),
                ]),
            )
        },
        expect: Expect::Value(Answer::Rules([(1, 1, 1, 2), (0, 0, 0, 0)])),
    },
    Case {
        name: CaseName("a position counts \\r\\n as one break and columns in characters"),
        word: Word::Succeeds,
        input: || Check::Position("a\r\né=x\rb", 6),
        expect: Expect::Value(Answer::Position(2, 3)),
    },
    Case {
        name: CaseName("an offset inside a character has no position"),
        word: Word::Refuses,
        input: || Check::Position("é", 1),
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("an offset past the source has no position"),
        word: Word::Refuses,
        input: || Check::Position("ab", 3),
        expect: Expect::Error(()),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Check::Rules(reference, candidate) => {
            let selected = [
                RuleId::try_from("A").expect("fixture rule"),
                RuleId::try_from("B").expect("fixture rule"),
            ];
            let counts: Vec<_> = compare_rules(&selected, &reference, &candidate)
                .iter()
                .map(|outcome| {
                    (
                        outcome.matched.0,
                        outcome.end_differences.len(),
                        outcome.missing.len(),
                        outcome.extra.len(),
                    )
                })
                .collect();
            Ok(Answer::Rules(counts.try_into().map_err(|_count| ())?))
        }
        Check::Position(source, offset) => {
            position(&SourceText(source.into()), SourceByteOffset(offset))
                .map(|found| Answer::Position(found.line.0, found.column.0))
                .map_err(|_refused| ())
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
