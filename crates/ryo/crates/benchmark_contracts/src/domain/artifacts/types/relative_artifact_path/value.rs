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
