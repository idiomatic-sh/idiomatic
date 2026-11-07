//! Coverage gates require the entire pinned default inventory.
use super::constants;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::suite::service::{
    require_complete, validate_case_sources, validate_cases, validate_selected_rules,
};
use crate::domain::suite::types::audit::value::Audit;
use ryo::linter::{
    GivenProjectInventory, ProjectEntryObservation, ProjectEntryState, RelativePath, RuleId,
    SourceText,
};
use ryo_benchmark_contracts::domain::context::types::CaseContext;
use ryo_benchmark_contracts::domain::reports::types::SourceByteOffset;
use ryo_benchmark_contracts::domain::workloads::types::Case as SuiteCase;
use ryo_benchmark_contracts::domain::workloads::types::CaseComparison;
use ryo_benchmark_contracts::domain::workloads::types::Cases;
use ryo_benchmark_contracts::domain::workloads::types::Expected;
use ryo_benchmark_contracts::domain::workloads::types::ExpectedRule;
use ryo_benchmark_contracts::domain::workloads::types::benchmark_case_name::value::BenchmarkCaseName;
use std::borrow::Cow;

/// What a case hands the part under test.
enum Check {
    /// The audit of the pinned defaults.
    Defaults(Audit),
    /// The cases of one rule, which must hold a positive and a negative.
    Polarity(RuleId, Cases),
    /// The sources of the cases alone.
    Sources(SuiteCase),
    /// An explicit rule selection.
    Selection(Option<Vec<RuleId>>),
}

type Input = Check;
type Value = ();
type Failure = Cow<'static, str>;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("complete defaults accepted"),
        word: Word::Succeeds,
        input: || {
            Check::Defaults(Audit {
                implemented: vec![RuleId::try_from("fixture").unwrap()],
                missing: Vec::new(),
            })
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("one positive and one negative full-report case are valid"),
        word: Word::Succeeds,
        input: || Check::Polarity(RuleId::try_from("F601").expect("rule identity"), polarity()),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a requested missing input with an absence seal is valid"),
        word: Word::Succeeds,
        input: || Check::Sources(explicit_missing()),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a context root spelled dot keeps the existing context path meaning"),
        word: Word::Succeeds,
        input: || {
            edited(explicit_missing(), |case| {
                case.context.as_mut().unwrap().logical_root = ".".into();
                case.context.as_mut().unwrap().missing = vec!["missing.py".into()];
            })
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a given project inventory stages an unselected local module"),
        word: Word::Succeeds,
        input: || Check::Sources(sibling(ProjectEntryState::File)),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("no explicit selection keeps the historical profile selection"),
        word: Word::Succeeds,
        input: || Check::Selection(None),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("one unique selected rule is accepted"),
        word: Word::Succeeds,
        input: || Check::Selection(Some(vec![RuleId::try_from("F401").unwrap()])),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("one missing default prevents measurement"),
        word: Word::Refuses,
        input: || {
            Check::Defaults(Audit {
                implemented: Vec::new(),
                missing: vec![RuleId::try_from("fixture").unwrap()],
            })
        },
        expect: Expect::Error(Cow::Borrowed(
            "default coverage incomplete: 1 missing rules: fixture",
        )),
    },
    Case {
        name: CaseName("empty internal audit is refused"),
        word: Word::Refuses,
        input: || {
            Check::Defaults(Audit {
                implemented: Vec::new(),
                missing: Vec::new(),
            })
        },
        expect: Expect::Error(Cow::Borrowed(constants::EMPTY_AUDIT)),
    },
    Case {
        name: CaseName("cases without a negative are refused"),
        word: Word::Refuses,
        input: || {
            let cases = polarity();
            assert_eq!(
                cases.cases[0].expected.len(),
                2,
                "duplicate findings remain significant"
            );
            Check::Polarity(
                RuleId::try_from("F601").expect("rule identity"),
                Cases {
                    cases: vec![cases.cases[0].clone()],
                },
            )
        },
        expect: Expect::Error(Cow::Borrowed(constants::MISSING_POLARITY)),
    },
    Case {
        name: CaseName("ordinary source omission is not a sealed absence"),
        word: Word::Refuses,
        input: || {
            edited(explicit_missing(), |case| {
                case.context.as_mut().unwrap().missing.clear();
            })
        },
        expect: Expect::Error(Cow::Borrowed(
            constants::REQUESTED_UNAVAILABLE_SOURCE_LACKS_A_CONTEXT,
        )),
    },
    Case {
        name: CaseName("there is no source byte at a missing path"),
        word: Word::Refuses,
        input: || edited(explicit_missing(), |case| case.expected[0].end = SourceByteOffset(1)),
        expect: Expect::Error(Cow::Borrowed(constants::MISSING_SOURCE)),
    },
    Case {
        name: CaseName("requested paths cannot be duplicated"),
        word: Word::Refuses,
        input: || {
            edited(explicit_missing(), |case| {
                case.requested_files
                    .as_mut()
                    .unwrap()
                    .push(RelativePath("missing.py".into()));
            })
        },
        expect: Expect::Error(Cow::Borrowed(
            constants::EXPLICIT_SOURCE_INVENTORY_MUST_NAME_EACH,
        )),
    },
    Case {
        name: CaseName("readable source inventory cannot silently shrink"),
        word: Word::Refuses,
        input: || {
            edited(explicit_missing(), |case| {
                case.requested_files.as_mut().unwrap().remove(0);
            })
        },
        expect: Expect::Error(Cow::Borrowed(
            constants::SEALED_DISK_CONTEXT_MUST_SELECT_EVERY,
        )),
    },
    Case {
        name: CaseName("unselected bytes need an observation"),
        word: Word::Refuses,
        input: || {
            edited(sibling(ProjectEntryState::File), |case| {
                case.given_project_inventory = None;
            })
        },
        expect: Expect::Error(Cow::Borrowed(
            constants::UNSELECTED_GIVEN_SOURCE_REQUIRES_AN_EXPLICIT,
        )),
    },
    Case {
        name: CaseName("staged bytes cannot also be absent"),
        word: Word::Refuses,
        input: || Check::Sources(sibling(ProjectEntryState::Missing)),
        expect: Expect::Error(Cow::Borrowed(
            constants::UNSELECTED_GIVEN_SOURCE_LACKS_A_MATCHING,
        )),
    },
    Case {
        name: CaseName("an empty explicit selection is refused"),
        word: Word::Refuses,
        input: || Check::Selection(Some(Vec::new())),
        expect: Expect::Error(Cow::Borrowed(constants::SELECTED_RULES_MUST_BE_NONEMPTY_AND)),
    },
    Case {
        name: CaseName("a duplicated selected rule is refused"),
        word: Word::Refuses,
        input: || {
            let rule = RuleId::try_from("F401").unwrap();
            Check::Selection(Some(vec![rule.clone(), rule]))
        },
        expect: Expect::Error(Cow::Borrowed(constants::SELECTED_RULES_MUST_BE_NONEMPTY_AND)),
    },
];

fn edited(mut case: SuiteCase, change: impl FnOnce(&mut SuiteCase)) -> Input {
    change(&mut case);
    Check::Sources(case)
}

/// A case with every optional declaration absent.
fn bare(name: &str) -> SuiteCase {
    SuiteCase {
        performance: Default::default(),
        selected_rules: None,
        rule_observations: None,
        rule_context: None,
        requested_files: None,
        project_search_roots: None,
        given_project_inventory: None,
        file_metadata: None,
        reference_discrepancy: None,
        context: None,
        comparison: CaseComparison::Full,
        name: BenchmarkCaseName(name.into()),
        files: Default::default(),
        expected: Vec::new(),
    }
}

/// One full-report case holding two findings of the rule, and one holding only another rule's.
fn polarity() -> Cases {
    let rule = RuleId::try_from("F601").expect("rule identity");
    let other_rule = RuleId::try_from("B017").expect("other rule identity");
    let file = RelativePath("case.py".into());
    let expected = |identity| Expected {
        identity: ExpectedRule::Explicit(identity),
        file: file.clone(),
        start: SourceByteOffset(0),
        end: SourceByteOffset(1),
    };
    let source = || std::collections::BTreeMap::from([(file.clone(), SourceText("x".into()))]);
    Cases {
        cases: vec![
            SuiteCase {
                files: source(),
                expected: vec![expected(rule.clone()), expected(rule)],
                ..bare("positive")
            },
            SuiteCase {
                files: source(),
                expected: vec![expected(other_rule)],
                ..bare("other-rule-only")
            },
        ],
    }
}

/// A readable companion plus a requested input that the context declares absent.
fn explicit_missing() -> SuiteCase {
    let readable = RelativePath("companion.py".into());
    let missing = RelativePath("missing.py".into());
    SuiteCase {
        requested_files: Some(vec![readable.clone(), missing.clone()]),
        context: Some(CaseContext {
            logical_root: "project".into(),
            auxiliary_files: Default::default(),
            missing: vec!["project/missing.py".into()],
        }),
        files: [(readable, SourceText("x = 1\n".into()))].into(),
        expected: vec![Expected {
            identity: ExpectedRule::Explicit(RuleId::try_from("R1").unwrap()),
            file: missing,
            start: SourceByteOffset(0),
            end: SourceByteOffset(0),
        }],
        ..bare("explicit-missing")
    }
}

/// An importer selected alone, with its sibling module staged but unselected and observed as `state`.
fn sibling(state: ProjectEntryState) -> SuiteCase {
    let importer = RelativePath("main.py".into());
    let sibling = RelativePath("localmod.py".into());
    SuiteCase {
        requested_files: Some(vec![importer.clone()]),
        project_search_roots: Some(vec![RelativePath("".into())]),
        given_project_inventory: Some(GivenProjectInventory {
            entries: vec![ProjectEntryObservation {
                path: sibling.clone(),
                state,
            }],
            completeness: Default::default(),
        }),
        files: [
            (importer, SourceText("import localmod\n".into())),
            (sibling, SourceText("value = 1\n".into())),
        ]
        .into(),
        ..bare("unselected-sibling")
    }
}

fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Check::Defaults(audit) => require_complete(&audit),
        Check::Polarity(rule, cases) => validate_cases(&rule, &cases),
        Check::Sources(case) => validate_case_sources(&Cases { cases: vec![case] }),
        Check::Selection(rules) => validate_selected_rules(rules.as_deref()),
    }
    .map_err(|error| error.to_string().into())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[cfg(test)]
mod audit_boundary_test;
