//! A borrowed artifact address whose components stay inside its selected root.
use crate::domain::artifacts::errors::ArtifactsError;
use std::path::{Component, Path};

/// A nonempty relative artifact path containing only normal components.
///
/// A caller cannot construct an unchecked artifact address.
/// ```compile_fail,E0423
/// use ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath;
/// let _ = RelativeArtifactPath(std::path::Path::new("../escape"));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RelativeArtifactPath<'a>(&'a Path);

/// The text of an address, its components joined with `/`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlashedPath(pub String);

impl<'a> TryFrom<&'a Path> for RelativeArtifactPath<'a> {
    type Error = ArtifactsError;

    fn try_from(path: &'a Path) -> Result<Self, Self::Error> {
        if path.as_os_str().is_empty()
            || path
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(ArtifactsError::Invalid(format!(
                "unsafe corpus path {}",
                path.display()
            )));
        }
        Ok(Self(path))
    }
}

impl AsRef<Path> for RelativeArtifactPath<'_> {
    fn as_ref(&self) -> &Path {
        self.0
    }
}

impl RelativeArtifactPath<'_> {
    /// The address as text, its components joined with `/` on every platform.
    ///
    /// This is the spelling a file keeps wherever it is persisted or compared across machines: the platform's own
    /// separator belongs to the machine, not to the address.
    ///
    /// # Errors
    /// Refuses a component that is not UTF-8.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath;
    /// use std::path::Path;
    ///
    /// let path = Path::new("pkg").join("module.py");
    /// let address = RelativeArtifactPath::try_from(path.as_path())?;
    /// assert_eq!(address.slashed()?.0, "pkg/module.py");
    /// # Ok::<(), ryo_benchmark_contracts::domain::artifacts::errors::ArtifactsError>(())
    /// ```
    pub fn slashed(&self) -> Result<SlashedPath, ArtifactsError> {
        let mut names = Vec::new();
        for part in self.0.components() {
            names.push(part.as_os_str().to_str().ok_or_else(|| {
                ArtifactsError::Invalid(format!("non-UTF8 corpus path {}", self.0.display()))
            })?);
        }
        Ok(SlashedPath(
            names.join(crate::domain::artifacts::constants::SLASH),
        ))
    }
}
