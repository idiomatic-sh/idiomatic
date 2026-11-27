//! The `coverage` service: an operator that answers from the absence of rows (an antijoin, a difference, a count) cannot trust it
//! where a relation it reads is read only in part. It drops the rows that the missing ones could change, and the plan answers
//! that it could not decide.

use ryo_relations::domain::relations::constants::{SYNTAX_ERRORS, UNPARSED};
use ryo_relations::domain::relations::service::{class_of, node_file};
use ryo_relations::domain::relations::types::Integer;
use ryo_relations::domain::relations::types::{CellKind, Class, FileId, Missing, Relation, Value};

use super::constants::REPOSITORY;
use super::constants::{UNKNOWN_BECAUSE, UNKNOWN_IN};
use super::types::{
    CoverageScope, DeclaredFiles, FileOrdinal, Lacking, LackingKind, Message,
    Partial, PartialPlace, Recovery,
};

use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::stored_facts::types::{PartialCause, PartialScope, ReadInPart};

/// The relations of a file that are read in part: those the reader said so of, and, when the whole file is only a recovery of what was
/// written (`everything` says how), every relation of code that was asked for.
#[must_use]
pub fn partial_of(said: &[ReadInPart], everything: Recovery, wanted: &[Relation]) -> Vec<Partial> {
    let mut partial: Vec<Partial> = said
        .iter()
        .map(|each| Partial {
            relation: each.relation,
            cause: PartialCause(each.cause.0.clone()),
            at: match each.at {
                PartialScope::OneFile(file) => PartialPlace::File(FileId(file.0)),
                PartialScope::Repository => PartialPlace::Repository,
            },
            lacking: match &each.lacking {
                Missing::Any => Lacking::Any,
                Missing::Kind(kind) => Lacking::Kind(LackingKind(kind.0.into())),
            },
        })
        .collect();
    let cause = match everything {
        Recovery::Whole => None,
        Recovery::SyntaxErrors | Recovery::LexicallyCompleteSyntaxErrors => Some(SYNTAX_ERRORS),
        Recovery::Unparsed => Some(UNPARSED),
    };
    if let Some(cause) = cause {
        for relation in wanted.iter().filter(|each| match everything {
            Recovery::SyntaxErrors | Recovery::Unparsed => matches!(
                class_of(**each),
                Class::Code | Class::Literal | Class::Document | Class::Lexical
            ),
            Recovery::LexicallyCompleteSyntaxErrors => matches!(
                class_of(**each),
                Class::Code | Class::Literal | Class::Document
            ),
            Recovery::Whole => false,
        }) {
            partial.push(Partial {
                relation: *relation,
                cause: PartialCause(cause.into()),
                lacking: Lacking::Any,
                at: PartialPlace::Repository,
            });
        }
    }
    partial.sort();
    partial.dedup();
    partial
}

/// The path that names the repository as a whole where an unknown names a file: where a rule that decided nothing anywhere in
/// the repository is unknown.
///
/// # Examples
///
/// ```
/// assert_eq!(&*ryo_execution_domain::domain::coverage::service::repository().0, ".");
/// ```
#[must_use]
pub fn repository() -> ryo_language::domain::files::types::FilePath {
    ryo_language::domain::files::types::FilePath(REPOSITORY.into())
}

/// What an operator that trusts an absence of rows says it could not decide, for a relation read in part in a file.
#[must_use]
pub fn unknown_message(partial: &Partial, scope: CoverageScope<'_>) -> Message {
    let path: &str = match scope {
        CoverageScope::Repository => REPOSITORY,
        CoverageScope::File(file) => &file.0,
    };
    Message(
        [
            ryo_relations::domain::relations::service::name(partial.relation).0,
            UNKNOWN_IN,
            path,
            UNKNOWN_BECAUSE,
            &*partial.cause.0,
        ]
        .concat()
        .into(),
    )
}

use ryo_values::domain::rows::types::Table;
