//! What the `linting` service hands the lint call and reads back, on values alone.

use std::collections::{BTreeMap, BTreeSet};

use super::service::{listed, listing_text, partitioned, refusals, repository_path, within_paths};
use super::types::RefusalReason;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::applications::types::{LintRuleId, Plan, PlannedRule};
use crate::domain::definitions::types::{PackagePath, QueryText};
use crate::domain::packages::types::PackageName;
use crate::domain::paths::types::RepoPath;

/// What a row of the cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// The files ryo reads, below the call's root and outside a `rules-exclude`.
    Listed,
    /// An answer's path below the call's root, and at the repository's root.
    RepositoryPath,
    /// The files of a local package left out of those the workspace governs.
    Partitioned,
    /// The listing's paths one to a line.
    ListingText,
    /// A rule whose query is refused, with no purpose to report it on.
    Refused,
    /// The paths a run is limited to, given from where it started.
    Within,
}

type Input = Scenario;
/// What the service answered, written out.
type Value = String;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("ryo_reads_the_files_below_the_root_but_those_excluded"),
        word: Word::Succeeds,
        input: || Scenario::Listed,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("an_answer_path_is_joined_to_the_root_unless_it_is_the_repository"),
        word: Word::Succeeds,
        input: || Scenario::RepositoryPath,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_local_package_is_not_governed_by_the_workspace"),
        word: Word::Succeeds,
        input: || Scenario::Partitioned,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_listing_is_one_path_to_a_line"),
        word: Word::Succeeds,
        input: || Scenario::ListingText,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_refused_query_refuses_its_rule"),
        word: Word::Succeeds,
        input: || Scenario::Refused,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("within_is_joined_to_where_the_run_started"),
        word: Word::Succeeds,
        input: || Scenario::Within,
        expect: Expect::Value(String::new()),
    },
];

fn paths(each: &[&str]) -> Vec<RepoPath> {
    each.iter().map(|path| RepoPath((*path).into())).collect()
}

fn shown(each: &[RepoPath]) -> String {
    each.iter()
        .map(|path| &*path.0)
        .collect::<Vec<_>>()
        .join(",")
}

/// What the scenario gave, and what it should give.
fn said(scenario: Scenario) -> (String, &'static str) {
    match scenario {
        Scenario::Listed => {
            let plan = Plan {
                base: RepoPath("a".into()),
                files: paths(&["a/x.txt", "a/y.txt", "b/z.txt"]),
                excluded: paths(&["a/y.txt"]).into_iter().collect(),
                ..Plan::default()
            };
            (shown(&listed(&plan)), "x.txt")
        }
        Scenario::RepositoryPath => {
            let under = repository_path(&RepoPath("a".into()), RepoPath("x.txt".into()));
            let root = repository_path(&RepoPath::default(), RepoPath("x.txt".into()));
            (shown(&[under, root]), "a/x.txt,x.txt")
        }
        Scenario::Partitioned => {
            let split = partitioned(
                &paths(&["src/lib.txt", "vendor/p/lib.txt"]),
                &[PackagePath {
                    name: PackageName("p".into()),
                    path: RepoPath("vendor/p".into()),
                }],
            );
            let packages = split
                .local_packages
                .iter()
                .map(|package| format!("{}@{}", package.package.0, package.directory.0))
                .collect::<Vec<_>>()
                .join(",");
            (
                format!("{packages}|{}", shown(&split.governed)),
                "p@vendor/p|src/lib.txt",
            )
        }
        Scenario::ListingText => (
            String::from_utf8(listing_text(&paths(&["a", "b/c"])).0).unwrap_or_default(),
            "a\nb/c",
        ),
        Scenario::Refused => {
            let plan = Plan {
                rules: vec![PlannedRule {
                    id: LintRuleId("r".into()),
                    query: QueryText("q".into()),
                    parameters: BTreeMap::new(),
                    include: Vec::new(),
                    exclude: Vec::new(),
                }],
                ..Plan::default()
            };
            let invalid = BTreeMap::from([(QueryText("q".into()), RefusalReason("bad".into()))]);
            let mut findings = Vec::new();
            let refused: BTreeSet<LintRuleId> = refusals(&plan, &invalid, &mut findings);
            (
                format!(
                    "{}|{}",
                    refused
                        .iter()
                        .map(|id| &*id.0)
                        .collect::<Vec<_>>()
                        .join(","),
                    findings.len()
                ),
                "r|0",
            )
        }
        Scenario::Within => (
            shown(&within_paths(&RepoPath("crates".into()), &paths(&["a"]))),
            "crates/a",
        ),
    }
}

fn subject(scenario: Input) -> Value {
    let (said, wanted) = said(scenario);
    if said == wanted { String::new() } else { said }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
