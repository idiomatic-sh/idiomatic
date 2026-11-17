//! What the `scenario_judging` service hosts for a scenario's verdict, and the problems it says of the rows the verdict
//! queries answer, on values alone. The queries themselves run through ryo in the ryo adapter's cases.

use super::service::{host, problems};
use super::types::{
    FindingCount, OnlyPlace, OnlyRow, RuleRow, ScenarioCounts, ScenarioNumber, ScenarioVerdict,
    VerdictRows, VerdictRule,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::findings::service::{check_id, error, of_rule};
use crate::domain::findings::types::{Line, Place, RuleId};
use crate::domain::paths::types::RepoPath;
use crate::domain::proofs::types::FoundOwn;
use crate::domain::scenarios::types::{
    ExpectedFinding, ExpectedLine, ExpectedRule, ExpectedRuleChoice, ScenarioId,
};

/// What a row of the cases asks.
#[derive(Clone, Copy)]
enum Question {
    /// The rows a rule's scenario hosts: its own rule blanked on both sides, another rule named as the expectation it
    /// ends with, a finding at the root at `.`, `#outdated` left out.
    Hosted,
    /// The problems of what the queries answered: what never ran first, `only`'s in its order, then each missing finding
    /// as often as it is short, then each extra one, each list in the order of its text.
    Said,
}

type Input = Question;
/// What the row gave, when it is not what it should; empty when it is.
type Value = String;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_scenario_hosts_its_findings_named_as_the_verdict_compares_them"),
        word: Word::Succeeds,
        input: || Question::Hosted,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_problems_say_what_never_ran_then_what_is_missing_then_what_is_extra"),
        word: Word::Succeeds,
        input: || Question::Said,
        expect: Expect::Value(String::new()),
    },
];

/// The rows a rule's scenario hosts, as text, and what they should be.
fn hosted() -> (String, String) {
    let expected = [
        ExpectedFinding {
            rule: ExpectedRuleChoice::Own,
            path: RepoPath("a.txt".into()),
            line: ExpectedLine::At(Line(3)),
        },
        ExpectedFinding {
            rule: ExpectedRuleChoice::Named(ExpectedRule("#count".into())),
            path: RepoPath(".".into()),
            line: ExpectedLine::Whole,
        },
    ];
    let reported = [Place::Line(Line(3)), Place::Whole, Place::Whole]
        .into_iter()
        .zip([
            of_rule(RuleId("p/r".into())),
            check_id("p/errors", "count"),
            check_id("p/errors", "outdated"),
        ])
        .map(|(place, rule)| crate::domain::findings::types::Finding {
            place,
            ..error(rule, RepoPath(String::new().into()), "wrong")
        })
        .collect::<Vec<_>>();
    let mut rows = VerdictRows::default();
    host(
        ScenarioNumber(2),
        Some(FoundOwn("p/r")),
        &expected,
        &[ExpectedRule("p/r".into())],
        &reported,
        &[of_rule(RuleId("p/r".into()))].into_iter().collect(),
        &mut rows,
    );
    let said = format!(
        "{} | {} | {} {} {}",
        rows.expected
            .iter()
            .map(|row| format!(
                "{}:{}:{}:{}:{}",
                row.scenario.0, row.finding.0, row.rule.0, row.path.0, row.line.0
            ))
            .collect::<Vec<_>>()
            .join(","),
        rows.found
            .iter()
            .map(|row| format!(
                "{}:{}:{}:{}:{}:{}",
                row.scenario.0, row.finding.0, row.rule.0, row.full.0, row.path.0, row.line.0
            ))
            .collect::<Vec<_>>()
            .join(","),
        rows.only.len(),
        rows.own.len(),
        rows.evaluated.len()
    );
    (
        said,
        "2:0::a.txt:3,2:1:#count:.:0 | 2:2::p/r:.:3,2:3:#count:p/errors#count:.:0 | 1 1 1"
            .to_owned(),
    )
}

/// The problems of a set of answered rows, joined, and what they should be.
fn said() -> (String, String) {
    let counted = |rule: &str, line: u32, expected: u32, found: u32| ScenarioCounts {
        scenario: ScenarioNumber(0),
        rule: VerdictRule(rule.into()),
        path: RepoPath("a.txt".into()),
        line: Line(line),
        expected: FindingCount(expected),
        found: FindingCount(found),
    };
    let verdicts = [
        ScenarioVerdict::Extra(counted("p/z", 0, 0, 1)),
        ScenarioVerdict::Missing(counted("", 3, 2, 0)),
        ScenarioVerdict::OnlyUnran(OnlyRow {
            scenario: ScenarioNumber(0),
            place: OnlyPlace(1),
            only: ExpectedRule("second".into()),
        }),
        ScenarioVerdict::OnlyUnran(OnlyRow {
            scenario: ScenarioNumber(0),
            place: OnlyPlace(0),
            only: ExpectedRule("first".into()),
        }),
        ScenarioVerdict::Missing(counted("p/a", 2, 1, 0)),
        ScenarioVerdict::OwnUnran(RuleRow {
            scenario: ScenarioNumber(0),
            rule: VerdictRule("p/r".into()),
        }),
    ];
    let said = problems(
        &ScenarioId("c".into()),
        &verdicts.iter().collect::<Vec<_>>(),
    )
    .iter()
    .map(|problem| problem.0.to_string())
    .collect::<Vec<_>>()
    .join("|");
    (
        said,
        [
            "scenario `c`: `p/r` reached no file of the project, so the scenario proves nothing",
            "scenario `c`: `only` names `first`, which the check did not evaluate",
            "scenario `c`: `only` names `second`, which the check did not evaluate",
            "scenario `c`: expected a.txt:3, not found",
            "scenario `c`: expected a.txt:3, not found",
            "scenario `c`: expected p/a a.txt:2, not found",
            "scenario `c`: found p/z a.txt, not expected",
        ]
        .join("|"),
    )
}

fn subject(question: Input) -> Value {
    let (said, wanted) = match question {
        Question::Hosted => hosted(),
        Question::Said => said(),
    };
    if said == wanted { String::new() } else { said }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |question| Ok(subject(question)));
}
