//! The `contract_rendering` service: free functions named by verb over what a placed file's template content is rendered
//! with and what its rendering makes of the file.

use std::hash::BuildHasher as _;

use rayon::prelude::*;

use super::types::{ContractRequest, Rendered, Requested};
use crate::domain::contexts::service::context;
use crate::domain::contexts::types::ContextRequest;
use crate::domain::contracts::constants::RENDERED;
use crate::domain::contracts::types::{Contract, RenderedContent, TemplateIdentity};
use crate::domain::definitions::service::content;
use crate::domain::definitions::types::{Exactness, LoadedWorkspace};
use crate::domain::entries::service::{identity, template_release};
use crate::domain::files::types::FileText;
use crate::domain::findings::constants::{TEMPLATE, UNKNOWN};
use crate::domain::findings::service::{check_id, error};
use crate::domain::findings::types::Finding;
use crate::domain::imports::constants::WORKSPACE_FILE;
use crate::domain::packages::types::DefinitionKind;
use crate::domain::paths::types::RepoPath;
use crate::domain::placement::types::Placement;
use crate::domain::scopes::service::composition_of;

/// The `#template` error of the template `identity` that does not render at `file`.
fn unrendered(
    identity: &TemplateIdentity,
    file: &RepoPath,
    reason: &dyn std::fmt::Display,
) -> Finding {
    error(
        check_id(&identity.0, TEMPLATE),
        file.clone(),
        format!(
            "the `{}` template does not render here: {reason}",
            identity.0
        ),
    )
}

/// The `#unknown` of a placed file whose template the placement cannot name: what it asks is not compared, and is said.
fn unplaced(file: &RepoPath) -> Finding {
    error(
        check_id(WORKSPACE_FILE, UNKNOWN),
        file.clone(),
        format!(
            "`{}` is placed by a template entry its placement no longer names: its template is not compared",
            file.0
        ),
    )
}

/// What each placed file asks of the template engine, in parallel: its content with the context its placement gives it,
/// or the `#template` error of a param with no value. Every placed file is asked, whatever language reads it: the lint
/// call says which files a language reads, and one none reads is `#unknown`, never a pass.
#[must_use]
pub fn requests(
    workspace: &LoadedWorkspace,
    placement: &Placement,
    written: &dyn crate::domain::entries::types::ValueWriter,
) -> Vec<Requested> {
    let placed = placement
        .placed
        .iter()
        .filter(|(file, _)| !placement.excluded.contains(*file))
        .collect::<Vec<_>>();
    placed
        .par_iter()
        .map(|(file, at)| {
            let Some(scope) = placement.scopes.get(at.scope.0) else {
                return Requested::Failed(unplaced(file));
            };
            let Some(entry) = composition_of(workspace, &scope.composition)
                .templates
                .get(at.entry.0)
            else {
                return Requested::Failed(unplaced(file));
            };
            let Some(template) = workspace.templates.get(&template_release(entry)) else {
                return Requested::Failed(unplaced(file));
            };
            let identity = TemplateIdentity(
                identity(&entry.header, DefinitionKind::Template, &entry.template)
                    .0
                    .into(),
            );
            let made = context(ContextRequest {
                workspace,
                scopes: &placement.scopes,
                matched: &placement.matched,
                index: at.scope,
                entry,
                template,
                file,
                captures: &at.captures,
                written,
            });
            match made {
                Ok(context) => {
                    // Named `<hash>.<name>`, the hash of the file's own path: the file's own name stays, so the language
                    // that reads the file reads the content too, and no rendered content is at a place a language reads
                    // as a project's: the rendered content is compared, never built. The name is the same in every run
                    // whatever else is placed, so the engine's cache finds it again.
                    let name = file.0.rsplit('/').next().unwrap_or_default();
                    let tag = foldhash::quality::FixedState::with_seed(0).hash_one(&*file.0);
                    Requested::Render(Box::new(ContractRequest {
                        target: (*file).clone(),
                        // Beneath the workspace's directory, the lint call's root, which every path it reads is under.
                        rendered: RepoPath(
                            if workspace.directory.0.is_empty() {
                                format!("{RENDERED}/{tag:016x}.{name}")
                            } else {
                                format!("{}/{RENDERED}/{tag:016x}.{name}", workspace.directory.0)
                            }
                            .into(),
                        ),
                        identity,
                        exact: template.exact,
                        content: content(template),
                        context,
                    }))
                }
                Err(reason) => Requested::Failed(unrendered(&identity, file, &reason.0)),
            }
        })
        .collect()
}

/// What rendering `request`'s content made of its file: the contract it is compared against, nothing when the content
/// is blank and not verbatim (any file meets it; a verbatim blank content is met by a blank file alone), or the `#template` error of a content that did not render.
#[must_use]
pub fn rendered(
    request: ContractRequest,
    text: Result<FileText, &dyn std::fmt::Display>,
) -> Rendered {
    match text {
        Ok(text) if text.0.trim().is_empty() && request.exact != Exactness::Verbatim => {
            Rendered::Nothing
        }
        Ok(text) => Rendered::Contract(Contract {
            target: request.target,
            rendered: request.rendered,
            text: RenderedContent(text.0),
            identity: request.identity,
            exact: request.exact,
        }),
        Err(reason) => Rendered::Failed(unrendered(&request.identity, &request.target, &reason)),
    }
}
