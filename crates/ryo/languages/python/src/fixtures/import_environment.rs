//! Package observations for Python file relations. A missing marker and an
//! unobserved entry have different effects on a negative import lookup.
use ryo::languages::{FileProject, ObservedEntryState as State};
use std::path::{Path, PathBuf};

fn observe(project: &dyn FileProject, path: &Path) -> Result<State, String> {
    let state = project.entry(path);
    if state == State::Unknown {
        Err(format!("unobserved project entry {}", path.display()))
    } else {
        Ok(state)
    }
}

/// Containing dotted package for relative resolution, and the regular package root.
pub(crate) fn package(
    project: &dyn FileProject,
    path: &str,
) -> Result<(Option<String>, String), String> {
    let path = Path::new(path);
    let mut components = Vec::new();
    let mut directory = path.parent();
    while let Some(parent) = directory {
        let Some(name) = parent.file_name().and_then(|n| n.to_str()) else {
            break;
        };
        if observe(project, &parent.join("__init__.py"))? != State::File {
            break;
        }
        components.push(name);
        directory = parent.parent();
    }
    components.reverse();
    let Some(&root) = components.first() else {
        return Ok((None, String::new()));
    };
    let Some(stem) = path.file_name().and_then(|f| f.to_str()).and_then(|name| {
        name.strip_suffix(".pyi")
            .or_else(|| name.strip_suffix(".py"))
    }) else {
        return Ok((None, String::new()));
    };
    if let Some((parent, _)) = stem.rsplit_once('.') {
        components.push(parent);
    }
    Ok((Some(components.join(".")), root.to_owned()))
}

/// Longest module and regular-package prefixes and final namespace membership.
pub(crate) fn prefixes(
    project: &dyn FileProject,
    target: &str,
) -> Result<(String, String, bool), String> {
    let roots = project
        .roots()
        .ok_or("project search roots were not declared")?;
    let mut module = String::new();
    let mut package = String::new();
    let mut namespace = false;
    let mut path = PathBuf::new();
    for end in target
        .match_indices('.')
        .map(|(i, _)| i)
        .chain(std::iter::once(target.len()))
    {
        let prefix = &target[..end];
        let component = prefix.rsplit('.').next().unwrap_or(prefix);
        // Module components come from the parser. A candidate may never escape the repository.
        if !matches!(
            Path::new(component)
                .components()
                .collect::<Vec<_>>()
                .as_slice(),
            [std::path::Component::Normal(_)]
        ) {
            return Err(format!("invalid import path component {component}"));
        }
        path.push(component);
        namespace = false;
        for root in roots {
            let base = Path::new(root).join(&path);
            if observe(project, &base.with_extension("py"))? == State::File
                || observe(project, &base.with_extension("pyi"))? == State::File
            {
                module = prefix.to_owned();
            }
            if observe(project, &base)? == State::Directory {
                if observe(project, &base.join("__init__.py"))? == State::File
                    || observe(project, &base.join("__init__.pyi"))? == State::File
                {
                    package = prefix.to_owned();
                } else {
                    namespace = true;
                }
            }
        }
    }
    Ok((module, package, namespace))
}
