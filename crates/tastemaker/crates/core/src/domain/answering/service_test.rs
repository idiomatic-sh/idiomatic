//! The `answering` service's cases, on values alone: what the lint call's answer comes to.

use super::service::{answered, judge};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::applications::types::{AnswerCell, CellText, CellValue, FoundAt, Rendering};
use crate::domain::comments::types::ByteSpan;
use crate::domain::definitions::types::{ColumnName, Diagnostic};
use crate::domain::findings::types::ByteOffset;
use crate::domain::findings::types::MessageTemplate;
use crate::domain::findings::types::{MessageText, RuleId};
use crate::domain::paths::types::RepoPath;

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
enum Question {
    Answered,
    Columns,
    /// An import no crate root reaches, of a source file and of a template's rendered content; a package outside the
    /// workspace not analyzed; the run's manifests read in part.
    Unreached,
    /// A primary column whose cell is text, not a span.
    NotASpan,
}

type Input = Question;
/// What the row said, empty when it said what was expected.
type Value = String;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_finding_is_rendered_and_placed_at_its_span"),
        word: Word::Succeeds,
        input: || Question::Answered,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_message_reads_every_column_of_the_row_whole"),
        word: Word::Succeeds,
        input: || Question::Columns,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("an_unreached_rendered_content_is_no_unknown_import"),
        word: Word::Succeeds,
        input: || Question::Unreached,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_primary_cell_that_is_no_span_places_the_finding_at_its_whole_file"),
        word: Word::Succeeds,
        input: || Question::NotASpan,
        expect: Expect::Value(String::new()),
    },
];

fn path(text: &str) -> RepoPath {
    RepoPath(text.into())
}

/// A cell of a row.
fn cell(name: &str, value: CellValue) -> AnswerCell {
    AnswerCell {
        name: ColumnName(name.into()),
        value,
    }
}

/// A rule whose findings say `message`, placed at the column `primary`.
fn rendering(primary: &str, message: &str) -> Rendering {
    Rendering {
        rule: RuleId("p/r".into()),
        diagnostic: Diagnostic {
            primary: ColumnName(primary.into()),
            message: MessageTemplate(message.into()),
            help: MessageTemplate("remove it".into()),
        },
    }
}

/// What the Answered question says, and what it should say: the message reads each cell by its column, and the finding is
/// at the span its primary cell holds.
fn answered_said() -> (String, String) {
    let answer = answered(
        &rendering("span", "`{file}` holds {text}"),
        path("a.txt"),
        &[
            cell("file", CellValue::File(path("a.txt"))),
            cell(
                "span",
                CellValue::Span(ByteSpan {
                    start: ByteOffset(3),
                    end: ByteOffset(9),
                }),
            ),
            cell("text", CellValue::Text(CellText("a marker, here".into()))),
        ],
    );
    let place =
        matches!(answer.located, FoundAt::Span(span) if span.start.0 == 3 && span.end.0 == 9);
    (
        format!("{} {place}", answer.finding.message.0),
        "`a.txt` holds a marker, here true".to_owned(),
    )
}

/// What the Columns question says, and what it should say: a cell's text is read whole, commas and all.
fn columns_said() -> (String, String) {
    let answer = answered(
        &rendering("file", "`{file}` misses {feature}"),
        path("w/inner/a.feature"),
        &[
            cell("file", CellValue::File(path("w/inner/a.feature"))),
            cell(
                "feature",
                CellValue::Text(CellText("a step, with a comma".into())),
            ),
        ],
    );
    (
        answer.finding.message.0.to_string(),
        "`w/inner/a.feature` misses a step, with a comma".to_owned(),
    )
}

/// What the `NotASpan` question says, and what it should say.
fn not_a_span_said() -> (String, String) {
    let answer = answered(
        &rendering("text", "{text}"),
        path("a.txt"),
        &[cell("text", CellValue::Text(CellText("3..9".into())))],
    );
    (
        format!("{}", matches!(answer.located, FoundAt::Whole)),
        "true".to_owned(),
    )
}

/// What the Unreached question says, and what it should say.
fn unreached_said() -> (String, String) {
    use crate::domain::applications::types::{LintRuleId, LintUnknown, Plan, Purpose};
    use crate::domain::imports::types::Relation;
    let id = LintRuleId("imports-edges".into());
    let mut plan = Plan::default();
    drop(
        plan.purposes
            .insert(id.clone(), Purpose::Imports(Relation::Edges)),
    );
    plan.files = vec![
        RepoPath("tastemaker.toml".into()),
        RepoPath("src/a.txt".into()),
    ];
    plan.anchors = vec![RepoPath("tastemaker.toml".into())];
    let unknown = |path: &str, message: &str| LintUnknown {
        rule: id.clone(),
        path: RepoPath(path.into()),
        message: MessageText(message.into()),
    };
    let judged = judge(
        &plan,
        Vec::new(),
        vec![
            unknown(
                "src/a.txt",
                "source-dependency is read only in part in src/a.txt: the file does not parse",
            ),
            unknown(
                "",
                "source-dependency is read only in part in .: .tastemaker-rendered/3.lib.txt does not parse",
            ),
            unknown(
                "../other/Cargo.toml",
                "package-dependency is read only in part in ../other/Cargo.toml: Cargo.toml: dependency package ../other/Cargo.toml was not analyzed",
            ),
            unknown(
                "",
                "package-dependency is read only in part in .: Cargo refused crates/b/Cargo.toml",
            ),
        ],
    );
    let said = judged
        .findings
        .iter()
        .map(|finding| {
            format!(
                "{} {}",
                crate::domain::findings::service::printed(&finding.rule).0,
                finding.path.0
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    // The source file's is its own; the package outside the workspace's is none; the run's is the workspace file's.
    (
        said,
        "tastemaker.toml#unknown src/a.txt; tastemaker.toml#unknown tastemaker.toml".to_owned(),
    )
}

fn subject(question: Input) -> Value {
    let (said, expected) = match question {
        Question::Answered => answered_said(),
        Question::Columns => columns_said(),
        Question::Unreached => unreached_said(),
        Question::NotASpan => not_a_span_said(),
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
