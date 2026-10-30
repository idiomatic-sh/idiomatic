//! The `tokens` service: free functions named by verb over the token kinds, with no IO and no ports.

#[cfg(any(test, feature = "fixtures"))]
use super::constants::STANDARD;
#[cfg(any(test, feature = "fixtures"))]
use ryo::languages::NameText;

/// The standard the token names come from, and its version.
#[cfg(any(test, feature = "fixtures"))]
#[must_use]
pub(crate) fn standard() -> NameText<'static> {
    NameText(STANDARD)
}
