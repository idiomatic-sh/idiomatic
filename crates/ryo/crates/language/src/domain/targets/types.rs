//! The types of the `targets` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// A target as the file writes it, after its library decoded it: a path, a fragment, a query or a URL.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WrittenTarget<'a>(pub &'a str);

/// What a target names: a path of the run, or the target as written when it names something outside the run.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResolvedTarget(pub String);

/// Whether a target names something in the run or outside it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetReach {
    /// A path of the run, or a place in the file that writes it.
    Inside,
    /// Something with a scheme or a host, which stays as written.
    Outside,
}

/// Whether a target opens with a scheme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scheme {
    /// It does: it names something outside the run.
    Present,
    /// It does not.
    Absent,
}

/// A path, or the part of one still to be read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PathPart<'a>(pub &'a str);
