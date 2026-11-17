//! The `HeldRows` value.

use crate::domain::contract::types::WithheldInput;
use ryo_values::domain::rows::types::Table;

/// Rows the engine holds for a run, or the token of why it withholds them: a withheld input is a fault, never empty rows.
#[derive(Clone, Copy, Debug)]
pub enum HeldRows<'a> {
    /// The rows.
    Held(&'a Table),
    /// Withheld; the engine answers the fault carrying this token with its own reason.
    Withheld(WithheldInput),
}

impl<'a> HeldRows<'a> {
    /// The rows, or the engine's token.
    ///
    /// # Errors
    ///
    /// The token when the engine withheld them.
    pub const fn table(self) -> Result<&'a Table, WithheldInput> {
        match self {
            Self::Held(table) => Ok(table),
            Self::Withheld(token) => Err(token),
        }
    }
}
