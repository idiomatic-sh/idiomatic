//! The fake of the `repository` port: what the language's tests stand in for it with.

use super::super::interface::RepositoryPort;
use crate::domain::paths::service::join;
use crate::domain::paths::types::Entry;
use crate::domain::paths::types::EntryKind;
use crate::domain::paths::types::EntryName;
use crate::domain::paths::types::FileText;
use crate::domain::paths::types::Height;
use crate::domain::paths::types::Look;
use crate::domain::paths::types::Placement;
use crate::domain::paths::types::Relative;
use crate::domain::paths::types::RepoPath;
use crate::domain::paths::types::Section;
use crate::domain::targets::types::WrittenPath;
use crate::outbound::repository::types::{Entries, Looks};

/// What the language's tests stand in for `repository` with: files held in memory, found by their path from the root; nothing is above
/// the root.
#[derive(Clone, Debug, Default)]
pub(crate) struct RepositoryFake {
    /// The path and the text of each file.
    pub files: Vec<(String, String)>,
}

/// `directory` and `name` as one path.
fn joined(directory: &str, name: &str) -> String {
    if directory.is_empty() {
        name.to_owned()
    } else {
        format!("{directory}/{name}")
    }
}

impl RepositoryPort for RepositoryFake {
    fn text(&self, path: &RepoPath) -> Section<FileText> {
        let path: &str = &path.0;
        self.files
            .iter()
            .find(|(each, _)| each == path)
            .map_or(Section::Absent, |(_, text)| {
                Section::Present(FileText(text.as_str().into()))
            })
    }

    fn entries(&self, directory: &RepoPath) -> Entries {
        let directory: &str = &directory.0;
        let prefix = if directory.is_empty() {
            String::new()
        } else {
            format!("{directory}/")
        };
        let mut found: Vec<Entry> = Vec::new();
        for (path, _) in &self.files {
            let Some(rest) = path.strip_prefix(&prefix) else {
                continue;
            };
            let (name, kind) = match rest.split_once('/') {
                Some((name, _)) => (name, EntryKind::Directory),
                None => (rest, EntryKind::File),
            };
            if !found.iter().any(|each| &*each.name == name) {
                found.push(Entry {
                    name: EntryName(name.into()),
                    kind,
                });
            }
        }
        Entries(found)
    }

    fn above(&self, directory: &RepoPath, name: &EntryName) -> Looks {
        let mut at = directory.0.to_string();
        let name: &str = &name.0;
        let mut looks = Vec::new();
        for height in 0_u32.. {
            let path = joined(&at, name);
            looks.push(Look {
                height: Height(height),
                text: self.text(&RepoPath(path.as_str().into())),
            });
            if at.is_empty() {
                break;
            }
            at = at
                .rsplit_once('/')
                .map_or(String::new(), |(above, _)| above.to_owned());
        }
        Looks(looks)
    }

    fn placed(&self, directory: &RepoPath, height: Height, written: &WrittenPath) -> Placement {
        let mut at: Vec<&str> = directory
            .0
            .split('/')
            .filter(|part| !part.is_empty())
            .collect();
        for _ in 0..height.0 {
            if at.pop().is_none() {
                return Placement::Outside;
            }
        }
        if written.0.starts_with('/') {
            return Placement::Outside;
        }
        join(&RepoPath(at.join("/").into()), &Relative(written.0.clone()))
            .map_or(Placement::Outside, Placement::Inside)
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
