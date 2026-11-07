//! The types of the `bases` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// What the targets of a document are resolved against: the file's own path, and the `<base href>` the document names, if it names one.
#[derive(Clone, Debug)]
pub(crate) struct Targeting<'a> {
    /// The file's own path: a fragment alone and the `<base>` element's own `href` are resolved against it.
    pub file: ryo::languages::SourcePath<'a>,
    /// The document's base.
    pub base: Base,
}

/// The base a document names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Base {
    /// None: every target is resolved against the file.
    Absent,
    /// A path of the run, in the directory of the base, which relative targets are resolved against.
    Inside(BasePath),
    /// Something outside the run, which every relative target then leaves the run through.
    Outside,
}

/// A path of the run in the directory of the document's base.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BasePath(pub String);

/// Whose target is resolved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Whose {
    /// The `<base>` element's own `href`: resolved against the file, as the document's base is not yet known.
    Base,
    /// Any other URL of the document.
    Other,
}
