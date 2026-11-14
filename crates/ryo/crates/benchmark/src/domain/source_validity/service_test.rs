//! A source the reference reports under its invalid-syntax rule leaves the run and is answered; the others stay.
use super::service::exclude_refused;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo::linter::{RelativePath, RuleId};
use ryo_benchmark_contracts::domain::reports::types::{
    BenchmarkFinding, CoverageAccounting, ExecutionMetadata, PairCount, ProjectContext,
    RuleCoverage, RunReport, SourceByteOffset,
};
use std::borrow::Cow;

/// The files the reference reports under its invalid-syntax rule.
type Input = &'static [&'static str];
/// The files the run keeps, and the refused files answered, each joined by `,`.
type Value = (Cow<'static, str>, Cow<'static, str>);
type Failure = Cow<'static, str>;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a source the reference refuses to parse leaves the run and is answered"),
        word: Word::Succeeds,
        input: || &["bad.py"],
        expect: Expect::Value((Cow::Borrowed("good.py"), Cow::Borrowed("bad.py"))),
    },
    Case {
        name: CaseName("with no refusal every source stays"),
        word: Word::Succeeds,
        input: || &[],
        expect: Expect::Value((Cow::Borrowed("bad.py,good.py"), Cow::Borrowed(""))),
    },
];

/// A finding of `rule` at the start of `file`.
fn finding(rule: &str, file: &str) -> Result<BenchmarkFinding, Failure> {
    Ok(BenchmarkFinding {
        rule: RuleId::try_from(rule).map_err(|error| Cow::Owned(error.to_string()))?,
        file: RelativePath(file.into()),
        start: SourceByteOffset(0),
        end: SourceByteOffset(1),
    })
}

fn subject(refused: Input) -> Result<Value, Failure> {
    let mut diagnostics = vec![finding("A", "good.py")?];
    for file in refused {
        diagnostics.push(finding("invalid-syntax", file)?);
    }
    let mut files = vec![
        RelativePath("bad.py".into()),
        RelativePath("good.py".into()),
    ];
    let report = RunReport {
        execution_metadata: ExecutionMetadata::Complete,
        project_context: ProjectContext::NotRequested,
        files: files.clone(),
        skipped: Vec::new(),
        coverage: CoverageAccounting::Accounted(RuleCoverage {
            selected: PairCount(2),
            evaluated: PairCount(2),
            skipped: PairCount(0),
            unknown_or_unread: PairCount(0),
        }),
        diagnostics,
    };
    let syntax =
        RuleId::try_from("invalid-syntax").map_err(|error| Cow::Owned(error.to_string()))?;
    let answered = exclude_refused(&report, &syntax, &mut files);
    let join = |names: Vec<&str>| Cow::Owned(names.join(","));
    Ok((
        join(files.iter().map(|file| file.0.as_ref()).collect()),
        join(
            answered
                .iter()
                .map(|finding| finding.file.0.as_ref())
                .collect(),
        ),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
