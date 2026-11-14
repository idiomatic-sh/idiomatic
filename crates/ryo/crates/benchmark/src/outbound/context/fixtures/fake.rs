//! The fake of the `context` port: what the core's tests stand in for it with. It seals in memory,
//! never touching the filesystem, and trusts the fingerprints a receipt carries.
use super::super::interface::ContextPort;
use super::super::types::CorpusFiles;
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::context::types::CaseContext;
use ryo_benchmark_contracts::domain::context::types::ContextBytes;
use ryo_benchmark_contracts::domain::context::types::ContextManifest;
use ryo_benchmark_contracts::domain::context::types::ProjectContextReceipt;
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

/// What the core's tests stand in for `context` with.
#[derive(Debug, Default)]
pub struct ContextFake;

/// Whether a path lies wholly inside a snapshot.
fn inside(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|part| matches!(part, Component::Normal(_) | Component::CurDir))
}

/// The identity of some bytes in the fake's own vocabulary.
fn seal(bytes: &ContextBytes) -> ArtifactDigest {
    ArtifactDigest(format!("{:?}", bytes.0))
}

/// Resolve a root-relative dependency lexically, never leaving the snapshot.
fn resolve(manifest: &ContextManifest, path: &Path) -> Result<PathBuf, Error> {
    let mut resolved = PathBuf::new();
    for part in manifest.logical_root.components() {
        if let Component::Normal(name) = part {
            resolved.push(name);
        }
    }
    for part in path.components() {
        match part {
            Component::Normal(name) => resolved.push(name),
            Component::CurDir => {}
            Component::ParentDir if resolved.pop() => {}
            Component::Prefix(_) | Component::RootDir | Component::ParentDir => {
                return Err(Error::Invalid(format!(
                    "context dependency escapes boundary: {}",
                    path.display()
                )));
            }
        }
    }
    Ok(resolved)
}

impl ContextPort for ContextFake {
    fn fingerprint(&self, manifest: &ContextManifest) -> ArtifactDigest {
        let mut missing = manifest.missing.clone();
        missing.sort();
        ArtifactDigest(format!(
            "{:?}",
            (
                &manifest.logical_root,
                &manifest.source_files,
                &manifest.auxiliary_files,
                missing
            )
        ))
    }

    fn validate(&self, manifest: &ContextManifest, actual_root: &Path) -> Result<(), Error> {
        let sealed = manifest
            .source_files
            .keys()
            .chain(manifest.auxiliary_files.keys())
            .chain(&manifest.missing);
        if !inside(&manifest.logical_root)
            || !manifest.boundary.is_absolute()
            || actual_root != manifest.boundary.join(&manifest.logical_root)
            || !sealed.into_iter().all(|path| inside(path))
        {
            return Err(Error::Invalid("unsafe context".into()));
        }
        Ok(())
    }

    fn seal_existing(
        &self,
        root: &Path,
        sources: &[ryo::linter::RelativePath],
        tracked: CorpusFiles<'_>,
    ) -> Result<ContextManifest, Error> {
        let source_set: BTreeSet<_> = sources
            .iter()
            .map(|path| PathBuf::from(path.0.as_ref()))
            .collect();
        let tracked_set: BTreeSet<_> = tracked.0.iter().cloned().collect();
        if source_set.len() != sources.len()
            || tracked_set.len() != tracked.0.len()
            || !source_set.is_subset(&tracked_set)
        {
            return Err(Error::Invalid("sealed corpus is not unique".into()));
        }
        let mut manifest = ContextManifest {
            boundary: std::path::absolute(root)?,
            logical_root: PathBuf::from(Component::CurDir.as_os_str()),
            auxiliary_files: BTreeMap::new(),
            source_files: BTreeMap::new(),
            missing: Vec::new(),
        };
        for path in tracked.0 {
            let identity = seal(&ContextBytes(path.as_os_str().as_encoded_bytes().to_vec()));
            let files = if source_set.contains(path) {
                &mut manifest.source_files
            } else {
                &mut manifest.auxiliary_files
            };
            let _prior = files.insert(path.clone(), identity);
        }
        self.validate(&manifest, root)?;
        Ok(manifest)
    }

    fn materialize(
        &self,
        boundary: &Path,
        context: &CaseContext,
        sources: &BTreeMap<ryo::linter::RelativePath, ryo::linter::SourceText>,
    ) -> Result<ContextManifest, Error> {
        let mut manifest = ContextManifest {
            boundary: std::path::absolute(boundary)?,
            logical_root: context.logical_root.clone(),
            auxiliary_files: context
                .auxiliary_files
                .iter()
                .map(|(path, bytes)| (path.clone(), seal(bytes)))
                .collect(),
            source_files: BTreeMap::new(),
            missing: context.missing.clone(),
        };
        for (path, source) in sources {
            let resolved = resolve(&manifest, Path::new(path.0.as_ref()))?;
            let _previous = manifest
                .source_files
                .insert(resolved, seal(&ContextBytes(source.0.as_bytes().to_vec())));
        }
        self.validate(&manifest, &manifest.boundary.join(&manifest.logical_root))?;
        Ok(manifest)
    }

    fn receipt(
        &self,
        manifest: &ContextManifest,
        actual_root: &Path,
        dependencies: Vec<ryo::linter::ProjectDependency>,
    ) -> Result<ProjectContextReceipt, Error> {
        let mut effective = manifest.clone();
        let mut observed = BTreeMap::new();
        for dependency in dependencies {
            let path = PathBuf::from(dependency.path.0.as_ref());
            let declared = resolve(manifest, &path)?;
            let sealed = manifest.auxiliary_files.contains_key(&declared)
                || manifest.source_files.contains_key(&declared);
            let state = match dependency.state {
                ryo::linter::ProjectDependencyState::Present(fingerprint) if sealed => {
                    Some(fingerprint)
                }
                ryo::linter::ProjectDependencyState::Missing if !sealed => {
                    if !effective.missing.contains(&declared) {
                        effective.missing.push(declared);
                    }
                    None
                }
                ryo::linter::ProjectDependencyState::Present(_)
                | ryo::linter::ProjectDependencyState::Missing => {
                    return Err(Error::Invalid(format!(
                        "context dependency contradicts the seal: {}",
                        path.display()
                    )));
                }
            };
            if observed.insert(path, state).is_some() {
                return Err(Error::Invalid("duplicate context dependency".into()));
            }
        }
        Ok(ProjectContextReceipt {
            mode: ProjectContextMode::ListedDisk,
            logical_root: manifest.logical_root.clone(),
            actual_root: actual_root.to_owned(),
            manifest_hash: self.fingerprint(&effective),
            dependencies: observed,
        })
    }

    fn verify_receipt(
        &self,
        manifest: &ContextManifest,
        actual_root: &Path,
        observed: &ProjectContextReceipt,
    ) -> Result<(), Error> {
        self.validate(manifest, actual_root)?;
        let dependencies = observed
            .dependencies
            .iter()
            .map(|(path, state)| ryo::linter::ProjectDependency {
                path: ryo::linter::RelativePath(
                    path.to_string_lossy().into_owned().into_boxed_str(),
                ),
                state: state.map_or(
                    ryo::linter::ProjectDependencyState::Missing,
                    ryo::linter::ProjectDependencyState::Present,
                ),
            })
            .collect();
        if self.receipt(manifest, actual_root, dependencies)? != *observed {
            return Err(Error::Invalid("context receipt identity disagrees".into()));
        }
        Ok(())
    }
    fn receipt_text(
        &self,
        receipt: &ProjectContextReceipt,
    ) -> Result<ryo_benchmark_contracts::domain::documents::types::text::value::CanonicalJson, Error>
    {
        Ok(
            ryo_benchmark_contracts::domain::documents::types::text::value::CanonicalJson(
                serde_json::Value::String(format!("{receipt:?}")).to_string(),
            ),
        )
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
