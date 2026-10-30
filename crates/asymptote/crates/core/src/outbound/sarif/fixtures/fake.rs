//! The fake of the `sarif` port: the results each text stands for, given up front.

use std::collections::BTreeMap;

use crate::domain::judgement::types::Inventory;
use crate::domain::judgement::types::{Judgement, Run};
use crate::domain::manifest::types::ProductRoot;
use crate::domain::violations::types::{
    Fingerprinted, Justification, Level, MessageText, RepoPath, Suppression,
};
use crate::outbound::sarif::errors::SarifError;
use crate::outbound::sarif::interface::SarifPort;
use crate::outbound::sarif::types::{Read, Reported, SarifText};

/// The key the fake reads a text by: the text itself.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Said(pub Box<str>);

/// What the core's tests stand in for `sarif` with.
#[derive(Debug, Default)]
pub struct SarifFake {
    /// What each text reads as.
    logs: BTreeMap<Said, Read>,
}

impl SarifPort for SarifFake {
    fn read(&self, text: &SarifText) -> Result<Read, SarifError> {
        self.logs
            .get(&Said(text.0.clone()))
            .cloned()
            .ok_or_else(|| SarifError::Unparsable {
                reason: "the fake holds no such log".to_owned(),
            })
    }

    fn write(&self, _root: &ProductRoot, _runs: &[Run], outcome: &Judgement) -> SarifText {
        let lines = outcome
            .new
            .iter()
            .map(|found| format!("{}:new:{}", found.path.0, found.rule.0))
            .chain(
                outcome
                    .accepted
                    .iter()
                    .map(|found| format!("{}:unchanged:{}", found.path.0, found.rule.0)),
            )
            .chain(outcome.resolved.iter().map(|resolved| {
                format!(
                    "{}:absent:{}",
                    resolved.record.key.path.0, resolved.record.key.rule.0
                )
            }))
            .collect::<Vec<_>>();
        SarifText(lines.join("\n").into())
    }
}

/// A fake whose text `said` reads as `results`, with no inventory.
#[must_use]
pub fn reading(said: &str, results: Vec<Reported>) -> SarifFake {
    SarifFake {
        logs: BTreeMap::from([(
            Said(said.into()),
            Read {
                results,
                inventory: Inventory::Undeclared,
            },
        )]),
    }
}

/// A reported result of rule `r` at `path`, line `line`, its fingerprint unknown.
#[must_use]
pub fn result(path: &str, line: u32) -> Reported {
    Reported {
        path: RepoPath(path.into()),
        place: crate::domain::violations::types::Place::Line(
            crate::domain::violations::types::LineNumber(line),
        ),
        end: crate::outbound::sarif::types::End {
            line: crate::outbound::sarif::types::Bound::Absent,
            column: crate::outbound::sarif::types::Bound::Absent,
        },
        rule: crate::domain::violations::types::RuleId("r".into()),
        level: Level::Warning,
        suppression: Suppression::Unsuppressed,
        message: MessageText("m".into()),
        fingerprint: Fingerprinted::Unknown,
        supplied: Fingerprinted::Unknown,
    }
}

/// The result [`result`] gives, suppressed by an in-source ignore.
#[must_use]
pub fn ignored(path: &str, line: u32) -> Reported {
    Reported {
        suppression: Suppression::Suppressed(Justification("ignored here".into())),
        ..result(path, line)
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
