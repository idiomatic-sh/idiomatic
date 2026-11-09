//! Module identity preserves raw package directories and strips only the source extension.
use super::constants;
use super::types::{
    Basenames, Markers, ModuleDecision, ModuleDottedPath, PackageModule, PackageRootName,
};
use ryo::languages::NameText;
use std::{
    collections::BTreeSet,
    path::{Component, Path, PathBuf},
};

/// Derive a dotted identity only within a fully observed package ancestry.
fn module_path(file: &Path, package: Option<&Path>) -> ModuleDecision {
    let Some(package) = package else {
        return ModuleDecision::Known(None);
    };
    let Some(parent) = package.parent() else {
        return ModuleDecision::Unknown;
    };
    let Ok(relative) = file.strip_prefix(parent) else {
        return ModuleDecision::Unknown;
    };
    let mut parts = Vec::new();
    for component in relative.parent().into_iter().flat_map(Path::components) {
        let Component::Normal(component) = component else {
            return ModuleDecision::Unknown;
        };
        let Some(directory) = component.to_str() else {
            return ModuleDecision::Unknown;
        };
        parts.push(directory);
    }
    let Some(names) = basenames(file) else {
        return ModuleDecision::Unknown;
    };
    parts.push(names.module.0);
    let Some(root_name) = package.file_name().and_then(|name| name.to_str()) else {
        return ModuleDecision::Unknown;
    };
    ModuleDecision::Known(Some(PackageModule {
        path: ModuleDottedPath(parts.join(constants::DOT)),
        package_root_name: PackageRootName(root_name.to_owned()),
    }))
}

/// Raw names are separate from dotted qualified identity; no absolute path is exposed.
pub(crate) fn basenames(file: &Path) -> Option<Basenames<'_>> {
    let name = file.file_name()?.to_str()?;
    let module = name
        .strip_suffix(constants::STUB_SUFFIX)
        .or_else(|| name.strip_suffix(constants::MODULE_SUFFIX))?;
    let package = file.parent()?.file_name()?.to_str()?;
    Some(Basenames {
        module: NameText(module),
        package: NameText(package),
    })
}

/// Resolve an absolute disk source using only supplied marker observations.
pub(crate) fn disk(file: &Path, markers: &Markers) -> ModuleDecision {
    let Some(mut directory) = file.parent() else {
        return ModuleDecision::Unknown;
    };
    let mut package = None;
    loop {
        let marker = directory.join(constants::INIT_FILE);
        match markers.0.get(&marker) {
            None => return ModuleDecision::Need(marker),
            Some(false) => return module_path(file, package),
            Some(true) => package = Some(directory),
        }
        let Some(parent) = directory.parent() else {
            return ModuleDecision::Unknown;
        };
        directory = parent;
    }
}

/// Given inventories never name their root directory and never permit external lookup.
pub(crate) fn given(file: &Path, files: &BTreeSet<PathBuf>) -> ModuleDecision {
    let Some(mut directory) = file.parent() else {
        return ModuleDecision::Unknown;
    };
    let mut package = None;
    loop {
        if !files.contains(&directory.join(constants::INIT_FILE)) {
            return module_path(file, package);
        }
        if directory.as_os_str().is_empty() {
            return ModuleDecision::Unknown;
        }
        package = Some(directory);
        let Some(parent) = directory.parent() else {
            return ModuleDecision::Unknown;
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
        result.push(constants::PARENT_DIRECTORY);
    }
    for part in path.iter().skip(common) {
        result.push(part.as_os_str());
    }
    Some(result)
}
