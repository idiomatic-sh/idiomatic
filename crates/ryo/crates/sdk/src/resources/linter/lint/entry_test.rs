//! What `linter lint` promises its caller, one row of `CASES` for each case it has.

use std::num::NonZeroUsize;

use super::entry::lint;
use super::errors::LintError;
use super::fixtures::capability_cache::capability_cache;
use super::fixtures::configured_scope::configured_scope;
use super::fixtures::excluded_producer::excluded_producer;
use super::fixtures::hosted_text_identity::hosted_text_identity;
use super::fixtures::outcome::Outcome;
use super::fixtures::permission::{permission_bytes, permission_unavailable};
use super::fixtures::producer_without_query::producer_without_query;
use super::fixtures::queries::{
    HOSTED, LITERAL_PARSE_POSITIVE, MISSPELLED, OPTIONAL_ABSENCE, OPTIONAL_POSITIVE,
    RAW_FORMAT_POSITIVE,
};
use super::fixtures::re_exported_publication::re_exported_publication;
use super::fixtures::refusal::Refusal;
use super::fixtures::refused_and_empty_producers::refused_and_empty_producers;
use super::fixtures::request::{
    Given, Hosting, LINTED, Linting, Reads, Root, cache, memory, request,
};
use super::fixtures::shared_calls::shared;
use super::fixtures::unbound_export::unbound_export;
use super::fixtures::zero_finding_producer::zero_finding_producer;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::language::types::Languages;
use crate::resources::linter::lint::types::Workers;

/// What a row gives the function: the rule's text, where the repository is and what else the caller states.
type Input = Given;
/// How many findings, how many files nothing read, whether the repository holds nothing but what it held, and how many columns the
/// first finding carries.
type Value = Outcome;
/// Why the function ends in error.
type Failure = Refusal;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("literal_parse_contract_changes_cannot_replay_cached_absence"),
        word: Word::Succeeds,
        input: || Given {
            linter: Linting::Capabilities,
            query: LITERAL_PARSE_POSITIVE,
            ..LINTED
        },
        expect: Expect::Value((1, 0, true, 0)),
    },
    Case {
        name: CaseName("raw_format_contract_changes_cannot_replay_cached_absence"),
        word: Word::Succeeds,
        input: || Given {
            linter: Linting::Capabilities,
            query: RAW_FORMAT_POSITIVE,
            ..LINTED
        },
        expect: Expect::Value((1, 0, true, 0)),
    },
    Case {
        name: CaseName("optional_contract_changes_cannot_replay_cached_absence"),
        word: Word::Succeeds,
        input: || Given {
            linter: Linting::Capabilities,
            query: OPTIONAL_ABSENCE,
            ..LINTED
        },
        expect: Expect::Value((1, 0, true, 0)),
    },
    Case {
        name: CaseName("optional_contract_changes_cannot_replay_cached_findings"),
        word: Word::Succeeds,
        input: || Given {
            linter: Linting::Capabilities,
            query: OPTIONAL_POSITIVE,
            ..LINTED
        },
        expect: Expect::Value((1, 0, true, 0)),
    },
    Case {
        name: CaseName(
            "unchanged_bytes_and_a_given_timestamp_do_not_reuse_the_findings_of_an_old_permission",
        ),
        word: Word::Succeeds,
        input: || Given {
            linter: Linting::Scenario(permission_bytes),
            ..LINTED
        },
        expect: Expect::Value((1, 1, true, 1)),
    },
    Case {
        name: CaseName("an_unavailable_permission_is_neither_false_nor_a_proven_absence"),
        word: Word::Succeeds,
        input: || Given {
            linter: Linting::Scenario(permission_unavailable),
            ..LINTED
        },
        expect: Expect::Value((1, 1, true, 1)),
    },
    Case {
        name: CaseName("changed_hosted_text_invalidates_retained_cross_file_findings"),
        word: Word::Succeeds,
        input: || Given {
            linter: Linting::Scenario(hosted_text_identity),
            ..LINTED
        },
        expect: Expect::Value((1, 0, true, 1)),
    },
    Case {
        name: CaseName(
            "a_changed_exclusion_and_changed_hosted_rows_change_what_a_retained_store_answers",
        ),
        word: Word::Succeeds,
        input: || Given {
            linter: Linting::Scenario(configured_scope),
            ..LINTED
        },
        expect: Expect::Value((2, 0, true, 1)),
    },
    Case {
        name: CaseName("a_later_rule_publishes_a_nonempty_observation_despite_zero_findings"),
        word: Word::Succeeds,
        input: || Given {
            linter: Linting::Scenario(zero_finding_producer),
            ..LINTED
        },
        expect: Expect::Value((1, 0, true, 1)),
    },
    Case {
        name: CaseName("an_excluded_producer_publishes_to_its_consumer_without_its_findings"),
        word: Word::Succeeds,
        input: || Given {
            linter: Linting::Scenario(excluded_producer),
            ..LINTED
        },
        expect: Expect::Value((1, 0, true, 1)),
    },
    Case {
        name: CaseName("a_re_exported_publication_survives_cold_warm_replay_and_a_fresh_engine"),
        word: Word::Succeeds,
        input: || Given {
            linter: Linting::Scenario(re_exported_publication),
            ..LINTED
        },
        expect: Expect::Value((1, 0, true, 2)),
    },
    Case {
        name: CaseName(
            "a_refused_and_a_truly_empty_producer_stay_distinct_on_cold_and_retained_replay",
        ),
        word: Word::Succeeds,
        input: || Given {
            linter: Linting::Scenario(refused_and_empty_producers),
            ..LINTED
        },
        expect: Expect::Value((0, 0, true, 0)),
    },
    Case {
        name: CaseName("a_selected_producer_with_no_query_is_never_a_complete_empty_input"),
        word: Word::Succeeds,
        input: || Given {
            linter: Linting::Scenario(producer_without_query),
            ..LINTED
        },
        expect: Expect::Value((0, 0, true, 0)),
    },
    Case {
        name: CaseName("an_unbound_export_demands_no_reader_and_changes_no_ordinary_result"),
        word: Word::Succeeds,
        input: || Given {
            linter: Linting::Scenario(unbound_export),
            ..LINTED
        },
        expect: Expect::Value((1, 0, true, 1)),
    },
    Case {
        name: CaseName("a_listed_corpus_reads_the_listed_files"),
        word: Word::Succeeds,
        input: || Given {
            query: HOSTED,
            hosted: Hosting::Placed,
            corpus: Reads::Listed,
            ..LINTED
        },
        expect: Expect::Value((1, 1, true, 2)),
    },
    Case {
        name: CaseName("a_listed_corpus_is_the_boundary_of_what_the_rules_see"),
        word: Word::Succeeds,
        input: || Given {
            query: HOSTED,
            hosted: Hosting::Placed,
            corpus: Reads::ListedNothing,
            ..LINTED
        },
        expect: Expect::Value((0, 0, true, 0)),
    },
    Case {
        name: CaseName("files_given_beside_the_disk_are_read_with_it"),
        word: Word::Succeeds,
        input: || Given {
            corpus: Reads::Beside,
            ..LINTED
        },
        expect: Expect::Value((0, 2, true, 0)),
    },
    Case {
        name: CaseName("lints_and_keeps_nothing_in_the_repository"),
        word: Word::Succeeds,
        input: || LINTED,
        expect: Expect::Value((0, 1, true, 0)),
    },
    Case {
        name: CaseName(
            "lints_and_keeps_its_store_where_the_caller_says_a_directory_no_run_made_yet",
        ),
        word: Word::Succeeds,
        input: || Given {
            kept: true,
            ..LINTED
        },
        expect: Expect::Value((0, 1, true, 0)),
    },
    Case {
        name: CaseName("a_file_the_host_says_was_placed_is_a_row_a_rule_reads"),
        word: Word::Succeeds,
        input: || Given {
            query: HOSTED,
            hosted: Hosting::Placed,
            ..LINTED
        },
        expect: Expect::Value((1, 1, true, 2)),
    },
    Case {
        name: CaseName("files_given_as_text_are_read_instead_of_the_disk"),
        word: Word::Succeeds,
        input: || Given {
            query: HOSTED,
            hosted: Hosting::Placed,
            corpus: Reads::Given,
            ..LINTED
        },
        expect: Expect::Value((1, 1, true, 2)),
    },
    Case {
        name: CaseName("a_shared_linter_gives_what_fresh_linters_give"),
        word: Word::Succeeds,
        input: || Given {
            linter: Linting::Shared,
            ..LINTED
        },
        expect: Expect::Value((1, 1, true, 2)),
    },
    Case {
        name: CaseName("a_row_with_fewer_cells_than_its_relation_has_columns_is_refused"),
        word: Word::Refuses,
        input: || Given {
            query: HOSTED,
            hosted: Hosting::ShortRow,
            ..LINTED
        },
        expect: Expect::Error(Refusal::Host),
    },
    Case {
        name: CaseName("a_cell_that_is_not_of_the_kind_of_its_column_is_refused"),
        word: Word::Refuses,
        input: || Given {
            query: HOSTED,
            hosted: Hosting::WrongKind,
            ..LINTED
        },
        expect: Expect::Error(Refusal::Host),
    },
    Case {
        name: CaseName("two_relations_with_one_name_are_refused"),
        word: Word::Refuses,
        input: || Given {
            query: HOSTED,
            hosted: Hosting::Twice,
            ..LINTED
        },
        expect: Expect::Error(Refusal::Host),
    },
    Case {
        name: CaseName("a_rule_with_wrong_text_is_refused_with_its_id"),
        word: Word::Refuses,
        input: || Given {
            query: MISSPELLED,
            ..LINTED
        },
        expect: Expect::Error(Refusal::Rule),
    },
    Case {
        name: CaseName("a_repository_that_does_not_fit_the_budget_is_refused"),
        word: Word::Refuses,
        input: || Given {
            bytes: Some(1),
            ..LINTED
        },
        expect: Expect::Error(Refusal::TooLarge),
    },
    #[cfg(any(unix, windows))]
    Case {
        name: CaseName("a_root_that_is_not_text_is_refused"),
        word: Word::Refuses,
        input: || Given {
            root: Root::NotText,
            ..LINTED
        },
        expect: Expect::Error(Refusal::PathNotText),
    },
    Case {
        name: CaseName("a_root_that_is_not_there_fails_in_the_corpus"),
        word: Word::Fails,
        input: || Given {
            root: Root::Missing,
            ..LINTED
        },
        expect: Expect::Error(Refusal::Io),
    },
];

/// The refusal a row tells apart for the error `lint` ends in.
pub(crate) fn refusal(error: LintError) -> Refusal {
    match error {
        LintError::Execution { reason } => Refusal::Execution(reason),
        LintError::ObservationBindings { .. } => Refusal::ObservationBindings,
        LintError::ObservationExecution { .. } => Refusal::ObservationExecution,
        LintError::SourceLanguage { .. } => Refusal::SourceLanguage,
        LintError::Rule { .. } => Refusal::Rule,
        LintError::Unrunnable { .. } => Refusal::Unrunnable,
        LintError::MissingParameter { .. } => Refusal::MissingParameter,
        LintError::TooLarge { .. } => Refusal::TooLarge,
        LintError::PathNotText { .. } => Refusal::PathNotText,
        LintError::ProjectPath { .. } => Refusal::PathNotText,
        LintError::Io { .. } => Refusal::Io,
        LintError::Provider { .. } => Refusal::Provider,
        LintError::DuplicateHostRelation { .. }
        | LintError::HostRowWidth { .. }
        | LintError::HostCellKind { .. } => Refusal::Host,
        LintError::MemoryBudgetExceeded { .. } => Refusal::MemoryBudgetExceeded,
    }
}

/// Lints a directory holding one file, with no language linked: the file is unread.
fn subject(given: Input) -> Result<Value, Failure> {
    if matches!(given.linter, Linting::Capabilities) {
        return Ok(capability_cache(given.query));
    }
    if matches!(given.linter, Linting::Shared) {
        return Ok(shared());
    }
    if let Linting::Scenario(scenario) = given.linter {
        return Ok(scenario());
    }
    let root = tempfile::tempdir().expect("a temporary directory");
    let elsewhere = tempfile::tempdir().expect("a temporary directory");
    std::fs::write(root.path().join("notes.txt"), "text").expect("a file");
    let request = request(given, root.path());
    let report = lint(
        Languages::default(),
        &crate::domain::capacity::types::WorkerPool::from(Workers(NonZeroUsize::MIN)),
        memory(given),
        cache(given, elsewhere.path()),
        request,
    )
    .map_err(refusal)?;
    let untouched = std::fs::read_dir(root.path()).is_ok_and(|entries| entries.count() == 1)
        && (!given.kept || elsewhere.path().join("cache").join("ryo").is_dir());
    let columns = report
        .findings
        .first()
        .map_or(0, |finding| finding.columns.len());
    Ok((
        report.findings.len(),
        report.unread.len(),
        untouched,
        columns,
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
