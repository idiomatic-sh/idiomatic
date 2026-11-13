//! The names of the `file_program` concept: the text of its diagnostics and of what it asserts.

/// The tracing target of the request preparation diagnostics.
pub(crate) const PREPARATION_TARGET: &str = "ryo::diagnostic::preparation";

/// The event of an ordered-existence alternative declined after canonical producer binding.
pub(crate) const ORDERED_EXISTENCE_DECLINED: &str = "ordered_existence_declined";

/// What a prepared request asserts when the binding slots of its rules overflow the address space.
pub(crate) const EXPECT_BINDING_SLOTS: &str = "request binding slots fit address space";

/// What a prepared request asserts when a compiled binding index does not fit its id.
pub(crate) const EXPECT_BINDING_INDEX: &str = "compiled binding index fits u32";

/// What a prepared request asserts when a compiled output index does not fit its id.
pub(crate) const EXPECT_OUTPUT_INDEX: &str = "compiled output index fits u32";
