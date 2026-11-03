//! Module identity preserves raw package directories and strips only the source extension.
use super::types::{Decision, Markers, PackageModule};
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
    for component in relative.parent().into_iter().flat_map(Path::components) {
        let Component::Normal(component) = component else {
            return Decision::Unknown;
        };
        let Some(directory) = component.to_str() else {
            return Decision::Unknown;
        };
        parts.push(directory);
    }
    let Some((module, _package)) = basenames(file) else {
        return Decision::Unknown;
    };
    parts.push(module);
    let Some(root_name) = package.file_name().and_then(|name| name.to_str()) else {
        return Decision::Unknown;
    };
    Decision::Known(Some(PackageModule {
        path: parts.join("."),
        package_root_name: root_name.to_owned(),
    }))
}

/// Raw names are separate from dotted qualified identity; no absolute path is exposed.
pub(crate) fn basenames(file: &Path) -> Option<(&str, &str)> {
    let name = file.file_name()?.to_str()?;
    let module = name
        .strip_suffix(".pyi")
        .or_else(|| name.strip_suffix(".py"))?;
    let package = file.parent()?.file_name()?.to_str()?;
    Some((module, package))
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
