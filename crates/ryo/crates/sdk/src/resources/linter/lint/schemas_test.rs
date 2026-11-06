//! What the schemas of this entry accept and refuse, one row of `CASES` for each input.

use super::{report, request};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::cell::types::{Cell as Typed, HostText, Slot};
use crate::domain::file::types::RelativePath;
use crate::resources::linter::lint::types::Glob;
use crate::resources::linter::lint::types::Place;
use crate::resources::linter::lint::types::RuleId;
use crate::resources::linter::{Corpus, LintRequest, Rule, RuleApplicability, SkipReason};
use ryo_core::inbound::use_cases::lint::types::LintReport as CoreReport;
use ryo_core::outbound::row_sink::factory::collect;
use ryo_core::outbound::row_sink::interface::RowSinkPort as _;
use ryo_core::outbound::row_sink::types::LentCell;
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::row_texts::types::{Cell, CellText};

/// A text and what it is offered as.
#[derive(Clone, Copy, Debug)]
enum Offered {
    /// The public SDK policy, converted into a core request.
    Policy(RuleApplicability),
    /// A core skip reason, converted into the SDK report.
    Skipped,
    Id(&'static str),
    Glob(&'static str),
    /// A request that selects the language its source is read as.
    SourceLanguage,
    /// A request whose project paths are written in other forms than the one the core reads.
    ProjectPaths,
    /// The typed cells of a finding the run wrote: rule, subject, how many key values and each, each column as its heading and its
    /// value, line and character.
    Row(&'static [Option<Cell<CellText<'static>>>]),
    /// The same, read for its columns: the count of columns when each reads whole, its name and its value typed as the SDK hands it.
    Columns(&'static [Option<Cell<CellText<'static>>>]),
}

/// What a row gives the schemas: a text and what it is offered as.
type Input = Offered;
/// How long the text the type holds is; for a finding, its line times a thousand plus its character, or zero with no place.
type Value = usize;
/// Why the schemas refuse.
type Failure = Refused;

/// Which conversion refused.
#[derive(Debug, PartialEq, Eq)]
enum Refused {
    Policy,
    Id,
    Glob,
    Row,
    Columns,
}

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("explicit_source_language_is_preserved_at_sdk_request_boundary"),
        word: Word::Succeeds,
        input: || Offered::SourceLanguage,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("project_paths_are_normalized_before_observation_or_rejected"),
        word: Word::Succeeds,
        input: || Offered::ProjectPaths,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("sdk_always_policy_converts_to_core"),
        word: Word::Succeeds,
        input: || Offered::Policy(RuleApplicability::Always),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("sdk_accepted_syntax_policy_converts_to_core"),
        word: Word::Succeeds,
        input: || Offered::Policy(RuleApplicability::AcceptedSyntax),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("core_syntax_skip_reason_converts_to_sdk"),
        word: Word::Succeeds,
        input: || Offered::Skipped,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_rule_id_with_text_is_accepted"),
        word: Word::Succeeds,
        input: || Offered::Id("no-env-reads"),
        expect: Expect::Value(12),
    },
    Case {
        name: CaseName("a_blank_rule_id_is_refused"),
        word: Word::Refuses,
        input: || Offered::Id("  "),
        expect: Expect::Error(Refused::Id),
    },
    Case {
        name: CaseName("a_glob_that_parses_is_accepted"),
        word: Word::Succeeds,
        input: || Offered::Glob("src/**/*.rs"),
        expect: Expect::Value(11),
    },
    Case {
        name: CaseName("a_glob_that_does_not_parse_is_refused"),
        word: Word::Refuses,
        input: || Offered::Glob("["),
        expect: Expect::Error(Refused::Glob),
    },
    Case {
        name: CaseName("a_finding_carries_its_line_and_character"),
        word: Word::Succeeds,
        input: || {
            Offered::Row(&[
                Some(Cell::Text(CellText("r"))),
                Some(Cell::File(CellText("a.rs"))),
                Some(Cell::Number(Integer(1))),
                Some(Cell::Text(CellText("k"))),
                Some(Cell::Number(Integer(2))),
                Some(Cell::Number(Integer(4))),
            ])
        },
        expect: Expect::Value(2004),
    },
    Case {
        name: CaseName("a_finding_with_no_place_has_none"),
        word: Word::Succeeds,
        input: || {
            Offered::Row(&[
                Some(Cell::Text(CellText("r"))),
                Some(Cell::File(CellText("a.rs"))),
                Some(Cell::Number(Integer(1))),
                Some(Cell::Text(CellText("k"))),
                None,
                None,
            ])
        },
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_finding_carries_every_column_of_its_row_whole"),
        word: Word::Succeeds,
        input: || {
            Offered::Columns(&[
                Some(Cell::Text(CellText("r"))),
                Some(Cell::File(CellText("src/a.rs"))),
                Some(Cell::Number(Integer(0))),
                Some(Cell::Text(CellText("path"))),
                Some(Cell::File(CellText("src/a.rs"))),
                Some(Cell::Text(CellText("span"))),
                Some(Cell::Text(CellText("1:2,3:4"))),
                None,
                None,
            ])
        },
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a_column_with_no_heading_is_refused"),
        word: Word::Refuses,
        input: || {
            Offered::Columns(&[
                Some(Cell::Text(CellText("r"))),
                Some(Cell::File(CellText("src/a.rs"))),
                Some(Cell::Number(Integer(0))),
                Some(Cell::Text(CellText("path"))),
                Some(Cell::File(CellText("src/a.rs"))),
                Some(Cell::Number(Integer(9))),
                None,
                None,
            ])
        },
        expect: Expect::Error(Refused::Columns),
    },
    Case {
        name: CaseName("a_finding_whose_line_is_no_number_is_refused"),
        word: Word::Refuses,
        input: || {
            Offered::Row(&[
                Some(Cell::Text(CellText("r"))),
                Some(Cell::File(CellText("a.rs"))),
                Some(Cell::Number(Integer(1))),
                Some(Cell::Text(CellText("k"))),
                Some(Cell::Text(CellText("two"))),
                Some(Cell::Number(Integer(4))),
            ])
        },
        expect: Expect::Error(Refused::Row),
    },
];

/// The SDK's public applicability value becomes exactly one core rule policy.
fn converted_policy(applicability: RuleApplicability) -> Result<Value, Failure> {
    let id = RuleId::try_from("policy").map_err(|_error| Refused::Policy)?;
    let query =
        crate::resources::query::RyoQlQuery::try_from("emit \"syntax-tree\" subject file key(id);");
    let query = query.map_err(|_error| Refused::Policy)?;
    let converted = request(LintRequest {
        project_search_scope: Default::default(),
        given_project_inventory: Default::default(),
        source_language: None,
        root: std::path::PathBuf::from("."),
        rules: [(
            id,
            Rule {
                observations: Vec::new(),
                query,
                applicability,
                parameters: std::collections::BTreeMap::new(),
                include: Vec::new(),
                exclude: Vec::new(),
            },
        )]
        .into_iter()
        .collect(),
        files: Vec::new(),
        hosted: Vec::new(),
        corpus: Corpus::Given(Vec::new()),
        beside: Vec::new(),
    })
    .map_err(|_error| Refused::Policy)?;
    let expected = match applicability {
        RuleApplicability::Always => ryo_values::domain::runs::types::RuleApplicability::Always,
        RuleApplicability::AcceptedSyntax => {
            ryo_values::domain::runs::types::RuleApplicability::AcceptedSyntax
        }
    };
    Ok(usize::from(
        converted.rules.len() == 1 && converted.rules[0].applicability == expected,
    ))
}

/// A core skip retains its identity and reason in SDK-owned values.
fn converted_skip() -> Result<Value, Failure> {
    let mut core = CoreReport::default();
    core.skipped.push(ryo_values::domain::runs::types::Skipped {
        rule: ryo_values::domain::runs::types::RuleId("policy".into()),
        path: ryo_language::domain::files::types::FilePath("bad.py".into()),
        reason: ryo_values::domain::runs::types::SkipReason::SyntaxRejected,
    });
    let converted = report(
        ryo_core::outbound::row_sink::types::Collector::default(),
        core,
    )
    .map_err(|_error| Refused::Policy)?;
    Ok(usize::from(
        converted.skipped.len() == 1
            && converted.skipped[0].rule.as_ref() == "policy"
            && converted.skipped[0].path == RelativePath("bad.py".into())
            && converted.skipped[0].reason == SkipReason::SyntaxRejected,
    ))
}

/// Converts one input through the schemas and answers how long the text the type holds is.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Offered::Policy(applicability) => converted_policy(applicability),
        Offered::Skipped => converted_skip(),
        Offered::Id(text) => RuleId::try_from(text)
            .map(|id| id.to_string().len())
            .map_err(|_refused| Refused::Id),
        Offered::Glob(text) => Glob::try_from(text)
            .map(|glob| glob.to_string().len())
            .map_err(|_refused| Refused::Glob),
        Offered::SourceLanguage => {
            Ok(explicit_source_language_is_preserved_at_sdk_request_boundary())
        }
        Offered::ProjectPaths => Ok(project_paths_are_normalized_before_observation_or_rejected()),
        Offered::Columns(cells) => {
            let read = written(cells).ok_or(Refused::Columns)?.columns;
            let whole = read.first().is_some_and(|column| {
                &*column.name.0 == "path"
                    && column.value == Slot::Is(Typed::File(RelativePath("src/a.rs".into())))
            }) && read.get(1).is_none_or(|column| {
                &*column.name.0 == "span"
                    && column.value == Slot::Is(Typed::Text(HostText("1:2,3:4".into())))
            });
            Ok(if whole { read.len() } else { 0 })
        }
        Offered::Row(cells) => {
            let finding = written(cells).ok_or(Refused::Row)?;
            match finding.place {
                Place::At { line, column } => {
                    Ok(usize::try_from(line.0 * 1000 + column.0).unwrap_or(0))
                }
                Place::Missing => Ok(0),
            }
        }
    }
}

/// The one finding the report reads of a run that wrote one row of `cells`; none when the report refuses the row.
fn written(
    cells: &[Option<Cell<CellText<'_>>>],
) -> Option<crate::resources::linter::lint::types::Finding> {
    let collected = ryo_core::outbound::row_sink::types::Collector::default();
    let collector = collect(collected.clone());
    collector.row(&mut cells.iter().copied().map(LentCell::from));
    match report(collected, CoreReport::default()) {
        Ok(read) => read.findings.into_iter().next(),
        Err(_refused) => None,
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

/// The explicit source language of a request is the one the core is handed.
fn explicit_source_language_is_preserved_at_sdk_request_boundary() -> usize {
    let converted = request(LintRequest {
        project_search_scope: Default::default(),
        given_project_inventory: Default::default(),
        source_language: Some(crate::domain::language::types::LanguageName(
            "selected-provider".into(),
        )),
        root: std::path::PathBuf::from("."),
        rules: std::collections::BTreeMap::new(),
        files: vec![],
        hosted: vec![],
        corpus: Corpus::Given(vec![]),
        beside: vec![],
    })
    .expect("request conversion");
    assert_eq!(
        converted.source_language,
        Some(ryo_values::domain::descriptions::types::LanguageName(
            "selected-provider".into()
        ))
    );
    1
}

/// A repository-relative path is normalized before it is observed, and any other is refused.
fn project_paths_are_normalized_before_observation_or_rejected() -> usize {
    use crate::resources::linter::lint::types::{
        GivenProjectInventory, ProjectEntryObservation, ProjectEntryState, ProjectSearchScope,
    };
    let mut request = LintRequest {
        project_search_scope: ProjectSearchScope::Declared(vec![RelativePath("src//".into())]),
        given_project_inventory: GivenProjectInventory {
            entries: vec![ProjectEntryObservation {
                path: RelativePath("src/./pkg.py".into()),
                state: ProjectEntryState::File,
            }],
            ..Default::default()
        },
        source_language: None,
        root: std::path::PathBuf::from("."),
        rules: std::collections::BTreeMap::new(),
        files: vec![],
        hosted: vec![],
        corpus: Corpus::Given(vec![]),
        beside: vec![],
    };
    super::project_context(&mut request).expect("repository-relative paths");
    assert_eq!(
        request.project_search_scope,
        ProjectSearchScope::Declared(vec![RelativePath("src".into())])
    );
    assert_eq!(
        request.given_project_inventory.entries[0].path,
        RelativePath("src/pkg.py".into())
    );
    for invalid in ["/outside", "../outside", "src/../outside", "C:\\outside"] {
        let mut path = RelativePath(invalid.into());
        assert!(matches!(
            super::project_path(&mut path, true),
            Err(super::LintError::ProjectPath { .. })
        ));
    }
    1
}
