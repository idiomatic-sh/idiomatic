//! Disk package marker observations fed to the pure module context policy.
use super::constants::{
    KNOWN_MODULE_LACKS_SOURCE_PACKAGE_BASENAMES, MODULE_MARKER_CANNOT_BE_EXPRESSED_FROM_RUN_ROOT,
    MODULE_MARKER_PATH_IS_NOT_UTF_8, PYTHON_EXTENSION, STUB_EXTENSION,
    UNRESOLVED_MODULE_CONTEXT_OBSERVATION,
};
use crate::domain::module_context::{
    service,
    types::{
        Markers, ModuleContexts, ModuleDecision, ModuleIdentity, ModuleName, PackageDirectoryName,
        PartialContext,
    },
};
use crate::outbound::module_context::interface::ModuleContextPort;
use ryo::languages::{FileId, FilePath, ProjectError, ProjectRequest, Repository, ToolMessage};
use std::{
    collections::BTreeSet,
    path::{Component, Path, PathBuf},
};
/// Filesystem-backed package context; Given requests use only their inventory.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ModuleContextFilesystemAdapter;
/// Preserve a context lookup failure as an owned diagnostic.
fn failed(reason: ToolMessage) -> ProjectError {
    ProjectError::Context { reason }
}
/// Normalize an absolute path without accessing filesystem contents.
fn absolute(path: &Path) -> Result<PathBuf, ProjectError> {
    let path = std::path::absolute(path).map_err(|error| failed(ToolMessage(error.to_string().into_boxed_str())))?;
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
impl ModuleContextPort for ModuleContextFilesystemAdapter {
    fn resolve(&self, request: &ProjectRequest) -> Result<ModuleContexts, ProjectError> {
        if !request.files.iter().any(|file| {
            matches!(
                Path::new(file.0.as_ref())
                    .extension()
                    .and_then(|extension| extension.to_str()),
                Some(PYTHON_EXTENSION | STUB_EXTENSION)
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
                Some(PYTHON_EXTENSION | STUB_EXTENSION)
            ) {
                continue;
            }
            let file_id = FileId(u32::try_from(number).map_err(|error| failed(ToolMessage(error.to_string().into_boxed_str())))?);
            let decision = if let Some(root) = &root {
                let full = absolute(&root.join(file_path))?;
                loop {
                    match service::disk(&full, &markers) {
                        ModuleDecision::Need(marker) => {
                            let present = match std::fs::metadata(&marker) {
                                Ok(metadata) => metadata.is_file(),
                                Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
                                Err(error) => {
                                    return Err(failed(ToolMessage(
                                        format!("cannot inspect {}: {error}", marker.display())
                                            .into_boxed_str(),
                                    )));
                                }
                            };
                            let relative =
                                service::relative_to(root, &marker).ok_or_else(|| {
                                    failed(ToolMessage(MODULE_MARKER_CANNOT_BE_EXPRESSED_FROM_RUN_ROOT.into()))
                                })?;
                            result.consulted.push(FilePath(
                                relative
                                    .to_str()
                                    .ok_or_else(|| failed(ToolMessage(MODULE_MARKER_PATH_IS_NOT_UTF_8.into())))?
                                    .into(),
                            ));
                            let _previous = markers.0.insert(marker, present);
                        }
                        complete @ (ModuleDecision::Known(_) | ModuleDecision::Unknown) => {
                            break complete;
                        }
                    }
                }
            } else {
                service::given(file_path, &inventory)
            };
            match decision {
                ModuleDecision::Known(Some(module)) => {
                    let source = match &root {
                        Some(root) => absolute(&root.join(file_path))?,
                        None => file_path.to_path_buf(),
                    };
                    let names =
                        service::basenames(&source).ok_or_else(|| {
                            failed(ToolMessage(KNOWN_MODULE_LACKS_SOURCE_PACKAGE_BASENAMES.into()))
                        })?;
                    result.rows.push(ModuleIdentity {
                        file: file_id,
                        path: module.path,
                        module_name: ModuleName(names.module.0.to_owned()),
                        package_name: PackageDirectoryName(names.package.0.to_owned()),
                        package_root_name: module.package_root_name,
                    });
                }
                ModuleDecision::Known(None) => {}
                ModuleDecision::Unknown => result.partial = PartialContext(true),
                ModuleDecision::Need(_) => {
                    return Err(failed(ToolMessage(UNRESOLVED_MODULE_CONTEXT_OBSERVATION.into())));
                }
            }
        }
        result.consulted.sort_by(|left, right| left.0.cmp(&right.0));
        result.consulted.dedup();
        Ok(result)
    }
}
