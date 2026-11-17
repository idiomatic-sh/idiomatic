//! The `operating` service: free functions named by verb over what an operator reads, with no IO and no ports.

use crate::domain::operating::errors::OperatingError;

/// A name the steps made, as a value: when the steps run after the files were read (the text of the workers is
/// given) it is a name that has left its file, which compares with the names the files handed over; before, a name of the file.
#[must_use]
pub fn minted(
    symbols: &ryo_language::domain::symbols::types::Symbols,
    symbol: ryo_relations::domain::terms::types::Symbol,
    after_the_files: super::types::AfterTheFiles,
) -> ryo_relations::domain::relations::types::Value {
    if after_the_files == super::types::AfterTheFiles::After
        && let Some(name) =
            ryo_language::domain::symbols::service::name_made_across(symbols, symbol)
    {
        return ryo_relations::domain::relations::types::Value::Name(name);
    }
    ryo_relations::domain::relations::types::Value::Symbol(symbol)
}

/// The operating error a portable operator fault stands for.
pub fn portable_fault(
    error: ryo_operators::domain::contract::types::PortableOperatorFault,
) -> OperatingError {
    use ryo_operators::domain::contract::types::PortableOperatorFault;
    match error {
        PortableOperatorFault::Interrupted => OperatingError::WrongOperation,
        PortableOperatorFault::NoSuchColumn { index } => {
            ryo_values::domain::rows::errors::RowsError::NoSuchColumn { index: index.0 }.into()
        }
        PortableOperatorFault::ShapeMismatch => OperatingError::ShapeMismatch,
        PortableOperatorFault::DuplicateOrder => OperatingError::DuplicateDfsOrder,
        PortableOperatorFault::NotALattice => OperatingError::NotALattice,
        PortableOperatorFault::InvalidSpanBounds => OperatingError::InvalidSpanBounds,
        PortableOperatorFault::InvalidSpan => OperatingError::InvalidSpan,
        PortableOperatorFault::InvalidSourceMatchSpan => OperatingError::InvalidSourceMatchSpan,
        PortableOperatorFault::UnboundKey => OperatingError::WrongBinding,
        PortableOperatorFault::WrongBinding => OperatingError::WrongBinding,
        PortableOperatorFault::Unbound => OperatingError::Unbound,
        // The engine answers a withheld input with its own reason before it asks this: a token alone names no reason.
        PortableOperatorFault::NotRun | PortableOperatorFault::Withheld(_) => {
            OperatingError::NotRun
        }
        PortableOperatorFault::WrongTest => OperatingError::WrongTest,
        PortableOperatorFault::Superlinear(refused) => {
            OperatingError::Superlinear { node: refused.0 }
        }
        PortableOperatorFault::Pattern(
            ryo_matching::inbound::use_cases::matching::errors::MatchingError::InvalidRegex {
                reason,
            }
            | ryo_matching::inbound::use_cases::matching::errors::MatchingError::BadGlob {
                reason,
                ..
            },
        ) => OperatingError::InvalidRegex { reason },
        PortableOperatorFault::MissingCapture { capture } => OperatingError::InvalidRegex {
            reason: format!("missing named capture `{}`", capture.0),
        },
        PortableOperatorFault::Derived(fault) => derived_fault(fault),
        // A refused rendering is answered from the engine's record of it; one no record holds is a wrong operation.
        PortableOperatorFault::WrongOperation
        | PortableOperatorFault::UnsupportedRender
        | PortableOperatorFault::RenderRefused { .. } => OperatingError::WrongOperation,
    }
}

/// The operating error a derived fault stands for.
pub fn derived_fault(fault: ryo_derived::domain::fault::types::DerivedFault) -> OperatingError {
    use ryo_derived::domain::fault::types::DerivedFault;
    match fault {
        DerivedFault::MissingInput { position } => OperatingError::MissingInput {
            position: position.0,
        },
        DerivedFault::NoSuchColumn { column } => {
            ryo_values::domain::rows::errors::RowsError::NoSuchColumn { index: column.0 }.into()
        }
        DerivedFault::DuplicateOrder => OperatingError::DuplicateOrder,
        DerivedFault::DuplicateDfsOrder => OperatingError::DuplicateDfsOrder,
        DerivedFault::ShapeMismatch => OperatingError::ShapeMismatch,
        DerivedFault::NotRun => OperatingError::NotRun,
        DerivedFault::WrongOperation | DerivedFault::Interrupted => OperatingError::WrongOperation,
    }
}

/// What the file's semantic session renders for `requests`, one answer each in request order: the engine's one call of the
/// session, which the interpreter and the file pass both make.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::operating::errors::OperatingError;
/// use ryo_execution_domain::domain::operating::service::rendered_batch;
/// use ryo_values::domain::files::types::SourceView;
/// assert!(matches!(
///     rendered_batch(None, &[], SourceView(b"")),
///     Err(OperatingError::SemanticIncomplete { .. })
/// ));
/// ```
///
/// # Errors
///
/// [`OperatingError::SemanticIncomplete`] when the file admitted no semantic session, the session's own refusal, and
/// [`OperatingError::SemanticBatchShape`] for a batch of answers of another length than the requests'.
pub fn rendered_batch(
    semantic: Option<&dyn crate::domain::operating::types::SemanticLookup>,
    requests: &[ryo_language::domain::semantics::types::RenderRequest],
    source: ryo_values::domain::files::types::SourceView<'_>,
) -> Result<Vec<ryo_language::domain::semantics::types::RenderOutcome>, OperatingError> {
    let lookup = semantic.ok_or_else(|| OperatingError::SemanticIncomplete {
        reason: crate::domain::operating::constants::NO_SEMANTIC_SESSION.into(),
    })?;
    let answers = lookup.render_batch(requests, source)?;
    if answers.len() == requests.len() {
        Ok(answers)
    } else {
        Err(OperatingError::SemanticBatchShape)
    }
}

/// A provider's unsupported answer invalidates the consuming rule, with the provider's reason: the first unsupported answer of
/// the batch.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::operating::errors::OperatingError;
/// use ryo_execution_domain::domain::operating::service::unsupported_render;
/// assert!(matches!(unsupported_render(&[]), OperatingError::SemanticIncomplete { .. }));
/// ```
#[must_use]
pub fn unsupported_render(
    answers: &[ryo_language::domain::semantics::types::RenderOutcome],
) -> OperatingError {
    use ryo_language::domain::semantics::types::RenderOutcome;
    let reason = answers.iter().find_map(|answer| match answer {
        RenderOutcome::Unsupported(reason) => Some(reason.0),
        RenderOutcome::Known(_) | RenderOutcome::NotApplicable => None,
    });
    OperatingError::SemanticIncomplete {
        reason: format!(
            "selected language cannot render this syntax (reason {})",
            reason.map_or_else(String::new, |reason| reason.to_string())
        ),
    }
}
