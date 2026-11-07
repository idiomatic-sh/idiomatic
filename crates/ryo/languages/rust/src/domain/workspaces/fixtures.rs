//! The test support of the `workspaces` concept: a repository held in memory that answers what a pass of the domain wants, and a
//! pass run again until it wants nothing.

use crate::domain::paths::service::present;
use crate::domain::paths::types::{Matching, Settlement};
use crate::domain::workspaces::service::settled;
use crate::domain::workspaces::types::{Looking, Reason};
use crate::outbound::conditions::factory::cargo_platform;
use crate::outbound::conditions::interface::ConditionsPort as _;
use crate::outbound::manifest::factory::toml;
use crate::outbound::manifest::interface::ManifestPort as _;
use crate::outbound::pattern_matching::factory::globset;
use crate::outbound::pattern_matching::interface::PatternMatchingPort as _;
use crate::outbound::repository::fixtures::fake::RepositoryFake;
use crate::outbound::repository::interface::RepositoryPort as _;

/// The files of a repository: path and text.
pub(crate) type Files = &'static [(&'static str, &'static str)];

/// The result of `pass` over `files`, run until it wants nothing the repository can tell.
pub(crate) fn answering<T>(files: Files, mut pass: impl FnMut(&mut Looking) -> T) -> T {
    let fake = RepositoryFake {
        files: files
            .iter()
            .map(|(path, text)| ((*path).to_owned(), (*text).to_owned()))
            .collect(),
    };
    let mut looking = Looking::default();
    loop {
        let result = pass(&mut looking);
        if settled(&looking) == Settlement::Settled {
            return result;
        }
        looking.manifests.seen.extend(
            std::mem::take(&mut looking.manifests.wanted)
                .into_iter()
                .map(|path| {
                    let told = present(fake.text(&path))
                        .ok_or_else(|| Reason(path.0.clone()))
                        .and_then(|text| {
                            toml()
                                .read(&text)
                                .map_err(|error| Reason(error.to_string().into()))
                        });
                    (path, told)
                }),
        );
        looking.directories.seen.extend(
            std::mem::take(&mut looking.directories.wanted)
                .into_iter()
                .map(|directory| {
                    let entries = fake.entries(&directory).0;
                    (directory, entries)
                }),
        );
        looking
            .matches
            .seen
            .extend(
                std::mem::take(&mut looking.matches.wanted)
                    .into_iter()
                    .map(|comparison| {
                        let matching = globset()
                            .matches(&comparison.pattern, &comparison.name)
                            .unwrap_or(Matching::Differs);
                        (comparison, matching)
                    }),
            );
        looking.configurations.seen.extend(
            std::mem::take(&mut looking.configurations.wanted)
                .into_iter()
                .map(|directory| (directory, Vec::new())),
        );
        looking.platforms.seen.extend(
            std::mem::take(&mut looking.platforms.wanted)
                .into_iter()
                .map(|platform| {
                    let shown = cargo_platform().show(&platform);
                    (platform, shown)
                }),
        );
    }
}
