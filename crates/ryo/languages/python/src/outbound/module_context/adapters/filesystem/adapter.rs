//! Disk package marker observations fed to the pure module context policy.
use crate::domain::module_context::{
    service,
    types::{Decision, Markers, ModuleContexts, ModuleIdentity},
};
use crate::outbound::module_context::interface::ModuleContextPort;
use ryo::languages::{FileId, FilePath, ProjectError, ProjectRequest, Repository, ToolMessage};
use std::{
    collections::BTreeSet,
    path::{Component, Path, PathBuf},
};
/// Filesystem-backed package context; Given requests use only their inventory.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ModuleContextFilesystem;
/// Preserve a context lookup failure as an owned diagnostic.
fn failed(reason: String) -> ProjectError {
    ProjectError::Context {
        reason: ToolMessage(reason.into_boxed_str()),
    }
}
/// Normalize an absolute path without accessing filesystem contents.
fn absolute(path: &Path) -> Result<PathBuf, ProjectError> {
    let path = std::path::absolute(path).map_err(|error| failed(error.to_string()))?;
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                let _removed = normalized.pop();
            }
            other @ (Component::Prefix(_) | Component::RootDir | Component::Normal(_)) => {
                normalized.push(other.as_os_str());
            }
        }
    }
    Ok(normalized)
}
impl ModuleContextPort for ModuleContextFilesystem {
    fn resolve(&self, request: &ProjectRequest) -> Result<ModuleContexts, ProjectError> {
        if !request.files.iter().any(|file| {
            matches!(
                Path::new(file.0.as_ref())
                    .extension()
                    .and_then(|extension| extension.to_str()),
                Some("py" | "pyi")
            )
        }) {
            return Ok(ModuleContexts::default());
        }
        let inventory: BTreeSet<_> = request
            .files
            .iter()
            .map(|file| PathBuf::from(file.0.as_ref()))
            .collect();
        let root = match request.repository {
            Repository::Disk => Some(absolute(Path::new(request.root.0.as_ref()))?),
            Repository::Given => None,
        };
        let mut markers = Markers::default();
        let mut result = ModuleContexts::default();
        for (number, file) in request.files.iter().enumerate() {
            let file_path = Path::new(file.0.as_ref());
            if !matches!(
                file_path
                    .extension()
                    .and_then(|extension| extension.to_str()),
                Some("py" | "pyi")
            ) {
                continue;
            }
            let file_id = FileId(u32::try_from(number).map_err(|error| failed(error.to_string()))?);
            let decision = if let Some(root) = &root {
                let full = absolute(&root.join(file_path))?;
                loop {
                    match service::disk(&full, &markers) {
                        Decision::Need(marker) => {
                            let present = match std::fs::metadata(&marker) {
                                Ok(metadata) => metadata.is_file(),
                                Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
                                Err(error) => {
                                    return Err(failed(format!(
                                        "cannot inspect {}: {error}",
                                        marker.display()
                                    )));
                                }
                            };
                            let relative =
                                service::relative_to(root, &marker).ok_or_else(|| {
                                    failed("module marker cannot be expressed from run root".into())
                                })?;
                            result.consulted.push(FilePath(
                                relative
                                    .to_str()
                                    .ok_or_else(|| {
                                        failed("module marker path is not UTF-8".into())
                                    })?
                                    .into(),
                            ));
                            let _previous = markers.0.insert(marker, present);
                        }
                        complete @ (Decision::Known(_) | Decision::Unknown) => break complete,
                    }
                }
            } else {
                service::given(file_path, &inventory)
            };
            match decision {
                Decision::Known(Some(module)) => {
                    let source = match &root {
                        Some(root) => absolute(&root.join(file_path))?,
                        None => file_path.to_path_buf(),
                    };
                    let (module_name, package_name) =
                        service::basenames(&source).ok_or_else(|| {
                            failed("known module lacks source/package basenames".into())
                        })?;
                    result.rows.push(ModuleIdentity {
                        file: file_id,
                        path: module.path,
                        module_name: module_name.to_owned(),
                        package_name: package_name.to_owned(),
                        package_root_name: module.package_root_name,
                    });
                }
                Decision::Known(None) => {}
                Decision::Unknown => result.partial = true,
                Decision::Need(_) => {
                    return Err(failed("unresolved module context observation".into()));
                }
            }
        }
        result.consulted.sort_by(|left, right| left.0.cmp(&right.0));
        result.consulted.dedup();
        Ok(result)
    }
}
