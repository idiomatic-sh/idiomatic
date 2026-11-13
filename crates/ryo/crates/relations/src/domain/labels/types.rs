//! The types of the `labels` concept: the names a relation is called and what it holds, borrowed or static text.

/// What a relation holds, in a sentence.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Holds(pub &'static str);

/// The name of a relation as a file system names it: the folder of its item, of its fixtures.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Folder(pub &'static str);

/// The name a relation or a column is called in the query language, borrowed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NameRef<'a>(pub &'a str);

impl std::ops::Deref for Folder {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl std::fmt::Display for Folder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl AsRef<std::path::Path> for Folder {
    fn as_ref(&self) -> &std::path::Path {
        std::path::Path::new(self.0)
    }
}

impl std::ops::Deref for Holds {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl std::fmt::Display for Holds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl std::ops::Deref for NameRef<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl std::fmt::Display for NameRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
