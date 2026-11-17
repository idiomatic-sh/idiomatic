//! The words a stamp names how a run is read with.

/// A SARIF run.
pub const SARIF: &[u8] = b"sarif";
/// An exit-status run.
pub const EXIT_STATUS: &[u8] = b"exit-status";
/// A Bencher Metric Format run.
pub const BMF: &[u8] = b"bmf";
/// What separates the two ends of a git range: `a..b`, and `a...b` once its third dot is taken off the right end.
pub const RANGE_DOTS: &str = "..";
/// The dot a symmetric range (`a...b`) has beyond the two of every range.
pub const SYMMETRIC_DOT: char = '.';
/// What an option joins to its value: `--rev-range=a..b`.
pub const OPTION_VALUE: char = '=';
/// The revision git reads an end of a range left out as.
pub const HEAD: &str = "HEAD";
/// What a shell command line puts around a word: quotes, substitutions, brackets and separators, which are not part of a revision.
pub const SHELL_PUNCTUATION: &[char] = &[
    '"', '\'', '$', '(', ')', '[', ']', '{', '}', ';', '&', '|', '`', ',', '<', '>',
];
/// The digest of a file that is absent.
pub const ABSENT: &str = "absent";
/// How long after a file's modification its record stays racy: a record taken sooner may have missed a write in the
/// same timestamp tick, the coarsest of which, on the filesystems asymptote runs on, is two seconds.
pub const RACY: super::types::Moment = super::types::Moment(2_000_000_000);
