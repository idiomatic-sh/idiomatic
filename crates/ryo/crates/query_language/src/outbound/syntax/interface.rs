//! What the `syntax` port asks of the outside: RyoQL text in, its shape out; and the grammar it reads.

use super::errors::SyntaxError;
use ryo_values::domain::descriptions::types::Grammar;
use ryo_values::domain::program::types::{ParsedQueryBundle, Program, QueryBundle, QueryView};

/// What every `syntax` adapter implements.
pub trait SyntaxPort: Send + Sync {
    /// Reads RyoQL text.
    ///
    /// # Errors
    ///
    /// `Unreadable` with the place where the text stops making sense.
    fn read(&self, text: QueryView<'_>) -> Result<Program, SyntaxError>;

    /// Parses a bundle without resolving imports or expanding module ASTs.
    ///
    /// # Errors
    ///
    /// Returns the source-aware syntax error for the first unreadable document.
    fn read_bundle(&self, bundle: &QueryBundle) -> Result<ParsedQueryBundle, SyntaxError>;

    /// The grammar the adapter reads: every production, how it is written and text it reads.
    fn grammar(&self) -> Grammar;
}
