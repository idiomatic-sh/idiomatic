//! The `template_placing` service's cases, on values alone.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::errors::TemplatePlacingError;
use super::service::{exclusion, names_parts};
use super::types::PartNaming;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::definitions::types::{LoadedWorkspace, WorkspaceSection};
use crate::domain::entries::types::Composition;
use crate::domain::findings::types::WorkspaceName;
use crate::domain::packages::types::DefinitionName;
use crate::domain::paths::types::RepoPath;
use crate::domain::selectors::types::{
    PartSelector, PathPattern, PathTest, PatternFailure, PlacedBySelector, Recursion, RegexText,
    Selector, Selectors, Spelling,
};
use crate::outbound::matching::interface::MatchingPort as _;

/// What a row asks of the service.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// Whether a list of one glob names parts.
    Glob,
    /// Whether a list of one part selector names parts.
    Part,
    /// Whether a list of one placed-by selector names parts.
    PlacedBy,
    /// The files the workspace's `rules-exclude` of `docs/**` covers.
    Excluded,
    /// The files a `rules-exclude` whose regex does not compile covers.
    BrokenExclude,
}

/// What the row asks.
type Input = Question;
/// What the service answers, in words.
type Value = String;
type Failure = TemplatePlacingError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_glob_names_no_part"),
        word: Word::Succeeds,
        input: || Question::Glob,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_part_selector_names_the_instances_of_a_spec"),
        word: Word::Succeeds,
        input: || Question::Part,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_placed_by_selector_names_the_files_an_entry_placed"),
        word: Word::Succeeds,
        input: || Question::PlacedBy,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_rules_exclude_covers_the_files_beneath_its_glob"),
        word: Word::Succeeds,
        input: || Question::Excluded,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_rules_exclude_that_does_not_compile_is_refused"),
        word: Word::Refuses,
        input: || Question::BrokenExclude,
        expect: Expect::Refusal(|actual| {
            if actual
                == &(TemplatePlacingError::Selector {
                    entry: "workspace".into(),
                    source: crate::domain::selectors::errors::SelectorsError::Pattern {
                        pattern: "(".into(),
                        reason: "regex parse error:\n    (\n    ^\nerror: unclosed group".into(),
                    },
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
];

fn compile(spelling: &Spelling) -> Result<Arc<dyn PathTest>, PatternFailure> {
    crate::outbound::matching::factory::globset().compile(spelling)
}

fn workspace(rules_exclude: Selector) -> LoadedWorkspace {
    LoadedWorkspace {
        file: RepoPath("tastemaker.toml".into()),
        directory: RepoPath::default(),
        section: WorkspaceSection {
            name: WorkspaceName("w".into()),
            rules_exclude: Selectors(vec![rules_exclude]),
            members: Selectors::default(),
            dependencies: Vec::new(),
        },
        composition: Composition::default(),
        locals: Vec::new(),
        packages: BTreeMap::new(),
        specs: BTreeMap::new(),
        rules: BTreeMap::new(),
        templates: BTreeMap::new(),
        latest: BTreeMap::new(),
        files: vec![RepoPath("docs/a.md".into()), RepoPath("src/lib.txt".into())],
        local_packages: Vec::new(),
    }
}

/// The answer, against the one the row's case expects: empty when they agree.
fn subject(question: Input) -> Result<Value, Failure> {
    let (said, expected) = match question {
        Question::Glob | Question::Part | Question::PlacedBy => {
            let (selector, expected) = match question {
                Question::Part => (
                    Selector::Part(PartSelector {
                        part: DefinitionName("hex".into()),
                        recursion: Recursion::Direct,
                    }),
                    PartNaming::NamesParts,
                ),
                Question::PlacedBy => (
                    Selector::PlacedBy(PlacedBySelector {
                        entry: DefinitionName("errors".into()),
                    }),
                    PartNaming::NamesParts,
                ),
                Question::Glob | Question::Excluded | Question::BrokenExclude => (
                    Selector::Glob(PathPattern("src/**".into())),
                    PartNaming::NamesNone,
                ),
            };
            let named = names_parts(&[&Selectors(vec![selector])]);
            (format!("{named:?}"), format!("{expected:?}"))
        }
        Question::Excluded | Question::BrokenExclude => {
            let selector = match question {
                Question::BrokenExclude => Selector::Regex(RegexText("(".into())),
                Question::Glob | Question::Part | Question::PlacedBy | Question::Excluded => {
                    Selector::Glob(PathPattern("docs/**".into()))
                }
            };
            let excluded = exclusion(&workspace(selector), &compile, &[])?;
            (
                excluded
                    .iter()
                    .map(|path| path.0.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
                "docs/a.md".to_owned(),
            )
        }
    };
    Ok(if said == expected {
        String::new()
    } else {
        said
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
