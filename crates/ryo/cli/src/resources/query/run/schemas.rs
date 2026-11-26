//! The shape of the data this command does not control: its arguments become the service's request, and
//! the service's end becomes what the command reports. Each refusal of a conversion is a `<Name>SchemaError`.

/// Why the arguments were refused, or the queries could not be answered.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum RunSchemaError {
    /// A query text is empty.
    #[error("the query text is empty")]
    BlankText,
    /// The queries could not be answered: what is wrong, as the SDK says it.
    #[error("{0}")]
    Failed(ryo_sdk::query::RunError),
    /// Whoever reads the answer stopped reading, so it was not finished.
    #[error("whoever reads the answer stopped reading")]
    OutputClosed,
    /// The answer could not be written.
    #[error("output: {0}")]
    Output(String),
}

/// The SDK's request for the arguments: the root and the queries, one for each `--text`.
///
/// # Errors
///
/// `BlankText` for a query text that is empty or that is not RyoQL text at all.
pub(super) fn request(
    root: String,
    text: &[String],
    limit: u64,
) -> Result<ryo_sdk::query::RunRequest, RunSchemaError> {
    let mut queries = Vec::with_capacity(text.len());
    for each in text {
        if each.trim().is_empty() {
            return Err(RunSchemaError::BlankText);
        }
        queries.push(ryo_sdk::query::RyoQlQuery::try_from(each.as_str()).map_err(|_refused| RunSchemaError::BlankText)?);
    }
    Ok(ryo_sdk::query::RunRequest {
        root: root.into(),
        queries,
        limit: ryo_sdk::query::RowLimit(limit),
    })
}

/// The most memory the arguments let the run use, as the SDK takes it: none means the machine's.
pub(super) fn memory(bytes: u64) -> ryo_sdk::linter::MemoryBudget {
    std::num::NonZeroU64::new(bytes).map_or(
        ryo_sdk::linter::MemoryBudget::Machine,
        ryo_sdk::linter::MemoryBudget::Bytes,
    )
}

/// What a run says of the stream it writes its answer to refusing the answer: a reader that closed it, or any other failure of the
/// write.
pub(super) fn unwritten(error: &std::io::Error) -> RunSchemaError {
    if error.kind() == std::io::ErrorKind::BrokenPipe {
        RunSchemaError::OutputClosed
    } else {
        RunSchemaError::Output(error.to_string())
    }
}

/// How the run ended: a reader that closed the stream took what it wanted, so that end is not an error.
///
/// # Errors
///
/// The run's failure, unless it is that the output was closed.
pub(super) fn delivered(ended: Result<(), RunSchemaError>) -> Result<(), RunSchemaError> {
    match ended {
        Err(RunSchemaError::OutputClosed) => Ok(()),
        other => other,
    }
}

/// What the user does next about a refusal of the arguments.
impl crate::outcome::remedy::Remedy for RunSchemaError {
    fn next(&self) -> crate::outcome::remedy::Next {
        match self {
            Self::BlankText => {
                crate::outcome::remedy::Next::Fix("pass the query with `--text`".to_owned())
            }
            Self::Failed(error) => crate::outcome::remedy::Remedy::next(error),
            Self::OutputClosed | Self::Output(_) => crate::outcome::remedy::Next::Fix(
                "give the command a stdout that takes its answer, or pass `--quiet`".to_owned(),
            ),
        }
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
