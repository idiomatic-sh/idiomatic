//! The types of the `physical` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use super::super::types::{OrderedExistence, RegexIdentity};

/// How the physical kernel of an instruction is reached.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::physical::types::PhysicalRoute;
/// fn named(_: Option<&PhysicalRoute<'_>>) {}
/// ```
#[derive(Debug)]
pub enum PhysicalRoute<'a> {
    /// Through the registered operator.
    Registered,
    /// Through an ordered existence.
    Ordered(&'a OrderedExistence),
}

/// What a physical evaluation is bound to: the request's parameters, the regular expression's identity and the source's length.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::physical::types::PhysicalBinding;
/// fn named(_: Option<&PhysicalBinding<'_>>) {}
/// ```
#[derive(Debug)]
pub struct PhysicalBinding<'a> {
    /// The request's parameter bindings.
    pub bindings: &'a [ryo_values::domain::query_bindings::types::ParameterBinding],
    /// The identity of the compiled regular expression, when the operation matches one.
    pub regex_identity: Option<RegexIdentity>,
    /// The length of the source in bytes.
    pub source_len: ryo_operators::domain::contract::types::SourceByteLength,
}

/// The proof that one operator call is the compiled meaning of its instruction; only
/// [`authorize`](super::service::authorize) makes one: no caller outside the part can build it.
///
/// ```compile_fail
/// let _forged = ryo_execution_domain::domain::executable_program::physical::types::PhysicalAuthorization {
///     _sealed: std::marker::PhantomData,
/// };
/// ```
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::physical::types::PhysicalAuthorization;
/// fn named(_: Option<&PhysicalAuthorization>) {}
/// ```
#[derive(Debug)]
pub struct PhysicalAuthorization {
    pub(super) _sealed: std::marker::PhantomData<()>,
}
