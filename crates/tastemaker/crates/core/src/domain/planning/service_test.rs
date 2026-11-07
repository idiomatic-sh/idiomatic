//! The `planning` service's cases, over the `check` fixture's workspace: the plan of its lint calls.

use super::service::plan;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::applications::fixtures::{matching, placement, workspace, workspace_of};
use crate::domain::applications::types::{PlacingSpec, Plan};
use crate::domain::findings::service::printed;
use crate::domain::paths::types::RepoPath;
use crate::inbound::use_cases::check::fixtures::SPEC;

/// The fixture's tree.
const FILES: &[&str] = &[
    "tastemaker.toml",
    "src/a/errors.txt",
    "src/a/stray.txt",
    "src/b/lib.txt",
];

/// Which question is asked of the plan.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// The rules of the lint call, by id, and what each reads.
    Rules,
    /// The placed rows the host declares.
    Placed,
    /// The placed row of a template entry of the `tastemaker.toml` itself.
    PlacedByTheWorkspace,
    /// A rule a spec applies whose include reaches no file of an instance: a rule that checks for an absence.
    SpecRuleReachingNothing,
    /// A rule entry of the `tastemaker.toml` whose include reaches no file.
    WorkspaceRuleReachingNothing,
}

/// The fixture's workspace with a template entry of its own and no spec.
const WORKSPACE_TEMPLATE: &str = r#"workspace = { name = "w", dependencies = [{ package = "p", path = "p" }] }
closed = false
templates = [{ template = "p/errors", file = "notes.txt" }]
"#;

/// The fixture's workspace with no rule entry of its own: its rules come from the spec.
const WORKSPACE_SPEC_ONLY: &str = r#"workspace = { name = "w", dependencies = [{ package = "p", path = "p" }] }
closed = false
specs = [{ spec = "p/part", include = ["src/{domain}"] }]
"#;

/// The fixture's workspace with one rule entry whose include matches nothing, and no spec.
const WORKSPACE_RULE_UNREACHED: &str = r#"workspace = { name = "w", dependencies = [{ package = "p", path = "p" }] }
closed = false
rules = [{ rule = "p/marker", include = ["nothing/**"] }]
"#;

/// The fixture's spec, applying the marker rule to a file an instance does not have, as `cli/no-root-composition` does.
fn spec_with_absence_rule() -> String {
    SPEC.replacen(
        "templates = [",
        "rules = [\n  { rule = \"p/marker@0.1.0\", include = [\"composition.txt\"], description = \"No composition file.\" },\n]\ntemplates = [",
        1,
    )
}

/// Whether a plan evaluated the marker rule, and which `#coverage` findings it holds, in words.
fn reach(plan: &Plan) -> String {
    format!(
        "evaluated: {} | coverage: {}",
        plan.evaluated
            .iter()
            .map(|rule| printed(rule).0.to_string())
            .filter(|rule| rule == "p/marker")
            .collect::<Vec<_>>()
            .join(","),
        plan.findings
            .iter()
            .map(|finding| printed(&finding.rule).0.to_string())
            .filter(|rule| rule.ends_with("#coverage"))
            .collect::<Vec<_>>()
            .join(",")
    )
}

type Input = Question;
/// What the row said, empty when it said what was expected.
type Value = String;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "the_placed_rule_by_its_definitions_own_id_and_the_comments_rule_are_the_lint_calls_rules",
        ),
        word: Word::Succeeds,
        input: || Question::Rules,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("each_placed_file_is_a_row_the_host_declares"),
        word: Word::Succeeds,
        input: || Question::Placed,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_file_the_workspace_s_own_template_entry_places_is_a_row_of_no_spec"),
        word: Word::Succeeds,
        input: || Question::PlacedByTheWorkspace,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName(
            "a_spec_s_rule_that_reaches_no_file_of_its_instance_is_evaluated_and_no_coverage_error",
        ),
        word: Word::Succeeds,
        input: || Question::SpecRuleReachingNothing,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName(
            "a_workspace_rule_entry_that_reaches_no_file_is_a_coverage_error_and_not_evaluated",
        ),
        word: Word::Succeeds,
        input: || Question::WorkspaceRuleReachingNothing,
        expect: Expect::Value(String::new()),
    },
];

fn subject(question: Input) -> Value {
    let workspace = match question {
        Question::Rules | Question::Placed => workspace(FILES),
        Question::PlacedByTheWorkspace => {
            workspace_of(WORKSPACE_TEMPLATE, SPEC, &["tastemaker.toml", "notes.txt"])
        }
        Question::SpecRuleReachingNothing => workspace_of(
            WORKSPACE_SPEC_ONLY,
            &spec_with_absence_rule(),
            &["tastemaker.toml", "src/a/errors.txt"],
        ),
        Question::WorkspaceRuleReachingNothing => {
            workspace_of(WORKSPACE_RULE_UNREACHED, SPEC, &["tastemaker.toml"])
        }
    };
    let placement = placement(&workspace);
    let Ok(plan) = plan(&workspace, &placement, RepoPath::default(), matching()) else {
        return "the fixture plans".to_owned();
    };
    let (said, expected) = match question {
        Question::Rules => (
            plan.rules
                .iter()
                .map(|rule| {
                    format!(
                        "{} {}",
                        rule.id.0,
                        rule.include
                            .iter()
                            .map(|glob| &*glob.0)
                            .collect::<Vec<_>>()
                            .join(",")
                    )
                })
                .collect::<Vec<_>>()
                .join("; "),
            "p/marker **,**/**; tastemaker-comments tastemaker.toml,src/a/errors.txt,src/a/stray.txt,src/b/lib.txt"
                .to_owned(),
        ),
        Question::Placed | Question::PlacedByTheWorkspace => (
            plan.placed
                .iter()
                .map(|row| {
                    format!(
                        "{} {} {}",
                        row.path.0,
                        row.template.0,
                        match &row.spec {
                            PlacingSpec::Workspace => "(no spec)",
                            PlacingSpec::Instance(spec) => &spec.0,
                        }
                    )
                })
                .collect::<Vec<_>>()
                .join("; "),
            if matches!(question, Question::Placed) {
                "src/a/errors.txt errors part"
            } else {
                "notes.txt errors (no spec)"
            }
            .to_owned(),
        ),
        Question::SpecRuleReachingNothing => (
            reach(&plan),
            "evaluated: p/marker | coverage: ".to_owned(),
        ),
        Question::WorkspaceRuleReachingNothing => (
            reach(&plan),
            "evaluated:  | coverage: p/marker#coverage".to_owned(),
        ),
    };
    if said == expected {
        String::new()
    } else {
        said
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
