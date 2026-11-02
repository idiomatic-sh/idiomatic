//! What the user does next about each error a command can end with (clig.dev, Errors),
//! printed as `help:` under it by `crate::output::fail`. Each product implements
//! [`Remedy`] here for every error type its commands fail with, and every match names
//! every variant: a wildcard is refused, so a new variant does not compile until it is
//! given its next step.
#![deny(clippy::wildcard_enum_match_arm)]

#[cfg(test)]
#[path = "remedy_test.rs"]
mod tests;

/// What the user does next about an error, printed as `help:` under it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Next {
    /// A command line that moves the user on.
    Run(String),
    /// What to change, said as an instruction.
    Fix(String),
    /// Nothing the user did: a bug, to report at the repository's issues.
    Report,
}

impl std::fmt::Display for Next {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Run(command) => write!(f, "run `{command}`"),
            Self::Fix(change) => f.write_str(change),
            Self::Report => write!(
                f,
                "this is a bug in {} {}; please report it, with the command line you ran: {}/issues/new",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
                env!("CARGO_PKG_REPOSITORY"),
            ),
        }
    }
}

/// An error that says what to do about it.
pub trait Remedy {
    /// What the user does next.
    fn next(&self) -> Next;
}

/// `--timings` could not install its subscriber: another was installed first, which no
/// command line the user writes can cause.
impl Remedy for tracing::subscriber::SetGlobalDefaultError {
    fn next(&self) -> Next {
        Next::Report
    }
}

/// A query the engine refused: wrong text is fixed in the text, a word for a kind in the word.
impl Remedy for ryo_core::inbound::use_cases::validate::errors::ValidateError {
    fn next(&self) -> Next {
        use ryo_core::inbound::use_cases::validate::errors::ValidateError;
        use ryo_values::domain::plan_parts::types::RefusedKind;
        match self {
            ValidateError::Refused { error } => match error.kind {
                RefusedKind::Syntax => Next::Fix("correct the query text at the byte the message names; `ryo operator list` shows the operations it may call".to_owned()),
                RefusedKind::UnknownRelation => Next::Run("ryo relation list".to_owned()),
                RefusedKind::Type | RefusedKind::Invalid => Next::Fix("correct the query text where the message says".to_owned()),
                RefusedKind::Catalog => Next::Report,
            },
            ValidateError::Uncompilable { .. } => Next::Fix("correct the query text where the message says".to_owned()),
            ValidateError::UnknownKind { .. } => Next::Fix("correct the kind the query text compares with".to_owned()),
        }
    }
}

impl Remedy for ryo_core::inbound::use_cases::lint::errors::LintError {
    fn next(&self) -> Next {
        use ryo_core::inbound::use_cases::lint::errors::LintError;
        match self {
            LintError::Query { .. } | LintError::UnknownKind { .. } | LintError::MissingParameter { .. } => Next::Fix("correct the query text; `ryo query validate --text <query>` checks it without running it".to_owned()),
            LintError::Corpus { .. } => Next::Fix("give `--root` a directory that exists and can be read".to_owned()),
            LintError::Memory { .. } | LintError::TooLarge { .. } | LintError::MemoryBudgetExceeded { .. } => Next::Fix("raise `--memory-budget`, or run on a smaller repository".to_owned()),
            LintError::Rule { .. } => Next::Fix("correct the arguments of the operation the message names; `ryo operator describe <name>` shows them".to_owned()),
            LintError::SourceLanguage { .. } | LintError::BlankRuleId | LintError::DuplicateRuleId { .. } | LintError::Store { .. } | LintError::Facts { .. } | LintError::Provider { .. } | LintError::Language { .. } | LintError::Scheduler { .. } => Next::Report,
            LintError::Output { .. } | LintError::OutputClosed => Next::Fix("give the command a stdout that takes its answer, or pass `--quiet`".to_owned()),
        }
    }
}

impl Remedy for ryo_core::inbound::use_cases::list_relations::errors::ListRelationsError {
    fn next(&self) -> Next {
        match self {
            Self::Language { .. } => Next::Report,
        }
    }
}

impl Remedy for ryo_core::inbound::use_cases::describe_relation::errors::DescribeRelationError {
    fn next(&self) -> Next {
        match self {
            Self::UnknownRelation { .. } => Next::Run("ryo relation list".to_owned()),
            Self::Language { .. } => Next::Report,
        }
    }
}

impl Remedy for ryo_core::inbound::use_cases::list_operators::errors::ListOperatorsError {
    fn next(&self) -> Next {
        match *self {}
    }
}

impl Remedy for ryo_core::inbound::use_cases::describe_operator::errors::DescribeOperatorError {
    fn next(&self) -> Next {
        match self {
            Self::UnknownOperator { .. } => Next::Run("ryo operator list".to_owned()),
        }
    }
}
