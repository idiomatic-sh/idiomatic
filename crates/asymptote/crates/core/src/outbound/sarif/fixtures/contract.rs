//! What every `sarif` adapter must pass: what it writes, it reads back as the same results, and no output is no log.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::judgement::types::{Inventory, Judgement, Run, Status};
use crate::domain::manifest::types::ProductRoot;
use crate::domain::violations::types::{
    ColumnNumber, Fingerprint, Fingerprinted, Justification, Level, LineNumber, MessageText, Place,
    RepoPath, RuleId, Suppression, ToolId, Violation,
};
use crate::outbound::sarif::errors::SarifError;
use crate::outbound::sarif::interface::SarifPort;
use crate::outbound::sarif::types::SarifText;

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
pub enum Question {
    /// A written log is read back.
    RoundTrip,
    /// No output is refused.
    Empty,
}

/// The results read back, `path:place:rule:standing:fingerprint`, joined.
type Input = Question;
/// The answer as text, borrowed when a row writes it in a constant.
type Value = std::borrow::Cow<'static, str>;
/// The SARIF port's error, which a refusal row expects.
type Failure = SarifError;

/// The contract's rows: every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_written_log_reads_back_as_its_results"),
        word: Word::Succeeds,
        input: || Question::RoundTrip,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "src/a.rs:3:2:r:Suppressed:ignored here:0000000000000abc",
        )),
    },
    Case {
        name: CaseName("no_output_is_no_log"),
        word: Word::Refuses,
        input: || Question::Empty,
        expect: Expect::Refusal(|actual| {
            if actual
                == &(SarifError::Unparsable {
                    reason: "the tool printed nothing".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
];

/// Runs the contract's rows against one implementation.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &impl SarifPort) {
    let subject = |question: Input| -> Result<String, Failure> {
        let text = match question {
            Question::RoundTrip => adapter.write(
                &ProductRoot("/repo/p".into()),
                &[Run {
                    tool: ToolId("t".into()),
                    status: Status::Completed,
                    inventory: Inventory::Undeclared,
                    reported: std::collections::BTreeSet::new(),
                }],
                &Judgement {
                    new: vec![Violation {
                        path: RepoPath("src/a.rs".into()),
                        place: Place::At(LineNumber(3), ColumnNumber(2)),
                        tool: ToolId("t".into()),
                        rule: RuleId("r".into()),
                        level: Level::Warning,
                        suppression: Suppression::Suppressed(Justification("ignored here".into())),
                        message: MessageText("m".into()),
                        fingerprint: Fingerprinted::Known(Fingerprint(0xabc)),
                    }],
                    ..Judgement::default()
                },
            ),
            Question::Empty => SarifText(Box::default()),
        };
        let read = adapter.read(&text)?;
        Ok(read
            .results
            .iter()
            .map(|result| {
                let place = match result.place {
                    Place::At(line, column) => format!("{}:{}", line.0, column.0),
                    Place::Line(line) => line.0.to_string(),
                    Place::File => String::new(),
                };
                let path = result
                    .path
                    .0
                    .strip_prefix("/repo/p/")
                    .unwrap_or(&result.path.0);
                let suppression = match &result.suppression {
                    Suppression::Suppressed(why) => format!("Suppressed:{}", why.0),
                    Suppression::Unsuppressed => "Reported".to_owned(),
                };
                let print = match result.supplied {
                    Fingerprinted::Known(print) => format!("{:016x}", print.0),
                    Fingerprinted::Unknown => "unknown".to_owned(),
                };
                format!("{path}:{place}:{}:{suppression}:{print}", result.rule.0)
            })
            .collect::<Vec<_>>()
            .join(" "))
    };
    run(CASES, Word::Succeeds, |input| {
        subject(input).map(std::borrow::Cow::Owned)
    });
    run(CASES, Word::Refuses, |input| {
        subject(input).map(std::borrow::Cow::Owned)
    });
}
