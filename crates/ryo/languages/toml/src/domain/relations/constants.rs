//! The text the `relations` concept's readers write when a relation is read only in part.

/// Why an entry's value is only in part: the file has no TOML value there.
pub(crate) const UNREADABLE_VALUE: &str = "the key has no value TOML reads: it is missing, or the library reads it only as a string it refuses";

/// Why a string has no value: the library refused its escapes, or its decoding is not at hand for this file.
pub(crate) const UNDECODED: &str =
    "the library refused the string's escapes, or did not decode this file in this worker";
