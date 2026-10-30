//! The `applications` service's cases, over the `check` fixture's workspace: what a rule entry's selectors can name.

use super::fixtures::{placement, workspace};
use super::service::known;
use crate::cases::{Case, CaseName, Expect, Word, run};

/// The fixture's tree.
const FILES: &[&str] = &[
    "tastemaker.toml",
    "src/a/errors.txt",
    "src/a/stray.txt",
    "src/b/notes.txt",
];

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// The spec instances and the placed files a selector can name.
    Known,
    /// A file with a template's contract, whose rule findings are what they were, when it is settled again.
    ReusableWithAContract,
}

type Input = Question;
/// What the row said, empty when it said what was expected.
type Value = String;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_instance_and_each_placed_file_can_be_named"),
        word: Word::Succeeds,
        input: || Question::Known,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_file_a_template_places_is_settled_again_whatever_its_findings_were"),
        word: Word::Succeeds,
        input: || Question::ReusableWithAContract,
        expect: Expect::Value(String::new()),
    },
];

/// A file `a.toml` with the same findings as when it was last settled, and a file `b.toml` likewise: only `a.toml` has a template.
fn reusable_with_a_contract() -> Value {
    use super::service::reusable_paths;
    use crate::domain::applications::types::{Judged, Plan};
    use crate::domain::contracts::types::{
        Contract, RenderedContent, RowKey, RowNode, SyntaxKind, TemplateIdentity, WrittenValue,
    };
    use crate::domain::definitions::types::Exactness;
    use crate::domain::findings::types::Fingerprint;
    use crate::domain::paths::types::RepoPath;
    use std::collections::BTreeMap;
    let (placed, loose) = (RepoPath("a.toml".into()), RepoPath("b.toml".into()));
    let plan = Plan {
        contracts: vec![Contract {
            target: placed.clone(),
            rendered: RepoPath("rendered/a.toml".into()),
            text: RenderedContent("key = 1\n".into()),
            identity: TemplateIdentity("p/t".into()),
            exact: Exactness::Contains,
        }],
        ..Plan::default()
    };
    let had: BTreeMap<RepoPath, Fingerprint> = [
        (placed.clone(), Fingerprint(1)),
        (loose.clone(), Fingerprint(2)),
    ]
    .into_iter()
    .collect();
    // The template's rows are answered, so the comparison reads rows alone and no text of the target.
    let row = |syntax| crate::domain::contracts::types::DataRow {
        syntax,
        id: RowNode::default(),
        owner: RowNode::default(),
        key: RowKey::default(),
        kind: SyntaxKind::default(),
        text: WrittenValue::default(),
        span: crate::domain::contracts::types::RowPlace::Unplaced,
    };
    let judged = Judged {
        data: [
            (
                RepoPath("rendered/a.toml".into()),
                vec![row(crate::domain::contracts::types::Syntax::Entries)],
            ),
            (placed.clone(), Vec::new()),
        ]
        .into_iter()
        .collect(),
        ..Judged::default()
    };
    let reusable = reusable_paths(&plan, &judged, &had, &had, &[]);
    if reusable.contains(&loose) && !reusable.contains(&placed) {
        String::new()
    } else {
        format!("reusable: {reusable:?}")
    }
}

fn subject(question: Input) -> Value {
    let workspace = workspace(FILES);
    let placement = placement(&workspace);
    let (said, expected) = match question {
        Question::ReusableWithAContract => (reusable_with_a_contract(), String::new()),
        Question::Known => {
            let known = known(&workspace, &placement);
            (
                format!(
                    "{} | {}",
                    known
                        .instances
                        .iter()
                        .map(|instance| format!("{} {}", instance.directory.0, instance.spec.0))
                        .collect::<Vec<_>>()
                        .join(", "),
                    known
                        .placed
                        .iter()
                        .map(|placed| format!("{} {}", placed.path.0, placed.entry.0))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                "src/a part, src/b part | src/a/errors.txt errors".to_owned(),
            )
        }
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
