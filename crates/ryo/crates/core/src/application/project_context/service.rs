//! One application authority for project context identity and warm observation reuse.
use super::constants::{DISK_REPOSITORY, GIVEN_REPOSITORY, IDENTITY_TAG};
use crate::domain::storing::service::as_kept;
use crate::outbound::corpus::interface::CorpusPort;
use crate::outbound::corpus::types::PathStat;
use ryo_execution::outbound::digest::interface::DigestPort;
use ryo_execution::outbound::digest::types::DigestBytes;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::project_context::types::EntryObservation;
use ryo_language::inbound::use_cases::project::types::ManifestFile;
use ryo_values::domain::files::types::FileBytes;
use ryo_values::domain::records::types::Freshness;
use ryo_values::domain::runs::types::Session;
use ryo_values::domain::stored_facts::types::Digest;
use ryo_values::domain::stores::types::StoredFile;

/// Digest the host's ordered search authority, Given inventory and project manifests.
pub(crate) fn identity(
    manifests: &[ManifestFile],
    session: &Session,
    digest: &dyn DigestPort,
) -> std::io::Result<Digest> {
    let absolute = std::path::absolute(std::path::Path::new(session.root.0.as_ref()))?;
    let mut normalized = std::path::PathBuf::new();
    for component in absolute.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                let _removed = normalized.pop();
            }
            other @ (std::path::Component::Prefix(_)
            | std::path::Component::RootDir
            | std::path::Component::Normal(_)) => normalized.push(other.as_os_str()),
        }
    }
    let repository = match session.repository {
        ryo_language::inbound::use_cases::project::types::Repository::Disk => DISK_REPOSITORY,
        ryo_language::inbound::use_cases::project::types::Repository::Given => GIVEN_REPOSITORY,
    };
    let mut parts = vec![
        digest.digest(DigestBytes(IDENTITY_TAG)),
        digest.digest(DigestBytes(normalized.as_os_str().as_encoded_bytes())),
        digest.digest(DigestBytes(repository)),
    ];
    let mut search_identity = Vec::new();
    match &session.search_scope {
        ryo_language::domain::project_context::types::ProjectSearchScope::Undeclared => {
            search_identity.push(0)
        }
        ryo_language::domain::project_context::types::ProjectSearchScope::Declared(roots) => {
            search_identity.push(1);
            search_identity
                .extend_from_slice(&u64::try_from(roots.len()).unwrap_or(u64::MAX).to_le_bytes());
            for root in roots {
                search_identity.extend_from_slice(
                    &u64::try_from(root.0.len())
                        .unwrap_or(u64::MAX)
                        .to_le_bytes(),
                );
                search_identity.extend_from_slice(root.0.as_bytes());
            }
        }
    }
    search_identity.push(match session.given_inventory.completeness {
            ryo_language::domain::project_context::types::ProjectInventoryCompleteness::Partial => 0,
            ryo_language::domain::project_context::types::ProjectInventoryCompleteness::CompleteWithinDeclaredScope => 1,
        });
    search_identity.extend_from_slice(
        &u64::try_from(session.given_inventory.entries.len())
            .unwrap_or(u64::MAX)
            .to_le_bytes(),
    );
    for entry in &session.given_inventory.entries {
        search_identity.extend_from_slice(
            &u64::try_from(entry.path.0.len())
                .unwrap_or(u64::MAX)
                .to_le_bytes(),
        );
        search_identity.extend_from_slice(entry.path.0.as_bytes());
        search_identity.push(match entry.state {
            ryo_language::domain::project_context::types::ObservedEntryState::File => 0,
            ryo_language::domain::project_context::types::ObservedEntryState::Directory => 1,
            ryo_language::domain::project_context::types::ObservedEntryState::Other => 2,
            ryo_language::domain::project_context::types::ObservedEntryState::Missing => 3,
            ryo_language::domain::project_context::types::ObservedEntryState::Unknown => 4,
        });
    }
    parts.push(digest.digest(DigestBytes(&search_identity)));
    parts.extend(manifests.iter().flat_map(|manifest| {
        [
            digest.digest(DigestBytes(manifest.path.0.as_bytes())),
            digest.digest(DigestBytes(manifest.text.0.as_bytes())),
        ]
    }));
    Ok(digest.combine(&parts))
}

/// Reobserve every entry consulted by project facts, then the older read-file dependencies.
pub(crate) fn as_it_was(
    corpus: &dyn CorpusPort,
    digest: &dyn DigestPort,
    kept: &[StoredFile],
    absent: &[FilePath],
    entries: &[EntryObservation],
) -> Freshness {
    if entries.iter().any(|entry| {
        entry.state == ryo_language::domain::project_context::types::ObservedEntryState::Unknown
            || corpus.observe_entry(&entry.path) != *entry
    }) {
        return Freshness::Changed;
    }
    // A file the lookup found none of stays so; one there now is one the lookup would have found.
    if absent
        .iter()
        .any(|path| !matches!(corpus.stat(path), Ok(PathStat::Missing)))
    {
        return Freshness::Changed;
    }
    let mut bytes = FileBytes::default();
    for each in kept {
        let Ok(PathStat::Found(entry)) = corpus.stat(&each.path) else {
            return Freshness::Changed;
        };
        if as_kept(each, &entry) == Freshness::AsItWas {
            continue;
        }
        if each.executable != entry.executable
            || corpus.read(&entry, &mut bytes).is_err()
            || digest.digest(DigestBytes(&bytes.0)) != each.digest
        {
            return Freshness::Changed;
        }
    }
    Freshness::AsItWas
}
