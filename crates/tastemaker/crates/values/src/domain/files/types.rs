//! The types of the `files` concept: plain data, public fields and derives, with no constructor and no inherent impl. A
//! place on the machine, the text of a file, and what its size and modification time come to.

/// A directory on the machine, as text.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MachinePath(pub Box<str>);

/// A directory name a walk of the repository never goes into, at any depth, and who declares it: a linked language, or Tastemaker.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SkippedDirectory {
    /// The directory's name, as `node_modules`.
    pub name: DirectoryName,
    /// Those who declare it, each a language by its name.
    pub by: Vec<Declarer>,
}

/// The directories a walk never goes into, sorted by name.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SkippedDirectories(pub Vec<SkippedDirectory>);

/// A directory's name, as `node_modules`.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DirectoryName(pub Box<str>);

/// Who declares a directory skipped: a linked language by its name, or Tastemaker.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Declarer(pub Box<str>);

/// The text of a file.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileText(pub Box<str>);

/// A file's size and modification time: a file whose stamp is unchanged is taken to be unchanged. A file that is not
/// there has the default stamp.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Stamp {
    /// Its size.
    pub size: FileSize,
    /// When it was last modified.
    pub modified: ModifiedTime,
}

/// A file's size, in bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileSize(pub u64);

/// When a file was last modified, in nanoseconds since the Unix epoch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModifiedTime(pub u64);

/// The stamps of files, in the order they were asked for.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Stamps(pub Vec<Stamp>);

impl std::ops::Deref for Stamps {
    type Target = [Stamp];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for Stamps {
    type Item = Stamp;
    type IntoIter = std::vec::IntoIter<Stamp>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Stamps {
    type Item = &'a Stamp;
    type IntoIter = std::slice::Iter<'a, Stamp>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

/// File text borrowed from an immutable source inventory.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BorrowedText<'a>(pub &'a str);

impl std::ops::Deref for BorrowedText<'_> {
    type Target = str;
    fn deref(&self) -> &str {
        self.0
    }
}

/// Line-start byte offsets borrowed from the inventory's storage column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineStarts<'a>(pub &'a [u64]);

impl std::ops::Deref for LineStarts<'_> {
    type Target = [u64];
    fn deref(&self) -> &[u64] {
        self.0
    }
}

/// A source's immutable storage, borrowed by every reporting stage.
#[derive(Clone, Copy, Debug)]
pub struct SourceView<'a> {
    /// The exact UTF-8 source, including its line endings.
    pub text: BorrowedText<'a>,
    /// Every line's starting byte, including the empty last line after a newline.
    pub starts: LineStarts<'a>,
}
