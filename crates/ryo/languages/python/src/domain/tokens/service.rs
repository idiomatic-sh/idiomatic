//! The `tokens` service: free functions named by verb over the token kinds, with no IO and no ports.

use super::constants::STANDARD;
use ryo::languages::NameText;

/// The standard the token names come from, and its version.
#[must_use]
pub(crate) fn standard() -> NameText<'static> {
    NameText(STANDARD)
}
