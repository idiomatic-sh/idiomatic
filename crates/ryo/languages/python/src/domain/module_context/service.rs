//! Module identity follows contiguous `__init__.py` ancestry and component file stems.
use super::types::{Decision, Markers};
use std::{
    collections::BTreeSet,
    path::{Component, Path, PathBuf},
};

/// Derive a dotted identity only within a fully observed package ancestry.
fn module_path(file: &Path, package: Option<&Path>) -> Decision {
    let Some(package) = package else {
        return Decision::Known(None);
    };
    let Some(parent) = package.parent() else {
        return Decision::Unknown;
    };
    let Ok(relative) = file.strip_prefix(parent) else {
        return Decision::Unknown;
    };
    let mut parts = Vec::new();
    for component in relative.components() {
        let Component::Normal(component) = component else {
            return Decision::Unknown;
        };
        let Some(stem) = Path::new(component)
            .file_stem()
            .and_then(|stem| stem.to_str())
        else {
            return Decision::Unknown;
        };
        parts.push(stem);
    }
    Decision::Known(Some(parts.join(".")))
}

/// Resolve an absolute disk source using only supplied marker observations.
pub(crate) fn disk(file: &Path, markers: &Markers) -> Decision {
    let Some(mut directory) = file.parent() else {
        return Decision::Unknown;
    };
    let mut package = None;
    loop {
        let marker = directory.join("__init__.py");
        match markers.0.get(&marker) {
            None => return Decision::Need(marker),
            Some(false) => return module_path(file, package),
            Some(true) => package = Some(directory),
        }
        let Some(parent) = directory.parent() else {
            return Decision::Unknown;
        };
        directory = parent;
    }
}

/// Given inventories never name their root directory and never permit external lookup.
pub(crate) fn given(file: &Path, files: &BTreeSet<PathBuf>) -> Decision {
    let Some(mut directory) = file.parent() else {
        return Decision::Unknown;
    };
    let mut package = None;
    loop {
        if !files.contains(&directory.join("__init__.py")) {
            return module_path(file, package);
        }
        if directory.as_os_str().is_empty() {
            return Decision::Unknown;
        }
        package = Some(directory);
        let Some(parent) = directory.parent() else {
            return Decision::Unknown;
        };
        directory = parent;
    }
}

/// A normalized absolute path expressed from the normalized absolute run root.
pub(crate) fn relative_to(root: &Path, path: &Path) -> Option<PathBuf> {
    let root: Vec<_> = root.components().collect();
    let path: Vec<_> = path.components().collect();
    if !matches!(
        root.first(),
        Some(Component::RootDir | Component::Prefix(_))
    ) || root.first() != path.first()
    {
        return None;
    }
    let common = root
        .iter()
        .zip(&path)
        .take_while(|(left, right)| left == right)
        .count();
    let mut result = PathBuf::new();
    for _ in common..root.len() {
        result.push("..");
    }
    for part in path.iter().skip(common) {
        result.push(part.as_os_str());
    }
    Some(result)
}
