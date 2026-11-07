//! The fake of the `packages` port: one package held in memory, its files by path.

use std::collections::BTreeMap;

use crate::domain::files::types::{FileText, Stamp};
use crate::domain::packages::types::{Dependency, PackageRoot, SourcePath, VersionText};
use crate::outbound::packages::errors::PackagesError;
use crate::outbound::packages::interface::PackagesPort;
use crate::outbound::packages::types::{Located, PackageFile};

/// The files of packages, by package directory and path.
type FakeFiles = BTreeMap<(Box<str>, Box<str>), Box<str>>;

/// What the core's tests stand in for `packages` with: every package at `/<name>`, its files by path.
#[derive(Debug, Default)]
pub struct PackagesFake {
    /// The files, by package directory and path.
    pub files: FakeFiles,
    /// Every file read, `<directory>/<path>`, in order: shared with the test that keeps a clone.
    pub read: std::sync::Arc<std::sync::Mutex<Vec<Box<str>>>>,
    /// Every listing of a definition's versions asked for, `<directory>/<path>`, in order: shared with the test that keeps a clone.
    pub listed: std::sync::Arc<std::sync::Mutex<Vec<Box<str>>>>,
    /// The files of the packages the run made and held, by package directory and path.
    pub held: std::sync::Mutex<FakeFiles>,
}

impl PackagesFake {
    /// Every file: the ones the test gave and the ones the run held.
    fn every(&self) -> FakeFiles {
        let mut every = self.files.clone();
        every.extend(
            self.held
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .iter()
                .map(|(key, text)| (key.clone(), text.clone())),
        );
        every
    }

    /// One package `name` holding `files`.
    #[must_use]
    pub fn of(name: &str, files: &[(&str, &str)]) -> Self {
        Self {
            files: files
                .iter()
                .map(|(path, text)| ((format!("/{name}").into(), (*path).into()), (*text).into()))
                .collect(),
            read: std::sync::Arc::default(),
            listed: std::sync::Arc::default(),
            held: std::sync::Mutex::default(),
        }
    }
}

impl PackagesPort for PackagesFake {
    fn locate(
        &self,
        dependency: &Dependency,
        from: &PackageRoot,
        _: &SourcePath,
    ) -> Result<Located, PackagesError> {
        // A package the run held is found at the directory its path names, one beside the file that names it at its path
        // from there, as a package in the repository is; any other at `/<name>`.
        let every = self.every();
        let beside: Box<str> = match &dependency.source {
            crate::domain::packages::types::PackageSource::Path(path) => {
                format!("{}/{}", from.0, path.0).into()
            }
            crate::domain::packages::types::PackageSource::Git(_)
            | crate::domain::packages::types::PackageSource::Registry => Box::default(),
        };
        let directory: Box<str> = match &dependency.source {
            crate::domain::packages::types::PackageSource::Path(path)
                if every.keys().any(|(package, _)| *package == path.0) =>
            {
                path.0.clone()
            }
            crate::domain::packages::types::PackageSource::Path(_)
                if every.keys().any(|(package, _)| *package == beside) =>
            {
                beside
            }
            crate::domain::packages::types::PackageSource::Path(_)
            | crate::domain::packages::types::PackageSource::Git(_)
            | crate::domain::packages::types::PackageSource::Registry => {
                format!("/{}", dependency.package.0).into()
            }
        };
        if !every.keys().any(|(package, _)| *package == directory) {
            return Err(PackagesError::Missing {
                package: dependency.package.0.to_string(),
                path: directory.to_string(),
            });
        }
        Ok(Located {
            directory: PackageRoot(directory),
            pin: crate::domain::packages::types::PackageSource::Path(SourcePath(
                dependency.package.0.clone(),
            )),
            base: SourcePath(dependency.package.0.clone()),
        })
    }

    fn read(&self, directory: &PackageRoot, file: &PackageFile) -> Result<FileText, PackagesError> {
        self.read
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(format!("{}/{}", directory.0, file.0).into());
        self.every()
            .get(&(directory.0.clone(), file.0.clone()))
            .map(|text| FileText(text.clone()))
            .ok_or_else(|| PackagesError::Read {
                path: format!("{}/{}", directory.0, file.0),
                reason: "no such file".to_owned(),
            })
    }

    fn offers(&self, directory: &PackageRoot, file: &PackageFile) -> Result<(), PackagesError> {
        self.read(directory, file).map(drop)
    }

    fn versions(
        &self,
        directory: &PackageRoot,
        within: &PackageFile,
    ) -> Result<crate::outbound::packages::types::Versions, PackagesError> {
        self.listed
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(format!("{}/{}", directory.0, within.0).into());
        let found: Vec<VersionText> = {
            let prefix = format!("{}/", within.0);
            let mut found = self
                .every()
                .into_keys()
                .filter(|(package, _)| *package == directory.0)
                .filter_map(|(_, path)| {
                    path.strip_prefix(&prefix)
                        .and_then(|rest| rest.split('/').next())
                        .map(|version| VersionText(version.into()))
                })
                .collect::<Vec<_>>();
            found.sort();
            found.dedup();
            found
        };
        Ok(crate::outbound::packages::types::Versions(found))
    }

    fn files(
        &self,
        directory: &PackageRoot,
        within: &PackageFile,
    ) -> Result<crate::outbound::packages::types::PackageFiles, PackagesError> {
        let found: Vec<PackageFile> = {
            // The package's own directory, named by no path, holds every file of it.
            let prefix = if within.0.is_empty() {
                String::new()
            } else {
                format!("{}/", within.0)
            };
            self.every()
                .into_keys()
                .filter(|(package, _)| *package == directory.0)
                .filter_map(|(_, path)| {
                    path.strip_prefix(&prefix)
                        .map(|rest| PackageFile(rest.into()))
                })
                .collect()
        };
        Ok(crate::outbound::packages::types::PackageFiles(found))
    }

    fn stamps(
        &self,
        files: &[crate::outbound::packages::types::PackagedFile],
    ) -> crate::domain::files::types::Stamps {
        let found: Vec<Stamp> = {
            files
                .iter()
                .map(|packaged| {
                    self.every()
                        .get(&(packaged.directory.0.clone(), packaged.file.0.clone()))
                        .map(|text| Stamp {
                            size: crate::domain::files::types::FileSize(
                                u64::try_from(text.len()).unwrap_or(u64::MAX),
                            ),
                            modified: crate::domain::files::types::ModifiedTime(0),
                        })
                        .unwrap_or_default()
                })
                .collect()
        };
        crate::domain::files::types::Stamps(found)
    }

    fn hold(&self, files: crate::outbound::packages::types::HeldFiles) -> PackageRoot {
        let mut held = self
            .held
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let made = held
            .keys()
            .map(|(package, _)| package)
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        let directory: Box<str> = format!("/held-{made}").into();
        held.extend(
            files
                .0
                .into_iter()
                .map(|(file, text)| ((directory.clone(), file.0), text.0)),
        );
        PackageRoot(directory)
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
