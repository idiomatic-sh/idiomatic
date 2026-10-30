//! What every `check_store` adapter must pass: the answer kept is loaded back whole, and keeping again replaces it.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::check_store::interface::CheckStorePort;

/// Whether a row keeps the whole answer, or only its files.
type Input = bool;
/// Whether the answer kept last is loaded back whole.
type Value = bool;
/// The text of the port's failure.
type Failure = String;

/// The contract's rows: every implementation runs them, in order.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_answer_kept_is_loaded_whole"),
        word: Word::Succeeds,
        input: || true,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("keeping_again_replaces_the_answer"),
        word: Word::Succeeds,
        input: || false,
        expect: Expect::Value(true),
    },
];

/// Runs the contract's rows against one implementation.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &impl CheckStorePort) {
    use crate::domain::digests::types::Digest;
    use crate::domain::files::types::Stamp;
    use crate::domain::findings::types::{Column, Finding, Fingerprint, Help, Line};
    use crate::domain::findings::types::{MessageText, Place, Reason, Region, RuleId, Suppression};
    use crate::outbound::check_store::types::{Checked, CheckedFile, CheckedPath, SourcedFile};
    let finding = |place, help| Finding {
        path: crate::domain::paths::types::RepoPath("src/a.rs".into()),
        place,
        rule: crate::domain::findings::service::of_rule(RuleId("p/a".into())),
        message: MessageText("a marker".into()),
        help,
        fingerprint: Fingerprint(7),
    };
    let file = |path: &str, size: u64| CheckedFile {
        path: CheckedPath(path.into()),
        digest: Digest(format!("d{size}").into()),
        stamp: Stamp {
            size: crate::domain::files::types::FileSize(size),
            modified: crate::domain::files::types::ModifiedTime(4),
        },
    };
    // A file and a package's file; a finding at a region, one at a line, one about a whole file and one accepted.
    let whole = Checked {
        files: vec![file("src/a.rs", 3), file("/p/rules/a/0.1.0/rule.toml", 5)],
        findings: vec![
            finding(
                Place::At(Region {
                    start_line: Line(1),
                    start_column: Column(2),
                    end_line: Line(3),
                    end_column: Column(4),
                }),
                Help::Said(MessageText("remove it".into())),
            ),
            finding(Place::Line(Line(5)), Help::Silent),
            finding(Place::Whole, Help::Silent),
        ],
        suppressed: vec![Suppression {
            finding: finding(Place::Line(Line(9)), Help::Silent),
            reason: Reason("it is generated".into()),
        }],
        evaluated: [
            crate::domain::findings::service::of_rule(RuleId("p/a".into())),
            crate::domain::findings::service::check_id("p/s", "count"),
        ]
        .into(),
        known: Digest("rules".into()),
        sourced: vec![SourcedFile {
            path: CheckedPath("src/b.rs".into()),
            incoming: Fingerprint(9),
        }],
    };
    run(CASES, Word::Succeeds, |all| -> Result<Value, Failure> {
        let kept = if all {
            whole.clone()
        } else {
            Checked {
                files: whole.files.clone(),
                ..Checked::default()
            }
        };
        adapter
            .keep_checked(&kept)
            .map_err(|error| error.to_string())?;
        Ok(adapter.checked() == kept)
    });
}
