//! `query validate`: composes, calls the service once.

use ryo_core::inbound::use_cases::validate::interface::ValidateUseCase as _;

use super::schemas;
use super::types::{ValidateReport, ValidateRequest};
use crate::domain::language::types::Languages;
use crate::domain::query::errors::ValidateError;

/// `query validate`: checks that RyoQL text is correct, and that every word it compares the kind of a node or a token with names a
/// kind of one of the linked `languages`, as a run checks it before it reads a file. With no language linked there is no kind to check
/// against, and the words are not asked.
///
/// # Errors
///
/// `Syntax`, `UnknownRelation`, `Type` or `Invalid`, with the place where the text is wrong; `Invalid` for a word that names no kind.
pub fn validate(
    languages: &Languages,
    request: ValidateRequest,
) -> Result<ValidateReport, ValidateError> {
    let text = request.query.to_string();
    super::composition::build(languages.0.clone())
        .validate(schemas::request(request))
        .map(schemas::report)
        .map_err(|error| schemas::error(&text, error))
}
